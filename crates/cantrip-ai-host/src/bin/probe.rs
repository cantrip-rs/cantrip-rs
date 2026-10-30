//! `cantrip-probe`: re-searches recorded positions under other search settings, for diagnosing the AI's choices.
//!
//! - `session --session DIR --decision NNNN --model DIR (--server HOST:PORT | --backend tract) [--sims N] [--prune on|off]
//!   [--worlds N] [--search-seed N] [--value-model DIR]`: the advisor's decision NNNN, the game remade from the session's `commands.jsonl`
//!   through an AI host's mirror (each earlier decision of the seat asked of it as the advisor asked it, so its knowledge
//!   is kept the same way), then searched with `AiStart` (Decklist, `Report`). Prints a JSON line: the choice, and per
//!   root option (summed over worlds, trimmed worlds included) its visits, mean value, and the worlds that trimmed it.
//! - `sample --traces DIR --positions FILE --model DIR (--server ...) --out FILE [--sims N[,N...]] [--lanes N] [--limit N]
//!   [--worlds N]`: for each position (a JSON line: `id`, `seed`, `at`) of the traces' games, the search with trimming
//!   on and off at each budget, from the same worlds; a JSON line a position with each run's choice and root options.

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::record::table;
use cantrip_ai::search::{self, Searched, WallClock};
use cantrip_ai::trace::Trace;
use cantrip_ai::{Evaluator, Knowledge, Prior, kind_name};
use cantrip_ai_host::{AiHost, Loaded, STACK_SIZE, Sink};
use cantrip_obs::InfoMode;
use serde_json::{Value, json};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let worker = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || go(&args))
        .expect("starts");
    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("cantrip-probe: {e}");
            std::process::exit(2);
        }
        Err(_) => std::process::exit(1),
    }
}

#[derive(Default)]
struct Args {
    values: BTreeMap<String, String>,
}

impl Args {
    fn parse(args: &[String]) -> Result<Args, String> {
        let mut out = Args::default();
        let mut i = 0;
        while i < args.len() {
            let v = args
                .get(i + 1)
                .cloned()
                .ok_or(format!("{} needs a value", args[i]))?;
            out.values
                .insert(args[i].trim_start_matches("--").to_string(), v);
            i += 2;
        }
        Ok(out)
    }

    fn get(&self, k: &str) -> Option<&str> {
        self.values.get(k).map(String::as_str)
    }

    fn need(&self, k: &str) -> Result<&str, String> {
        self.get(k).ok_or(format!("--{k} is needed"))
    }

    fn num(&self, k: &str, d: u64) -> Result<u64, String> {
        self.get(k).map_or(Ok(d), |v| {
            v.parse().map_err(|_| format!("--{k} takes a number"))
        })
    }
}

fn evaluator(a: &Args, dir: &Path) -> Result<Arc<dyn Evaluator>, String> {
    Ok(match a.get("server") {
        Some(s) => Arc::new(
            cantrip_ai::RemoteEvaluator::connect(s, dir, table()).map_err(|e| e.to_string())?,
        ),
        None => {
            Arc::new(cantrip_ai::TractEvaluator::load(dir, table()).map_err(|e| e.to_string())?)
        }
    })
}

fn go(argv: &[String]) -> Result<(), String> {
    let command = argv.first().ok_or("session or sample")?.clone();
    let a = Args::parse(&argv[1..])?;
    match command.as_str() {
        "session" => session(&a),
        "sample" => sample(&a),
        "play" => play(&a),
        "mana-report" => mana_report(&a),
        other => Err(format!("no command {other}")),
    }
}

// --- One advisor decision ------------------------------------------------------------------------------------------------

