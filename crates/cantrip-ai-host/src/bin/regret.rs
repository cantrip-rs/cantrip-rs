//! `cantrip-regret`: the regret measure's reference (`cantrip_ai_host::regret`), and checks of the traces it stands
//! positions in.
//!
//! - `check --traces DIR`: replays every trace in the directory to its end and checks it is the game recorded: every
//!   answer taken, the same result.
//! - `value --traces DIR --positions FILE --out FILE --model DIR (--server HOST:PORT | --backend tract)`: values each
//!   position of FILE (a JSON line each: `id`, `seed`, `at`) into OUT, a JSON line each; a rerun skips the positions OUT
//!   holds. `--worlds N`, `--top N`, `--turns N`, `--lanes N`, `--seed N`.

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::path::PathBuf;

use cantrip_ai::Evaluator;
use cantrip_ai::record::table;
use cantrip_ai_host::regret::{self, Options, Position};
use cantrip_engine::Outcome;

const USAGE: &str = "usage: cantrip-regret check --traces DIR
       cantrip-regret value --traces DIR --positions FILE --out FILE --model DIR [--server HOST:PORT] [--lanes N] [--worlds N] [--top N] [--turns N] [--seed N]
         [--player NAME=pimc:W:SIMS|is:W:EVALS[:extend] ...] [--merge EARLIER_OUT]
       cantrip-regret luck --traces DIR --positions FILE --out FILE --model DIR [--server HOST:PORT] [--lanes N]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let worker = std::thread::Builder::new().stack_size(cantrip_ai_host::STACK_SIZE).spawn(move || go(&args)).expect("the main thread starts");
    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("cantrip-regret: {e}\n{USAGE}");
            std::process::exit(2);
        }
        Err(_) => std::process::exit(1),
    }
}

