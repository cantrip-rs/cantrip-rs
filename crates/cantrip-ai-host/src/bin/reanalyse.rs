//! `cantrip-reanalyse`: self-played games' chosen positions searched again by the newest network, deeply and in bulk,
//! and written as shards of search targets (`cantrip_ai_host::reanalyse`; the review of 2026-11-13, item 8). The games
//! need their traces: self-play run with `--traces`.
//!
//! `cantrip-reanalyse --selfplay DIR [--selfplay DIR ...] --model DIR --out DIR [--positions N] [--random-share P]
//!     [--flat P] [--swing V] [--before-swing N] [--simulations N] [--worlds N] [--batch N] [--lanes N] [--seed N]
//!     [--backend ort|tract|remote] [--server HOST:PORT] [--threads N] [--prior DIR] [--starts DIR]`
//!
//! The report is printed and written to `OUT/reanalyse-report.json`; the chosen positions, with why, to
//! `OUT/reanalyse-positions.jsonl`.

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::path::{Path, PathBuf};

use cantrip_ai::Evaluator;
use cantrip_ai::record::table;
use cantrip_ai_host::reanalyse::{self, Options, Run};
use cantrip_ai_host::selfplay;

const USAGE: &str = "usage: cantrip-reanalyse --selfplay DIR [--selfplay DIR ...] --model DIR --out DIR [--positions N] [--random-share P]
    [--flat P] [--swing V] [--before-swing N] [--simulations N] [--worlds N] [--batch N] [--lanes N] [--seed N]
    [--backend ort|tract|remote] [--server HOST:PORT] [--threads N] [--prior DIR] [--starts DIR]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let worker = std::thread::Builder::new().stack_size(cantrip_ai_host::STACK_SIZE).spawn(move || go(&args)).expect("the main thread starts");
    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("cantrip-reanalyse: {e}\n{USAGE}");
            std::process::exit(2);
        }
        Err(_) => std::process::exit(1),
    }
}

fn go(args: &[String]) -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut model: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut lanes = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let mut backend = if cfg!(feature = "ort") { "ort".to_string() } else { "tract".to_string() };
    let mut server: Option<String> = None;
    let mut threads = 1usize;
    let mut prior_dir: Option<PathBuf> = None;
    let mut starts: Option<PathBuf> = None;
    let mut o = Options::default();
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().ok_or(format!("{} needs a value", args[i]))?;
        let num = |v: &str| v.parse::<f64>().map_err(|_| format!("{} takes a number, not {v}", args[i]));
        match args[i].as_str() {
            "--selfplay" => dirs.push(PathBuf::from(v)),
            "--model" => model = Some(PathBuf::from(v)),
            "--out" => out = Some(PathBuf::from(v)),
            "--positions" => o.positions = num(&v)? as usize,
            "--random-share" => o.random_share = num(&v)? as f32,
            "--flat" => o.flat = num(&v)? as f32,
            "--swing" => o.swing = num(&v)? as f32,
            "--before-swing" => o.before_swing = num(&v)? as usize,
            "--simulations" => o.simulations = num(&v)? as usize,
            "--worlds" => o.worlds = (num(&v)? as usize).max(1),
            "--batch" => o.batch = (num(&v)? as usize).max(1),
            "--lanes" => lanes = (num(&v)? as usize).max(1),
            "--seed" => o.seed = v.parse().map_err(|_| format!("--seed takes a number, not {v}"))?,
            "--backend" => backend = v,
            "--server" => server = Some(v),
            "--threads" => threads = (num(&v)? as usize).max(1),
            "--prior" => prior_dir = Some(PathBuf::from(v)),
            "--starts" => starts = Some(PathBuf::from(v)),
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    if dirs.is_empty() {
        return Err("no --selfplay".into());
    }
    let model = model.ok_or("no --model")?;
    let out = out.ok_or("no --out")?;
    if server.is_some() {
        backend = "remote".into();
    }
    let prior = cantrip_ai::Prior::from_decks(prior_dir.as_deref().unwrap_or(&root.join("decks")))?;
    let run = Run {
        dirs,
        out,
        lanes,
        producer: format!("reanalyse:{}@{}", selfplay::model_id(&model), o.simulations),
        starts: starts.as_deref().map(selfplay::Starts::load).transpose()?,
    };
    load(&model, &backend, threads, server.as_deref())?;
    let report = reanalyse::run(&run, &o, &prior, || load(&model, &backend, threads, server.as_deref()))?;
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