fn session(a: &Args) -> Result<(), String> {
    let dir = PathBuf::from(a.need("session")?);
    let model = PathBuf::from(a.need("model")?);
    let want = a.need("decision")?;
    let file = std::fs::read_dir(dir.join("decisions"))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(want))
        })
        .ok_or(format!("no decision {want}"))?;
    let record: Value =
        serde_json::from_str(&std::fs::read_to_string(&file).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let upto = record["Observation"]["CommandIndex"]
        .as_u64()
        .ok_or("no CommandIndex")? as usize;
    let request_id = record["RequestId"].as_u64().ok_or("no RequestId")?;
    let seat = 1;
    // The earlier decisions the advisor asked of its AI host: asked again before their answers.
    let mut asked: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for e in std::fs::read_dir(dir.join("decisions"))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
    {
        let n = e.file_name().to_string_lossy().into_owned();
        if let Some(r) = n
            .split("-r")
            .nth(1)
            .and_then(|r| r.trim_end_matches(".json").parse::<u64>().ok())
        {
            asked.insert(r);
        }
    }

    // Each search on a host of its own, the game remade afresh: a search notes its answer in the seat's cancel guard as if
    // it were given, so a second search of the same request on the same host would not be the first's twin.
    let replay = || -> Result<(AiHost, Receiver<Value>, u64), String> {
        let (send, receive) = channel::<Value>();
        let send = Mutex::new(send);
        let sink: Sink = Arc::new(move |v: &Value| {
            let _ = send.lock().unwrap().send(v.clone());
        });
        let name = model
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut loaded = vec![Loaded {
            name: name.clone(),
            evaluator: evaluator(a, &model)?,
        }];
        // A perfect-information value model, valuing the leaves on each world's full state (`--value-model DIR`).
        if let Some(v) = a.get("value-model") {
            let dir = PathBuf::from(v);
            loaded.push(Loaded {
                name: dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                evaluator: evaluator(a, &dir)?,
            });
        }
        let mut host = AiHost::new(
            loaded,
            "probe",
            1,
            sink,
        );
        let commands =
            std::fs::read_to_string(dir.join("commands.jsonl")).map_err(|e| e.to_string())?;
        let mut game_id = 0;
        for line in commands.lines().take(upto) {
            let c: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
            if c["Cmd"] == "NewGame" {
                game_id = c["GameId"].as_u64().unwrap_or(1);
            }
            if c["Cmd"] == "Answer"
                && c["Seat"] == json!(seat)
                && c["RequestId"].as_u64().is_some_and(|r| asked.contains(&r))
            {
                // Its knowledge kept up as the advisor's was: the seat asked at each of its decisions (the network alone).
                let r = host.line(&json!({ "Cmd": "AiStart", "GameId": game_id, "Seat": seat, "Mode": "Decklist" }).to_string());
                if r.get("Ok").is_some() {
                    result(&receive)?;
                }
            }
            let reply = host.line(line);
            if reply.get("Ok").is_none() {
                return Err(format!("replaying {line}: {reply}"));
            }
        }

        Ok((host, receive, game_id))
    };
    let sims: Vec<u64> = a
        .get("sims")
        .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_else(|| {
            vec![
                record["Bot"]["Search"]["Simulations"]
                    .as_u64()
                    .unwrap_or(2048),
            ]
        });
    let arms: Vec<String> = a
        .get("arms")
        .unwrap_or("cut,reversible,off")
        .split(',')
        .map(str::to_string)
        .collect();
    for &n in &sims {
        for arm in &arms {
            let (prune, reversible) = arm_of(arm)?;
            let (mut host, receive, game_id) = replay()?;
            let mut start = json!({
                "Cmd": "AiStart", "GameId": game_id, "Seat": seat, "Mode": "Decklist", "Simulations": n,
                "Worlds": a.num("worlds", 8)?, "Report": true, "Prune": prune, "PruneReversible": reversible,
            });
            if let Some(s) = a.get("search-seed") {
                start["SearchSeed"] = json!(s.parse::<u64>().map_err(|_| "--search-seed")?);
            }
            if let Some(v) = a.get("value-model") {
                start["ValueModel"] = json!(PathBuf::from(v).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
            }
            let started = Instant::now();
            let r = host.line(&start.to_string());
            let got = r["Ok"]["Started"]["RequestId"].as_u64();
            if got != Some(request_id) {
                return Err(format!(
                    "the replay is at request {got:?}, the decision was request {request_id}: {r}"
                ));
            }
            let res = result(&receive)?;
            let an = &res["AiResult"]["Analysis"];
            println!(
                "{}",
                json!({
                    "decision": want, "sims": n, "arm": arm, "ms": started.elapsed().as_millis() as u64,
                    "value_model": a.get("value-model"),
                    "choice": an["Chosen"]["Label"], "root": an["Search"]["RootValue"],
                    "options": options_of(an),
                    "world0_hand": an["Worlds"][0]["Hands"], "recorded_world0_hand": record["Worlds"][0]["Hands"],
                })
            );
            host.finish();
        }
    }
    Ok(())
}

/// A trimming arm by name: `cut` (cuts for good), `reversible` (the fix), `off`.
fn arm_of(arm: &str) -> Result<(bool, bool), String> {
    match arm {
        "cut" => Ok((true, false)),
        "reversible" => Ok((true, true)),
        "off" => Ok((false, false)),
        other => Err(format!("no arm {other}: cut, reversible or off")),
    }
}

fn result(receive: &Receiver<Value>) -> Result<Value, String> {
    receive
        .recv_timeout(Duration::from_secs(600))
        .map_err(|_| "no result".to_string())
}

/// Each root option over the worlds, trimmed worlds included: its visits, mean, the trimmed worlds, and per world.
fn options_of(an: &Value) -> Value {
    let labels: BTreeMap<String, String> = an["Candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            Some((
                c["Token"].as_str()?.to_string(),
                c["Label"].as_str()?.to_string(),
            ))
        })
        .collect();
    let mut by: BTreeMap<String, (u64, f64, Vec<Value>, usize)> = BTreeMap::new();
    for (w, world) in an["Worlds"].as_array().into_iter().flatten().enumerate() {
        for o in world["Options"].as_array().into_iter().flatten() {
            let t = o["Token"].as_str().unwrap_or_default().to_string();
            let n = o["Visits"].as_u64().unwrap_or(0);
            let v = o["Value"].as_f64();
            let e = by.entry(t).or_default();
            e.0 += n;
            e.1 += v.unwrap_or(0.0) * n as f64;
            if o["Pruned"] == json!(true) {
                e.3 += 1;
            }
            e.2.push(json!([
                w,
                n,
                v.map(|x| (x * 1000.0).round() / 1000.0),
                o["Pruned"]
            ]));
        }
    }
    let mut out: Vec<Value> = by
        .into_iter()
        .map(|(t, (n, s, per, cut))| {
            json!({ "label": labels.get(&t).cloned().unwrap_or(t), "visits": n, "mean": if n > 0 { s / n as f64 } else { 0.0 }, "trimmed_worlds": cut, "per_world": per })
        })
        .collect();
    out.sort_by(|a, b| b["visits"].as_u64().cmp(&a["visits"].as_u64()));
    json!(out)
}