fn go(args: &[String]) -> Result<(), String> {
    let command = args.first().ok_or("no command")?.clone();
    let mut traces: Option<PathBuf> = None;
    let mut positions: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut model: Option<PathBuf> = None;
    let mut server: Option<String> = None;
    let mut lanes = 64usize;
    let mut merge: Option<PathBuf> = None;
    let mut o = Options::default();
    let mut i = 1;
    while i < args.len() {
        let v = args.get(i + 1).cloned().ok_or(format!("{} needs a value", args[i]))?;
        let num = |v: &str| v.parse::<u64>().map_err(|_| format!("{} takes a number, not {v}", args[i]));
        match args[i].as_str() {
            "--traces" => traces = Some(PathBuf::from(v)),
            "--positions" => positions = Some(PathBuf::from(v)),
            "--out" => out = Some(PathBuf::from(v)),
            "--model" => model = Some(PathBuf::from(v)),
            "--server" => server = Some(v),
            "--lanes" => lanes = num(&v)? as usize,
            "--worlds" => o.worlds = num(&v)? as usize,
            "--top" => o.top = num(&v)? as usize,
            "--turns" => o.turns = num(&v)? as u32,
            "--seed" => o.seed = num(&v)?,
            "--player" => {
                let (name, spec) = v.split_once('=').ok_or(format!("--player NAME=SPEC, not {v}"))?;
                o.players.push((name.to_string(), regret::Player::parse(spec)?));
            }
            "--merge" => merge = Some(PathBuf::from(v)),
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    let traces = regret::load_traces(&traces.ok_or("no --traces")?)?;
    match command.as_str() {
        "check" => {
            let (mut ok, mut incomplete, mut bad) = (0, 0, 0);
            let mut seeds: Vec<&u64> = traces.keys().collect();
            seeds.sort();
            for seed in seeds {
                let t = &traces[seed];
                if !t.complete {
                    incomplete += 1;
                }
                match t.replay(t.steps.len()) {
                    Ok((game, _)) => {
                        let result: Vec<i8> = (0..game.state.players.len())
                            .map(|p| match game.state.outcome {
                                Outcome::Won(w) if w.index() == p => 1,
                                Outcome::Won(_) => -1,
                                _ => 0,
                            })
                            .collect();
                        if t.complete && (!game.is_over() || result != t.outcome) && t.outcome.iter().any(|&r| r != 0) {
                            bad += 1;
                            eprintln!("seed {seed}: replayed to {:?} (over: {}), recorded {:?}", result, game.is_over(), t.outcome);
                        } else {
                            ok += 1;
                        }
                    }
                    Err(e) => {
                        bad += 1;
                        eprintln!("seed {seed}: {e}");
                    }
                }
            }
            println!("{ok} traces replay, {bad} do not; {incomplete} were cut short");
            if bad > 0 {
                return Err(format!("{bad} traces do not replay"));
            }
            Ok(())
        }
        "value" => {
            let positions_path = positions.ok_or("no --positions")?;
            let text = std::fs::read_to_string(&positions_path).map_err(|e| format!("{}: {e}", positions_path.display()))?;
            let positions: Vec<Position> = text.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
            let model = model.ok_or("no --model")?;
            let out = out.ok_or("no --out")?;
            let load = || -> Result<Box<dyn Evaluator>, String> {
                match &server {
                    Some(s) => Ok(Box::new(cantrip_ai::RemoteEvaluator::connect(s, &model, table()).map_err(|e| format!("{}: {e}", model.display()))?)),
                    None => Ok(Box::new(cantrip_ai::TractEvaluator::load(&model, table()).map_err(|e| format!("{}: {e}", model.display()))?)),
                }
            };
            load()?;
            eprintln!("cantrip-regret: {} positions, {} worlds, top {}, {} turns, on {lanes} lanes", positions.len(), o.worlds, o.top, o.turns);
            let before: std::collections::HashMap<u64, serde_json::Value> = match &merge {
                Some(m) => std::fs::read_to_string(m)
                    .map_err(|e| format!("{}: {e}", m.display()))?
                    .lines()
                    .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                    .filter(|v| v.get("error").is_none())
                    .filter_map(|v| Some((v["id"].as_u64()?, v)))
                    .collect(),
                None => Default::default(),
            };
            let (ok, failed) = regret::run(&traces, &positions, &out, lanes, &o, &before, load)?;
            println!("{ok} positions valued, {failed} failed, into {}", out.display());
            Ok(())
        }
        "luck" => {
            let positions_path = positions.ok_or("no --positions")?;
            let text = std::fs::read_to_string(&positions_path).map_err(|e| format!("{}: {e}", positions_path.display()))?;
            let positions: Vec<Position> = text.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
            let model = model.ok_or("no --model")?;
            let out = out.ok_or("no --out")?;
            let next = std::sync::atomic::AtomicUsize::new(0);
            let lines = std::sync::Mutex::new(Vec::new());
            std::thread::scope(|scope| -> Result<(), String> {
                let mut hs = Vec::new();
                for _ in 0..lanes.max(1) {
                    let (positions, next, lines, model, server, traces) = (&positions, &next, &lines, &model, &server, &traces);
                    hs.push(std::thread::Builder::new().stack_size(cantrip_ai_host::STACK_SIZE).spawn_scoped(scope, move || -> Result<(), String> {
                        let evaluator: Box<dyn Evaluator> = match server {
                            Some(s) => Box::new(cantrip_ai::RemoteEvaluator::connect(s, model, table()).map_err(|e| e.to_string())?),
                            None => Box::new(cantrip_ai::TractEvaluator::load(model, table()).map_err(|e| e.to_string())?),
                        };
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(p) = positions.get(i) else { break };
                            let line = match traces.get(&p.seed) {
                                Some(t) => regret::luck(t, p, evaluator.as_ref()).unwrap_or_else(|e| serde_json::json!({"id": p.id, "error": e})),
                                None => serde_json::json!({"id": p.id, "error": "no trace"}),
                            };
                            lines.lock().unwrap_or_else(|e| e.into_inner()).push(line.to_string());
                        }
                        Ok(())
                    }).map_err(|e| e.to_string())?);
                }
                for h in hs {
                    h.join().map_err(|_| "a lane panicked".to_string())??;
                }
                Ok(())
            })?;
            let lines = lines.into_inner().unwrap_or_default();
            std::fs::write(&out, lines.join("\n") + "\n").map_err(|e| format!("{}: {e}", out.display()))?;
            println!("{} positions into {}", lines.len(), out.display());
            Ok(())
        }
        other => Err(format!("no command {other}")),
    }
}
