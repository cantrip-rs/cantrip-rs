//! `cantrip-selfplay`: games of one model against itself, both seats searching, recorded with the search's visits
//! (`cantrip_ai_host::selfplay`; docs/ai-player-plan.md, "Phase 4"). Progress goes to stderr; the run's report is
//! printed on stdout and written to `OUT/selfplay-report.json`.

use std::path::PathBuf;
use std::time::Duration;

use cantrip_ai::Evaluator;
use cantrip_ai::record::table;
use cantrip_ai::shard::parse_mode;
use cantrip_ai_host::selfplay::{self, Options, Pairing, Run};

const USAGE: &str = "usage: cantrip-selfplay --model DIR --out DIR [--games N] [--hours H] [--simulations N] [--lanes N] [--seed N]
    [--pairing DECK:DECK ...] [--decks DIR] [--sampled N] [--temperature T] [--mode full|decklist|hidden] [--worlds N] [--max-turns N] [--max-answers N] [--backend ort|tract] [--threads N] [--prior DIR]
    [--resume] [--shard-games N]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let worker = std::thread::Builder::new()
        .stack_size(cantrip_ai_host::STACK_SIZE)
        .spawn(move || go(&args))
        .expect("the main thread starts");
    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("cantrip-selfplay: {e}\n{USAGE}");
            std::process::exit(2);
        }
        Err(_) => std::process::exit(1),
    }
}

fn go(args: &[String]) -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut model: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut games = 1000usize;
    let mut hours: Option<f64> = None;
    let mut lanes = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut seed = 1u64;
    let mut pairings: Vec<String> = Vec::new();
    let mut decks = root.join("decks");
    let mut prior_dir: Option<PathBuf> = None;
    let mut backend = if cfg!(feature = "ort") {
        "ort".to_string()
    } else {
        "tract".to_string()
    };
    let mut threads = 1usize;
    let mut shard_games = 100usize;
    let mut resume = false;
    let mut o = Options::default();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--resume" {
            resume = true;
            i += 1;
            continue;
        }
        let v = args
            .get(i + 1)
            .cloned()
            .ok_or(format!("{} needs a value", args[i]))?;
        let num = |v: &str| {
            v.parse::<f64>()
                .map_err(|_| format!("{} takes a number, not {v}", args[i]))
        };
        match args[i].as_str() {
            "--model" => model = Some(PathBuf::from(v)),
            "--out" => out = Some(PathBuf::from(v)),
            "--games" => games = num(&v)? as usize,
            "--hours" => hours = Some(num(&v)?),
            "--simulations" => o.simulations = num(&v)? as usize,
            "--lanes" => lanes = (num(&v)? as usize).max(1),
            "--seed" => {
                seed = v
                    .parse()
                    .map_err(|_| format!("--seed takes a number, not {v}"))?
            }
            "--pairing" => pairings.push(v),
            "--decks" => decks = PathBuf::from(v),
            "--sampled" => o.sampled = num(&v)? as u32,
            "--temperature" => o.temperature = num(&v)? as f32,
            "--mode" => o.mode = parse_mode(&v).ok_or(format!("no mode {v}"))?,
            "--worlds" => o.worlds = num(&v)? as usize,
            "--max-turns" => o.max_turns = num(&v)? as u32,
            "--max-answers" => o.max_answers = num(&v)? as u32,
            "--backend" => backend = v,
            "--threads" => threads = (num(&v)? as usize).max(1),
            "--prior" => prior_dir = Some(PathBuf::from(v)),
            "--shard-games" => shard_games = (num(&v)? as usize).max(1),
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    let model = model.ok_or("no --model")?;
    let out = out.ok_or("no --out")?;
    if pairings.is_empty() {
        pairings = vec![
            "rw-aggro:ub-reanimator".into(),
            "rw-aggro:rw-aggro".into(),
            "ub-reanimator:ub-reanimator".into(),
        ];
    }
    let pairings = pairings
        .iter()
        .map(|p| {
            let (a, b) = p
                .split_once(':')
                .ok_or(format!("a pairing is DECK:DECK, not {p}"))?;
            Ok(Pairing {
                names: [a.to_string(), b.to_string()],
                lists: [
                    selfplay::load_deck(&decks.join(format!("{a}.txt")))?,
                    selfplay::load_deck(&decks.join(format!("{b}.txt")))?,
                ],
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let prior = cantrip_ai::Prior::from_decks(prior_dir.as_deref().unwrap_or(&decks))?;
    let id = selfplay::model_id(&model);
    o.model = model
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "model".into());
    let run = Run {
        out,
        games,
        limit: hours.map(|h| Duration::from_secs_f64(h * 3600.0)),
        lanes,
        seed,
        pairings,
        producer: format!("selfplay:{id}@{}", o.simulations),
        shard_mb: 64,
        shard_games,
        resume,
    };
    // Load once here, so a bad model fails before the lanes start.
    load(&model, &backend, threads)?;
    eprintln!(
        "cantrip-selfplay: {} @{} on {lanes} lanes ({backend}), {} mode, {games} games from seed {seed}",
        run.producer,
        o.simulations,
        cantrip_ai::shard::mode_name(o.mode)
    );
    let report = selfplay::run(&run, &o, &prior, || load(&model, &backend, threads))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    Ok(())
}

fn load(
    dir: &std::path::Path,
    backend: &str,
    threads: usize,
) -> Result<Box<dyn Evaluator>, String> {
    let _ = threads;
    match backend {
        "tract" => Ok(Box::new(
            cantrip_ai::TractEvaluator::load(dir, table())
                .map_err(|e| format!("{}: {e}", dir.display()))?,
        )),
        #[cfg(feature = "ort")]
        "ort" => Ok(Box::new(
            cantrip_ai::eval::OrtEvaluator::load(dir, table(), threads)
                .map_err(|e| format!("{}: {e}", dir.display()))?,
        )),
        other => Err(format!("no backend {other} in this build")),
    }
}
