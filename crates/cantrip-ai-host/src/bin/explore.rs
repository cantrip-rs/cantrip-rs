//! `cantrip-explore`: searches with a prior floor at the root, to ask what the policy excludes
//! (`cantrip_ai_host::explore`; the review of 2026-11-13, item 9).
//!
//! - `audit --traces DIR --model DIR --out FILE`: positions the priority shortcut passed, sampled from traced games,
//!   searched with a floor on every option (`search::audit_shortcut`); how often an excluded option beats passing by a
//!   margin. `--positions N` (2000), `--floor F` (0.2), `--margin M` (0.05), `--min-visits N` (8), `--shortcut P` (the
//!   search's, 0.01: the bound positions are found with, and the search's below the root), `--simulations N` (4096),
//!   `--worlds N` (8),
//!   `--mode full|decklist|hidden` (decklist), `--starts DIR`, `--lanes N`, `--seed N`.
//! - `verify --scenario FILE --action TEXT --model DIR`: the scenario's position (as self-play's `--starts` takes it),
//!   searched with a floor on the nominated action (`search::Config::root_favour`; `TEXT` in its words, or `#k` the
//!   k-th option), and the paired
//!   difference of its value and the best other option's over the worlds, with a 95% interval. `--floor F` (0.2),
//!   `--worlds N` (16), `--simulations N` (4096), `--mode` (decklist), `--seed N`, `--out FILE`.
//!
//! - The pair miner (`cantrip_ai_host::mine`), over games recorded by the native host with traces (BotLab's `--ai-record`
//!   with `--ai-record-traces`), the two games of each pair in two directories:
//!   - `mine-forks --first DIR --second DIR --out FILE [--lanes N]`: each pair's first differing answer, with the proof
//!     that the games are one game up to there (no network: `--model` is not asked for).
//!   - `mine-values --dir DIR --games FILE --out FILE`: `FILE` a JSON line a game, `{"seed", "seat" (from 0), "model"}`:
//!     that seat's network value at each of its decisions, by that model (each model's network on the `--server`).
//!   - `mine-filter --from DIR --keep FILE --out DIR`: of the target shards `mine-judge` wrote for one split
//!     (`OUT/<split>`), the positions `FILE` names (a JSON line each, `{"dir", "seed", "decision"}`), into `DIR`.
//!   - `mine-judge --plan FILE --dir NAME=DIR ... --model DIR --out DIR [--simulations N] [--worlds N] [--floor F]
//!     [--batch N] [--seed N] [--lanes N] [--targets true|false] [--producer TEXT]`: the plan's positions (`mine::Planned`, a
//!     JSON line each) judged and searched; targets in `OUT/<split>/`, a line a position in `OUT/judged.jsonl`.
//!
//! All but `mine-forks` take `--backend ort|tract|remote`, `--server HOST:PORT`, `--threads N`, `--prior DIR`. The summary
//! or the verdict is printed as JSON.

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cantrip_ai::Evaluator;
use cantrip_ai::record::table;
use cantrip_ai::shard::parse_mode;
use cantrip_ai_host::explore::{self, AuditOptions, VerifyOptions};
use cantrip_ai_host::selfplay::{Start, Starts};

const USAGE: &str = "usage: cantrip-explore audit --traces DIR --model DIR --out FILE [--positions N] [--floor F] [--margin M] [--min-visits N] [--shortcut P]
           [--simulations N] [--worlds N] [--mode full|decklist|hidden] [--starts DIR] [--lanes N] [--seed N]
       cantrip-explore verify --scenario FILE --action TEXT|#k --model DIR [--floor F] [--worlds N] [--simulations N] [--mode M] [--seed N] [--out FILE]
       cantrip-explore mine-forks --first DIR --second DIR --out FILE [--lanes N]
       cantrip-explore mine-values --dir DIR --games FILE --out FILE [--lanes N]
       cantrip-explore mine-filter --from DIR --keep FILE --out DIR [--producer TEXT]
       cantrip-explore mine-judge --plan FILE --dir NAME=DIR ... --model DIR --out DIR [--simulations N] [--worlds N] [--floor F] [--batch N]
           [--seed N] [--lanes N] [--targets true|false] [--producer TEXT] [--mode M] [--search K=V,...]
       all but mine-forks: [--backend ort|tract|remote] [--server HOST:PORT] [--threads N] [--prior DIR]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let worker = std::thread::Builder::new().stack_size(cantrip_ai_host::STACK_SIZE).spawn(move || go(&args)).expect("the main thread starts");
    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("cantrip-explore: {e}\n{USAGE}");
            std::process::exit(2);
        }
        Err(_) => std::process::exit(1),
    }
}