// --- A sample of self-play positions -------------------------------------------------------------------------------------

#[derive(serde::Deserialize, Clone)]
struct Position {
    id: u64,
    seed: u64,
    at: usize,
}

/// One root option of one search, summed over its worlds.
fn root_options(s: &Searched) -> (Option<String>, BTreeMap<String, Vec<(u32, f32, bool)>>) {
    let mut by: BTreeMap<String, Vec<(u32, f32, bool)>> = BTreeMap::new();
    let Some(r) = &s.report else {
        return (None, by);
    };
    let worlds = r.worlds.len();
    for (w, world) in r.worlds.iter().enumerate() {
        for e in &world.edges {
            let v = by
                .entry(format!("{:?}", e.token))
                .or_insert_with(|| vec![(0, 0.0, false); worlds]);
            v[w] = (e.visits, e.value_sum, e.pruned);
        }
    }
    let chosen = s
        .visits
        .iter()
        .find(|st| st.prefix == r.prefix)
        .map(|st| format!("{:?}", st.chosen));
    (chosen, by)
}

fn summary(s: &Searched, ms: u128) -> Value {
    let (chosen, by) = root_options(s);
    let options: Vec<Value> = by
        .iter()
        .map(|(t, per)| {
            json!({
                "token": t,
                "per_world": per.iter().map(|&(n, w, p)| json!([n, if n > 0 { Some(w / n as f32) } else { None }, p])).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "chosen": chosen, "root": s.root_value, "sims": s.simulations, "ms": ms, "shortcut": s.shortcut, "options": options })
}

fn sample(a: &Args) -> Result<(), String> {
    let traces = cantrip_ai_host::regret::load_traces(Path::new(a.need("traces")?))?;
    let text = std::fs::read_to_string(a.need("positions")?).map_err(|e| e.to_string())?;
    let mut positions: Vec<Position> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    positions.truncate(a.num("limit", u64::MAX)? as usize);
    let model = PathBuf::from(a.need("model")?);
    let sims: Vec<usize> = a
        .get("sims")
        .unwrap_or("64")
        .split(',')
        .filter_map(|x| x.parse().ok())
        .collect();
    let worlds = a.num("worlds", 8)? as usize;
    let lanes = a.num("lanes", 16)? as usize;
    let arms: Vec<String> = a
        .get("arms")
        .unwrap_or("cut,reversible,off")
        .split(',')
        .map(str::to_string)
        .collect();
    let out = Arc::new(Mutex::new(
        std::fs::File::create(a.need("out")?).map_err(|e| e.to_string())?,
    ));
    let next = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let traces = Arc::new(traces);
    let positions = Arc::new(positions);
    let prior = Arc::new(Prior::new());
    let mut handles = Vec::new();
    for _ in 0..lanes {
        let (out, next, traces, positions, prior, sims, arms) = (
            out.clone(),
            next.clone(),
            traces.clone(),
            positions.clone(),
            prior.clone(),
            sims.clone(),
            arms.clone(),
        );
        let ev = evaluator(a, &model)?;
        handles.push(
            std::thread::Builder::new()
                .stack_size(STACK_SIZE)
                .spawn(move || {
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(p) = positions.get(i) else { break };
                        let line = match probe_position(
                            traces.get(&p.seed),
                            p,
                            ev.as_ref(),
                            &prior,
                            &sims,
                            &arms,
                            worlds,
                        ) {
                            Ok(v) => v,
                            Err(e) => json!({ "id": p.id, "error": e }),
                        };
                        let mut o = out.lock().unwrap();
                        let _ = writeln!(o, "{line}");
                    }
                })
                .expect("a lane starts"),
        );
    }
    for h in handles {
        let _ = h.join();
    }
    Ok(())
}

fn probe_position(
    trace: Option<&Trace>,
    p: &Position,
    ev: &dyn Evaluator,
    prior: &Prior,
    sims: &[usize],
    arms: &[String],
    worlds: usize,
) -> Result<Value, String> {
    let trace = trace.ok_or("no trace")?;
    let (game, _) = trace.replay(p.at)?;
    let request = game.pending().cloned().ok_or("asks nothing")?;
    let knowledge = Knowledge::of(&game, request.player);
    let mut runs = Vec::new();
    for &n in sims {
        for arm in arms {
            let (prune, prune_reversible) = arm_of(arm)?;
            let config = search::Config {
                simulations: Some(n),
                worlds,
                prune,
                prune_reversible,
                report: true,
                seed: p.id ^ 0x5ea2c4,
                ..search::Config::default()
            };
            let t0 = Instant::now();
            let d = search::search(
                &game,
                &request,
                &knowledge,
                InfoMode::Decklist,
                table(),
                ev,
                &config,
                prior,
                &CancelGuard::default(),
                &WallClock,
            )
            .map_err(|e| e.0)?;
            let ms = t0.elapsed().as_millis();
            match &d.search {
                Some(s) => runs.push(json!({ "sims": n, "arm": arm, "result": summary(s, ms) })),
                None => runs.push(json!({ "sims": n, "arm": arm, "result": null })),
            }
        }
    }
    Ok(json!({
        "id": p.id, "seed": p.seed, "at": p.at, "turn": game.state.turn.number, "kind": kind_name(&request.kind),
        "runs": runs,
    }))
}

// --- Self-play-like games, to measure tree reuse -------------------------------------------------------------------------

/// A decklist file's main deck, one name per copy.
fn deck_of(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.eq_ignore_ascii_case("sideboard") || line.eq_ignore_ascii_case("companion") {
            break;
        }
        match line.split_once(' ') {
            Some((n, name)) if n.parse::<usize>().is_ok() => {
                out.extend(std::iter::repeat_n(name.to_string(), n.parse().unwrap_or(0)))
            }
            _ => out.push(line.to_string()),
        }
    }
    Ok(out)
}

