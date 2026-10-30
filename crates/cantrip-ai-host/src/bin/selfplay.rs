//! `cantrip-selfplay`: games of one model against itself, both seats searching, recorded with the search's visits
//! (`cantrip_ai_host::selfplay`; docs/ai-player-plan.md, "Phase 4"). Progress goes to stderr; the run's report is
//! printed on stdout and written to `OUT/selfplay-report.json`.

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::path::PathBuf;
use std::time::Duration;

use cantrip_ai::Evaluator;
use cantrip_ai::record::table;
use cantrip_ai::shard::parse_mode;
use cantrip_ai_host::selfplay::{self, Options, Pairing, Run};

const USAGE: &str = "usage: cantrip-selfplay --model DIR --out DIR [--games N] [--hours H] [--simulations N] [--lanes N] [--seed N]
    [--pairing DECK:DECK ...] [--pairings FILE] [--decks DIR] [--sampled N] [--temperature T] [--mode full|decklist|hidden] [--also-record MODE ...] [--traces] [--worlds N] [--max-turns N] [--max-answers N] [--backend ort|tract|remote|gpu] [--server HOST:PORT] [--threads N] [--prior DIR]
    [--resume] [--shard-games N] [--precision fp32|tf32|fp16] [--gpu-batch N|32:N,64:N,128:N] [--pipelines N] [--flush-ms MS] [--device N]";

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
    let mut server: Option<String> = None;
    let mut shard_games = 100usize;
    let mut resume = false;
    let mut o = Options::default();
    let mut gpu: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--resume" {
            resume = true;
            i += 1;
            continue;
        }
        if args[i] == "--traces" {
            o.traces = true;
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
            // One DECK:DECK a line: the generation driver's metagame list, a pairing a game, is longer than Windows
            // lets a command line be.
            "--pairings" => pairings.extend(
                std::fs::read_to_string(&v)
                    .map_err(|e| format!("{v}: {e}"))?
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(String::from),
            ),
            "--decks" => decks = PathBuf::from(v),
            "--sampled" => o.sampled = num(&v)? as u32,
            "--temperature" => o.temperature = num(&v)? as f32,
            "--mode" => o.mode = parse_mode(&v).ok_or(format!("no mode {v}"))?,
            "--also-record" => o.also.push(parse_mode(&v).ok_or(format!("no mode {v}"))?),
            "--worlds" => o.worlds = num(&v)? as usize,
            "--max-turns" => o.max_turns = num(&v)? as u32,
            "--max-answers" => o.max_answers = num(&v)? as u32,
            "--backend" => backend = v,
            "--server" => server = Some(v),
            "--threads" => threads = (num(&v)? as usize).max(1),
            "--prior" => prior_dir = Some(PathBuf::from(v)),
            "--shard-games" => shard_games = (num(&v)? as usize).max(1),
            "--batch" => o.batch = (num(&v)? as usize).max(1),
            "--full-simulations" => o.full_simulations = num(&v)? as usize,
            "--full-share" => o.full_share = v.parse::<f32>().map_err(|e| format!("--full-share: {e}"))?,
            // The GPU engine's (`--backend gpu`): as cantrip-infer takes them.
            "--precision" | "--gpu-batch" | "--pipelines" | "--flush-ms" | "--device" => {
                gpu.push((args[i].clone(), v))
            }
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    let model = model.ok_or("no --model")?;
    let out = out.ok_or("no --out")?;
    #[cfg(feature = "gpu")]
    let gpu_engine = if backend == "gpu" {
        Some(gpu_engine(&model, &gpu)?)
    } else {
        None
    };
    #[cfg(not(feature = "gpu"))]
    if backend == "gpu" || !gpu.is_empty() {
        return Err("--backend gpu needs a build with --features gpu".into());
    }
    if server.is_some() && backend != "remote" {
        backend = "remote".into();
    }
    if backend == "remote" && server.is_none() {
        return Err("--backend remote needs --server HOST:PORT".into());
    }
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
    #[cfg(feature = "gpu")]
    let engine = gpu_engine.as_ref();
    #[cfg(not(feature = "gpu"))]
    let engine: Option<&()> = None;
    // Load once here, so a bad model fails before the lanes start.
    load(&model, &backend, threads, server.as_deref(), engine)?;
    eprintln!(
        "cantrip-selfplay: {} @{} on {lanes} lanes ({backend}), {} mode, {games} games from seed {seed}",
        run.producer,
        o.simulations,
        cantrip_ai::shard::mode_name(o.mode)
    );
    let report = selfplay::run(&run, &o, &prior, || {
        load(&model, &backend, threads, server.as_deref(), engine)
    })?;
    #[cfg(feature = "gpu")]
    if let Some(e) = &gpu_engine {
        eprintln!("{}", e.summary());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    Ok(())
}

/// The GPU engine every lane of `--backend gpu` shares, its model checked against its references (cantrip-infer's
/// options: `--precision fp32|tf32|fp16`, `--gpu-batch N|32:N,64:N,128:N`, `--pipelines N`, `--flush-ms MS`,
/// `--device N`).
#[cfg(feature = "gpu")]
fn gpu_engine(
    model: &std::path::Path,
    args: &[(String, String)],
) -> Result<std::sync::Arc<cantrip_ai::gpu::Engine>, String> {
    use cantrip_ai::gpu::{self, Precision};
    let mut options = gpu::Options::default();
    for (flag, v) in args {
        let num = |v: &str| v.parse::<f64>().map_err(|_| format!("{flag} takes a number, not {v}"));
        match flag.as_str() {
            "--precision" => {
                options.precision = Precision::parse(v).ok_or(format!("no precision {v} (fp32, tf32, fp16)"))?
            }
            "--gpu-batch" => options.batch = gpu::parse_batch(v)?,
            "--pipelines" => options.pipelines = (num(v)? as usize).max(1),
            "--flush-ms" => options.flush = Duration::from_secs_f64(num(v)? / 1000.0),
            "--device" => options.device = num(v)? as i32,
            _ => {}
        }
    }
    let engine = std::sync::Arc::new(gpu::Engine::start(&[model], options).map_err(|e| e.to_string())?);
    let c = gpu::check(&engine, 0).map_err(|e| e.to_string())?;
    eprintln!(
        "cantrip-selfplay: the GPU engine's references {} at {}: {c:?}",
        if c.pass { "pass" } else { "FAIL" },
        engine.options().precision.name()
    );
    if !c.pass {
        return Err(format!("{}: the GPU engine's outputs are not the model's references", model.display()));
    }
    Ok(engine)
}

#[cfg(feature = "gpu")]
type Engine = std::sync::Arc<cantrip_ai::gpu::Engine>;
#[cfg(not(feature = "gpu"))]
type Engine = ();

fn load(
    dir: &std::path::Path,
    backend: &str,
    threads: usize,
    server: Option<&str>,
    engine: Option<&Engine>,
) -> Result<Box<dyn Evaluator>, String> {
    let _ = (threads, engine);
    match backend {
        // A lane an evaluator, every lane's rows batched together in this process.
        #[cfg(feature = "gpu")]
        "gpu" => Ok(Box::new(
            cantrip_ai::gpu::GpuEvaluator::new(engine.ok_or("no GPU engine")?.clone(), 0, table())
                .map_err(|e| format!("{}: {e}", dir.display()))?,
        )),
        // A lane a connection: the server batches every lane's leaves together.
        "remote" => Ok(Box::new(
            cantrip_ai::RemoteEvaluator::connect(server.unwrap_or_default(), dir, table())
                .map_err(|e| format!("{}: {e}", dir.display()))?,
        )),
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