fn go(args: &[String]) -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let command = args.first().ok_or("no command")?.clone();
    let mut flags: BTreeMap<String, String> = BTreeMap::new();
    let mut dirs: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let v = args.get(i + 1).cloned().ok_or(format!("{} needs a value", args[i]))?;
        if args[i] == "--dir" && command == "mine-judge" {
            let (name, dir) = v.split_once('=').ok_or(format!("--dir takes NAME=DIR, not {v}"))?;
            dirs.insert(name.to_string(), PathBuf::from(dir));
        } else {
            flags.insert(args[i].clone(), v);
        }
        i += 2;
    }
    if command.starts_with("mine-") {
        return mine(&command, &flags, dirs, &root);
    }
    let take = |k: &str| flags.get(k).cloned();
    let num = |k: &str| -> Result<Option<f64>, String> { take(k).map(|v| v.parse::<f64>().map_err(|_| format!("{k} takes a number, not {v}"))).transpose() };
    let model = PathBuf::from(take("--model").ok_or("no --model")?);
    let backend = if flags.contains_key("--server") { "remote".to_string() } else { take("--backend").unwrap_or_else(|| if cfg!(feature = "ort") { "ort".into() } else { "tract".into() }) };
    let server = take("--server");
    let threads = num("--threads")?.unwrap_or(1.0) as usize;
    let prior = cantrip_ai::Prior::from_decks(&take("--prior").map(PathBuf::from).unwrap_or(root.join("decks")))?;
    let mode = take("--mode").map(|m| parse_mode(&m).ok_or(format!("no mode {m}"))).transpose()?;
    let known: &[&str] = match command.as_str() {
        "audit" => &["--traces", "--model", "--out", "--positions", "--floor", "--margin", "--min-visits", "--shortcut", "--simulations", "--worlds", "--mode", "--starts", "--lanes", "--seed", "--backend", "--server", "--threads", "--prior", "--batch"],
        "verify" => &["--scenario", "--action", "--model", "--floor", "--worlds", "--simulations", "--mode", "--seed", "--out", "--backend", "--server", "--threads", "--prior", "--batch"],
        other => return Err(format!("no command {other}")),
    };
    if let Some(k) = flags.keys().find(|k| !known.contains(&k.as_str())) {
        return Err(format!("unknown argument {k}"));
    }
    let evaluator = || load(&model, &backend, threads, server.as_deref());
    evaluator()?;
    let report = match command.as_str() {
        "audit" => {
            let mut o = AuditOptions::default();
            if let Some(m) = mode {
                o.mode = m;
            }
            o.positions = num("--positions")?.map_or(o.positions, |x| x as usize);
            o.floor = num("--floor")?.map_or(o.floor, |x| x as f32);
            o.margin = num("--margin")?.map_or(o.margin, |x| x as f32);
            o.min_visits = num("--min-visits")?.map_or(o.min_visits, |x| x as u32);
            o.shortcut = num("--shortcut")?.map_or(o.shortcut, |x| x as f32);
            o.simulations = num("--simulations")?.map_or(o.simulations, |x| x as usize);
            o.worlds = num("--worlds")?.map_or(o.worlds, |x| (x as usize).max(1));
            o.batch = num("--batch")?.map_or(o.batch, |x| (x as usize).max(1));
            o.seed = num("--seed")?.map_or(o.seed, |x| x as u64);
            let lanes = num("--lanes")?.map_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1), |x| (x as usize).max(1));
            let dir = PathBuf::from(take("--traces").ok_or("no --traces")?);
            let dir = if dir.join("traces").is_dir() { dir.join("traces") } else { dir };
            let traces: BTreeMap<u64, _> = cantrip_ai_host::regret::load_traces(&dir)?.into_iter().collect();
            let starts = take("--starts").map(|d| Starts::load(Path::new(&d))).transpose()?;
            let out = PathBuf::from(take("--out").ok_or("no --out")?);
            explore::audit(&traces, starts.as_ref(), &o, &prior, lanes, &out, evaluator)?
        }
        _ => {
            let mut o = VerifyOptions::default();
            if let Some(m) = mode {
                o.mode = m;
            }
            o.floor = num("--floor")?.map_or(o.floor, |x| x as f32);
            o.worlds = num("--worlds")?.map_or(o.worlds, |x| (x as usize).max(1));
            o.simulations = num("--simulations")?.map_or(o.simulations, |x| x as usize);
            o.batch = num("--batch")?.map_or(o.batch, |x| (x as usize).max(1));
            o.seed = num("--seed")?.map_or(o.seed, |x| x as u64);
            let start = Start::load(Path::new(&take("--scenario").ok_or("no --scenario")?))?;
            let game = start.game.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let action = take("--action").ok_or("no --action")?;
            let v = explore::verify(&game, &action, evaluator()?.as_ref(), &prior, &o)?;
            if let Some(out) = take("--out") {
                std::fs::write(&out, serde_json::to_string_pretty(&v).unwrap_or_default()).map_err(|e| format!("{out}: {e}"))?;
            }
            v
        }
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    Ok(())
}