/// `play --decks DIR --model DIR --server HOST:PORT --games N [--sims N] [--worlds N] [--reuse add|topup|none|keep] [--follow all|passes|own] [--check on] [--pay-classic on] [--pay-split F]
/// [--lanes N] [--seed N] --out FILE`: games of each deck as a mirror, both seats searching in Decklist mode as
/// self-play does; a JSON line a game with its searches' counts, and for re-rooted searches what they carried and
/// whether the choice was the kept tree's principal line.
fn play(a: &Args) -> Result<(), String> {
    let dir = PathBuf::from(a.need("decks")?);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    let decks: Vec<(String, Vec<String>)> = files
        .iter()
        .map(|f| Ok((f.file_stem().unwrap_or_default().to_string_lossy().into_owned(), deck_of(f)?)))
        .collect::<Result<_, String>>()?;
    let model = PathBuf::from(a.need("model")?);
    let games = a.num("games", 24)? as usize;
    let sims = a.num("sims", 64)? as usize;
    let worlds = a.num("worlds", 8)? as usize;
    let lanes = a.num("lanes", 8)? as usize;
    let seed = a.num("seed", 9_200_000)?;
    let reuse = match a.get("reuse").unwrap_or("add") {
        "none" => search::Reuse::None,
        "topup" => search::Reuse::TopUp,
        "add" => search::Reuse::Add,
        "keep" => search::Reuse::KeepOnly,
        other => return Err(format!("no reuse {other}")),
    };
    let follow = match a.get("follow").unwrap_or("all") {
        "own" => search::Follow::Own,
        "passes" => search::Follow::Passes,
        "all" => search::Follow::All,
        other => return Err(format!("no follow {other}: own, passes or all")),
    };
    let atomic = a.get("atomic") != Some("off");
    let flatten = a.num("flatten", 64)? as usize;
    let check = a.get("check") == Some("on");
    let pay_classic = a.get("pay-classic") == Some("on");
    let pay_split: f32 = a.get("pay-split").map_or(Ok(search::Config::default().pay_split), |v| v.parse().map_err(|e| format!("--pay-split: {e}")))?;
    let out = Arc::new(Mutex::new(std::fs::File::create(a.need("out")?).map_err(|e| e.to_string())?));
    let next = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let decks = Arc::new(decks);
    let mut handles = Vec::new();
    for _ in 0..lanes {
        let (out, next, decks) = (out.clone(), next.clone(), decks.clone());
        let ev = evaluator(a, &model)?;
        handles.push(
            std::thread::Builder::new()
                .stack_size(STACK_SIZE)
                .spawn(move || {
                    loop {
                        let g = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if g >= games {
                            break;
                        }
                        let (name, list) = &decks[g % decks.len()];
                        let line = match play_game(list, seed + g as u64, ev.as_ref(), sims, worlds, reuse, follow, atomic, flatten, check, pay_classic, pay_split) {
                            Ok(mut v) => {
                                v["game"] = json!(g);
                                v["deck"] = json!(name);
                                v
                            }
                            Err(e) => json!({ "game": g, "error": e }),
                        };
                        let mut o = out.lock().unwrap();
                        let _ = writeln!(o, "{line}");
                    }
                })
                .expect("a lane starts"),
        );
    }
    for h in handles {
        let _ = h.join();
    }
    Ok(())
}