fn load(dir: &Path, backend: &str, threads: usize, server: Option<&str>) -> Result<Box<dyn Evaluator>, String> {
    let _ = threads;
    match backend {
        "remote" => Ok(Box::new(cantrip_ai::RemoteEvaluator::connect(server.ok_or("--backend remote needs --server")?, dir, table()).map_err(|e| format!("{}: {e}", dir.display()))?)),
        "tract" => Ok(Box::new(cantrip_ai::TractEvaluator::load(dir, table()).map_err(|e| format!("{}: {e}", dir.display()))?)),
        #[cfg(feature = "ort")]
        "ort" => Ok(Box::new(cantrip_ai::eval::OrtEvaluator::load(dir, table(), threads).map_err(|e| format!("{}: {e}", dir.display()))?)),
        other => Err(format!("no backend {other} in this build")),
    }
}

/// The pair miner's commands (`cantrip_ai_host::mine`).
fn mine(command: &str, flags: &BTreeMap<String, String>, dirs: BTreeMap<String, PathBuf>, root: &Path) -> Result<(), String> {
    use cantrip_ai_host::mine;
    let take = |k: &str| flags.get(k).cloned();
    let num = |k: &str| -> Result<Option<f64>, String> { take(k).map(|v| v.parse::<f64>().map_err(|_| format!("{k} takes a number, not {v}"))).transpose() };
    let known: &[&str] = match command {
        "mine-forks" => &["--first", "--second", "--out", "--lanes"],
        "mine-filter" => &["--from", "--keep", "--out", "--producer"],
        "mine-values" => &["--dir", "--games", "--out", "--lanes", "--backend", "--server", "--threads", "--prior"],
        "mine-judge" => &[
            "--plan", "--model", "--out", "--simulations", "--worlds", "--floor", "--batch", "--seed", "--lanes", "--targets", "--producer", "--mode", "--backend",
            "--server", "--threads", "--prior", "--search",
        ],
        other => return Err(format!("no command {other}")),
    };
    if let Some(k) = flags.keys().find(|k| !known.contains(&k.as_str())) {
        return Err(format!("unknown argument {k}"));
    }
    let lanes = num("--lanes")?.map_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1), |x| (x as usize).max(1));
    let backend = if flags.contains_key("--server") { "remote".to_string() } else { take("--backend").unwrap_or_else(|| if cfg!(feature = "ort") { "ort".into() } else { "tract".into() }) };
    let server = take("--server");
    let threads = num("--threads")?.unwrap_or(1.0) as usize;
    let report = match command {
        "mine-filter" => {
            #[derive(serde::Deserialize)]
            struct Kept {
                dir: String,
                seed: u64,
                decision: u32,
            }
            let keep_path = take("--keep").ok_or("no --keep")?;
            let keep: std::collections::HashSet<(String, u64, u32)> = std::fs::read_to_string(&keep_path)
                .map_err(|e| format!("{keep_path}: {e}"))?
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str::<Kept>(l).map(|k| (k.dir, k.seed, k.decision)).map_err(|e| format!("{keep_path}: {e}")))
                .collect::<Result<_, _>>()?;
            let from = PathBuf::from(take("--from").ok_or("no --from")?);
            let out = PathBuf::from(take("--out").ok_or("no --out")?);
            let producer = take("--producer").unwrap_or_else(|| "mine".into());
            let (games, decisions) = mine::filter(&from, &keep, &out, &producer)?;
            serde_json::json!({ "Games": games, "Decisions": decisions, "Asked": keep.len() })
        }
        "mine-forks" => {
            let first = mine::load(Path::new(&take("--first").ok_or("no --first")?))?;
            let second = mine::load(Path::new(&take("--second").ok_or("no --second")?))?;
            if first.mode != second.mode {
                return Err("the two directories were recorded in different modes".into());
            }
            let out = PathBuf::from(take("--out").ok_or("no --out")?);
            mine::forks(&first, &second, lanes, &out)?
        }
        "mine-values" => {
            #[derive(serde::Deserialize)]
            struct Wanted {
                seed: u64,
                seat: u8,
                model: String,
            }
            let rec = mine::load(Path::new(&take("--dir").ok_or("no --dir")?))?;
            let games_path = take("--games").ok_or("no --games")?;
            let wanted: Vec<Wanted> = std::fs::read_to_string(&games_path)
                .map_err(|e| format!("{games_path}: {e}"))?
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str(l).map_err(|e| format!("{games_path}: {e}")))
                .collect::<Result<_, _>>()?;
            let out = PathBuf::from(take("--out").ok_or("no --out")?);
            let next = std::sync::atomic::AtomicUsize::new(0);
            let lines = std::sync::Mutex::new(Vec::<serde_json::Value>::new());
            std::thread::scope(|scope| -> Result<(), String> {
                let mut handles = Vec::new();
                for _ in 0..lanes {
                    let (wanted, next, lines, rec, backend, server) = (&wanted, &next, &lines, &rec, &backend, &server);
                    handles.push(scope.spawn(move || -> Result<(), String> {
                        let mut evaluators: BTreeMap<String, Box<dyn Evaluator>> = BTreeMap::new();
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(w) = wanted.get(i) else { break };
                            if !evaluators.contains_key(&w.model) {
                                evaluators.insert(w.model.clone(), load(Path::new(&w.model), backend, threads, server.as_deref())?);
                            }
                            let v = match rec.games.get(&w.seed) {
                                Some(g) => match mine::values(g, rec.traces.get(&w.seed), w.seat, evaluators[&w.model].as_ref()) {
                                    Ok(values) => serde_json::json!({ "seed": w.seed, "seat": w.seat, "model": w.model, "values": values }),
                                    Err(e) => serde_json::json!({ "seed": w.seed, "seat": w.seat, "model": w.model, "error": e }),
                                },
                                None => serde_json::json!({ "seed": w.seed, "seat": w.seat, "model": w.model, "error": "no recorded game" }),
                            };
                            lines.lock().unwrap_or_else(|e| e.into_inner()).push(v);
                        }
                        Ok(())
                    }));
                }
                for h in handles {
                    h.join().map_err(|_| "a lane panicked".to_string())??;
                }
                Ok(())
            })?;
            let mut lines = lines.into_inner().unwrap_or_else(|e| e.into_inner());
            lines.sort_by_key(|v| (v["seed"].as_u64(), v["seat"].as_u64()));
            std::fs::write(&out, lines.iter().map(|v| format!("{v}\n")).collect::<String>()).map_err(|e| format!("{}: {e}", out.display()))?;
            serde_json::json!({ "Games": lines.len(), "Errors": lines.iter().filter(|v| v.get("error").is_some()).count() })
        }
        _ => {
            let model = PathBuf::from(take("--model").ok_or("no --model")?);
            let prior = cantrip_ai::Prior::from_decks(&take("--prior").map(PathBuf::from).unwrap_or(root.join("decks")))?;
            let mut o = mine::JudgeOptions::default();
            if let Some(m) = take("--mode") {
                o.mode = parse_mode(&m).ok_or(format!("no mode {m}"))?;
            }
            o.simulations = num("--simulations")?.map_or(o.simulations, |x| x as usize);
            o.worlds = num("--worlds")?.map_or(o.worlds, |x| (x as usize).max(1));
            o.floor = num("--floor")?.map_or(o.floor, |x| x as f32);
            o.batch = num("--batch")?.map_or(o.batch, |x| (x as usize).max(1));
            o.seed = take("--seed").map(|v| v.parse::<u64>().map_err(|_| format!("--seed takes a number, not {v}"))).transpose()?.unwrap_or(o.seed);
            // --search K=V[,K=V...]: the target searches' settings by name.
            for kv in take("--search").iter().flat_map(|v| v.split(',').map(str::to_string).collect::<Vec<_>>()) {
                let (k, v) = kv.split_once('=').ok_or(format!("--search takes K=V, not {kv}"))?;
                let mut probe = cantrip_ai::search::Config::default();
                probe.set(k, v)?;
                o.search.push((k.to_string(), v.to_string()));
            }
            let plan_path = take("--plan").ok_or("no --plan")?;
            let plan: Vec<mine::Planned> = std::fs::read_to_string(&plan_path)
                .map_err(|e| format!("{plan_path}: {e}"))?
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str(l).map_err(|e| format!("{plan_path}: {e}")))
                .collect::<Result<_, _>>()?;
            if dirs.is_empty() {
                return Err("no --dir NAME=DIR".into());
            }
            let targets = take("--targets").is_none_or(|v| v == "true");
            let run = mine::JudgeRun {
                dirs,
                out: PathBuf::from(take("--out").ok_or("no --out")?),
                lanes,
                producer: take("--producer").unwrap_or_else(|| format!("mine:{}@{}", cantrip_ai_host::selfplay::model_id(&model), o.simulations)),
                targets,
            };
            let evaluator = || load(&model, &backend, threads, server.as_deref());
            evaluator()?;
            mine::run(&run, &plan, &o, &prior, evaluator)?
        }
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    Ok(())
}