fn play_game(
    list: &[String],
    seed: u64,
    ev: &dyn Evaluator,
    sims: usize,
    worlds: usize,
    reuse: search::Reuse,
    follow: search::Follow,
    atomic: bool,
    flatten: usize,
    check: bool,
    pay_classic: bool,
    pay_split: f32,
) -> Result<Value, String> {
    use cantrip_engine::{Event, Outcome, Rejected, RequestKind, deal};
    let mut game = deal(&[list.to_vec(), list.to_vec()], seed, true, None)?;
    let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
    let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
    let mut kept: Vec<Option<search::Kept>> = vec![None, None];
    let prior = Prior::new();
    let started = Instant::now();
    let (mut answers, mut searched, mut sims_run, mut evaluations) = (0u64, 0u64, 0u64, 0u64);
    let (mut reused, mut carried, mut principal, mut principal_match, mut reused_worlds) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut search_us = 0u128;
    let (mut rows, mut flattened, mut fallbacks, mut waste, mut refused) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut unpayable = 0u64;
    // Each searched decision's tree shape: the main line's turns and decisions, the deepest line's turns and decisions.
    let mut shapes: Vec<[usize; 4]> = Vec::new();
    let mut refilled = 0u64;
    // The reuse check's totals (`search::ReuseCheck`): worlds checked, mismatches by kind, the largest differences.
    let (mut checked, mut mismatches, mut prior_diff, mut value_diff) = (0u64, BTreeMap::<String, u64>::new(), 0f32, 0f32);
    let (mut fresh_checked, mut fresh_mismatched) = (0u64, 0u64);
    // Casts split by class of payment (`search::Config::pay_split`), and the searches that split one.
    let (mut pay_splits, mut split_searches) = (0u64, 0u64);
    let mut root_edges: Vec<usize> = Vec::new();
    let (mut offered, mut opp_choices, mut opp_actions, mut lost) = (0u64, 0u64, 0u64, BTreeMap::<String, u64>::new());
    let mut survival: Vec<(usize, usize, usize)> = Vec::new();
    let mut windows = search::Windows::default();
    // The root's windows: each seat's last searched priority with an empty stack (its turn and history length), and
    // the searches and evaluations at windows of the same class as the seat's previous one.
    let mut last_window: Vec<Option<(u32, usize)>> = vec![None, None];
    let (mut root_windows, mut root_same, mut root_window_evals, mut root_same_evals) = (0u64, 0u64, 0u64, 0u64);
    while !game.is_over() && game.state.turn.number <= 40 && answers < 6000 {
        let Some(request) = game.pending().cloned() else { break };
        answers += 1;
        let p = request.player.index();
        ks[p].update(&game);
        let config = search::Config {
            simulations: Some(sims),
            worlds,
            seed: search::splitmix_pub(seed ^ ((request.id as u64) << 24) ^ 0x5ea2c4),
            reuse,
            reuse_follow: follow,
            reuse_check: check,
            pay_classic,
            pay_split,
            atomic,
            flatten,
            ..search::Config::default()
        };
        payments[p].atomic = atomic;
        payments[p].classic = pay_classic;
        let k = &ks[p];
        let keep = &mut kept[p];
        let t0 = Instant::now();
        let decided = payments[p].decide(&game, &request, k, InfoMode::Decklist, |request, guard| {
            search::search_keeping(&game, request, k, InfoMode::Decklist, table(), ev, &config, &prior, guard, &WallClock, keep)
        });
        let answer = match decided {
            Ok(d) => {
                if let Some(s) = &d.search
                    && !s.visits.is_empty()
                {
                    searched += 1;
                    search_us += t0.elapsed().as_micros();
                    if matches!(request.kind, RequestKind::Priority { .. }) && game.state.stack.is_empty() {
                        root_windows += 1;
                        root_window_evals += s.evaluations as u64;
                        let now = (game.state.turn.number, game.log.len());
                        if let Some((turn, len)) = last_window[p]
                            && turn == now.0
                            && game.log[len.min(now.1)..now.1].iter().all(|r| matches!(r.event, Event::StepBegan { .. }))
                        {
                            root_same += 1;
                            root_same_evals += s.evaluations as u64;
                        }
                        last_window[p] = Some(now);
                    }
                    sims_run += s.simulations as u64;
                    evaluations += s.evaluations as u64;
                    rows += s.rows as u64;
                    flattened += s.flattened as u64;
                    fallbacks += s.flatten_fallbacks as u64;
                    waste += s.waste_leaves as u64;
                    refused += s.refused as u64;
                    unpayable += s.unpayable as u64;
                    if !s.shortcut {
                        root_edges.push(s.root_edges);
                        shapes.push([s.shape.line_turns as usize, s.shape.line_decisions, s.shape.deepest_turns as usize, s.shape.deepest_decisions]);
                    }
                    fresh_checked += s.fresh_checked as u64;
                    pay_splits += s.pay_splits as u64;
                    split_searches += u64::from(s.pay_splits > 0);
                    fresh_mismatched += s.fresh_mismatched as u64;
                    if let Some(r) = &s.reused {
                        reused += 1;
                        carried += u64::from(r.visits);
                        reused_worlds += r.worlds as u64;
                        offered += r.offered as u64;
                        opp_choices += r.opponent_choices as u64;
                        opp_actions += r.opponent_actions as u64;
                        for (why, n) in &r.lost {
                            *lost.entry(why.clone()).or_default() += *n as u64;
                        }
                        survival.push((r.offered, r.worlds, r.opponent_actions));
                        refilled += r.refilled as u64;
                        if let Some(c) = &r.check {
                            checked += c.worlds as u64;
                            for (what, n) in &c.mismatches {
                                *mismatches.entry(what.clone()).or_default() += *n as u64;
                            }
                            prior_diff = prior_diff.max(c.prior_diff);
                            value_diff = value_diff.max(c.value_diff);
                        }
                        if let Some(t) = r.principal {
                            principal += 1;
                            principal_match += u64::from(s.visits.first().is_some_and(|v| v.chosen == t));
                        }
                    }
                }
                d.answer
            }
            Err(_) => {
                let filtered = payments[p].filtered(&game, &request);
                cantrip_ai::net::answer(&game, &filtered, &ks[p], InfoMode::Decklist, table(), ev, cantrip_ai::net::Choice::Best)
                    .map_err(|e| e.0)?
                    .answer
            }
        };
        if reuse == search::Reuse::KeepOnly
            && let Some(k) = kept[p].as_mut()
        {
            let w = k.windows();
            windows.positions += w.positions;
            windows.evaluations += w.evaluations;
            windows.transposed_positions += w.transposed_positions;
            windows.transposed_evaluations += w.transposed_evaluations;
            windows.opponent_windows += w.opponent_windows;
            windows.opponent_window_evaluations += w.opponent_window_evaluations;
            windows.same_class_windows += w.same_class_windows;
            windows.same_class_evaluations += w.same_class_evaluations;
            kept[p] = None;
        }
        let board = cantrip_ai::cancel::board_hash(&game.state);
        payments[p].played(&game, board, &request, &answer);
        game.answer(request.id, request.player, answer.clone()).map_err(|Rejected(why)| why)?;
    }
    Ok(json!({
        "answers": answers, "searched": searched, "simulations": sims_run, "evaluations": evaluations,
        "reused": reused, "carried": carried, "reused_worlds": reused_worlds, "principal": principal, "principal_match": principal_match,
        "offered": offered, "opponent_choices": opp_choices, "opponent_actions": opp_actions, "lost": lost, "survival": survival,
        "windows": format!("{windows:?}"),
        "tree": [windows.positions, windows.evaluations, windows.transposed_positions, windows.transposed_evaluations,
                 windows.opponent_windows, windows.opponent_window_evaluations, windows.same_class_windows, windows.same_class_evaluations],
        "root_windows": [root_windows, root_window_evals, root_same, root_same_evals],
        "rows": rows, "flattened": flattened, "flatten_fallbacks": fallbacks, "waste_leaves": waste, "refused": refused, "unpayable": unpayable, "shapes": shapes, "refilled": refilled, "root_edges": root_edges,
        "check": { "worlds": checked, "mismatches": mismatches, "prior_diff": prior_diff, "value_diff": value_diff,
                   "fresh_worlds": fresh_checked, "fresh_mismatched": fresh_mismatched },
        "pay_splits": pay_splits, "split_searches": split_searches,
        "search_ms": (search_us / 1000) as u64, "ms": started.elapsed().as_millis() as u64,
        "turns": game.state.turn.number,
        "winner": match game.state.outcome { Outcome::Won(w) => json!(w.0 + 1), _ => json!(null) },
    }))
}

// --- Mana abilities that do more than make mana -------------------------------------------------------------------------

/// `mana-report [--decks DIR]`: every implemented card's mana abilities whose activation does more than add mana (words
/// besides "Add ..." and its restrictions), and those with a cost beyond tapping (life, sacrifice, counters, energy):
/// what the AI gives up by never activating a mana ability on its own (`cantrip_ai::atomic`). With `--decks`, the cards
/// of those decklists are marked.
fn mana_report(a: &Args) -> Result<(), String> {
    let mut played: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Some(dir) = a.get("decks") {
        for e in std::fs::read_dir(dir).map_err(|e| e.to_string())?.filter_map(Result::ok) {
            if e.path().extension().is_some_and(|x| x == "txt") {
                played.extend(deck_of(&e.path())?);
            }
        }
    }
    let quiet = |sentence: &str| {
        let s = sentence.trim();
        s.is_empty()
            || s.starts_with("Add ")
            || s.starts_with("{T}: Add")
            || s.starts_with("Spend this mana only")
            || s.starts_with("Activate only")
            || s.starts_with("This ability can't")
    };
    let mut effects = Vec::new();
    let mut costs = Vec::new();
    for card in &cantrip_engine::cards().cards {
        for ability in card.abilities {
            let cantrip_engine::AbilityDef::Activated(d) = ability else { continue };
            if !d.is_mana_ability() {
                continue;
            }
            let words = d.text.split_once(':').map_or(d.text, |(_, w)| w);
            let mark = if played.contains(card.name) { " [played]" } else { "" };
            if !words.split('.').all(quiet) {
                effects.push(format!("{}{mark}: {}", card.name, d.text));
            }
            let c = &d.cost;
            if c.life > 0 || c.sacrifice_self || c.remove_counters.is_some() || c.energy > 0 || c.permanents.is_some() || c.discard_self {
                costs.push(format!("{}{mark}: {}", card.name, d.text));
            }
        }
    }
    println!("Mana abilities whose activation does more than add mana ({}):", effects.len());
    for e in &effects {
        println!("  {e}");
    }
    println!("\nMana abilities with a cost beyond tapping ({}):", costs.len());
    for e in &costs {
        println!("  {e}");
    }
    Ok(())
}
