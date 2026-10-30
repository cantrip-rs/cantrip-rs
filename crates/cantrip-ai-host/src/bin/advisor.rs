//! `cantrip-advisor-host`: the AI host of the advisor's table (`dotnet/tools/Cantrip.Advisor`), a process of its own. It
//! speaks the AI host's protocol (docs/ai-host.md) on stdin and stdout, stderr its log, with the advisor's additions:
//! `AiStart`'s `Report` (`cantrip_ai_host::advisor`) and `Trace`. Beside the models it answers with, it loads one network
//! on the CPU for its belief head (`--belief DIR`), which the report reads; without it the report has no belief.
//!
//! ```text
//! cantrip-advisor-host --model DIR [--model DIR ...] [--backend tract|remote] [--server HOST:PORT] [--workers N]
//!     [--prior DIR] [--belief DIR]
//! ```

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cantrip_ai::Evaluator;
use cantrip_ai::record::table;
use cantrip_ai_host::{AiHost, Loaded, STACK_SIZE, Sink};

const USAGE: &str = "usage: cantrip-advisor-host --model DIR [--model DIR ...] [--backend tract|remote] [--server HOST:PORT] [--workers N] [--prior DIR] [--belief DIR]";

/// What a search holds is counted, as in the AI host.
#[global_allocator]
static ALLOCATOR: cantrip_ai::alloc::Counting = cantrip_ai::alloc::Counting;

struct Config {
    models: Vec<PathBuf>,
    backend: String,
    server: Option<String>,
    workers: usize,
    prior: Option<PathBuf>,
    belief: Option<PathBuf>,
}

fn config(args: &[String]) -> Result<Config, String> {
    let mut c = Config {
        models: Vec::new(),
        backend: "tract".into(),
        server: None,
        workers: 2,
        prior: None,
        belief: None,
    };
    let mut i = 0;
    while i < args.len() {
        let value = args
            .get(i + 1)
            .cloned()
            .ok_or(format!("{} needs a value", args[i]))?;
        match args[i].as_str() {
            "--model" => c.models.push(PathBuf::from(value)),
            "--backend" => c.backend = value,
            "--server" => c.server = Some(value),
            "--workers" => {
                c.workers = value
                    .parse::<usize>()
                    .map_err(|_| format!("--workers takes a number, not {value}"))?
                    .max(1)
            }
            "--prior" => c.prior = Some(PathBuf::from(value)),
            "--belief" => c.belief = Some(PathBuf::from(value)),
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    if c.models.is_empty() {
        return Err("no --model".into());
    }
    if c.server.is_some() {
        c.backend = "remote".into();
    }
    if c.backend == "remote" && c.server.is_none() {
        return Err("--backend remote needs --server HOST:PORT".into());
    }
    Ok(c)
}

fn load(c: &Config) -> Result<Vec<Loaded>, String> {
    let mut loaded = Vec::new();
    for dir in &c.models {
        let started = Instant::now();
        let evaluator: Arc<dyn Evaluator> = match c.backend.as_str() {
            "remote" => Arc::new(
                cantrip_ai::RemoteEvaluator::connect(
                    c.server.as_deref().unwrap_or_default(),
                    dir,
                    table(),
                )
                .map_err(|e| format!("{}: {e}", dir.display()))?,
            ),
            "tract" => Arc::new(
                cantrip_ai::TractEvaluator::load(dir, table())
                    .map_err(|e| format!("{}: {e}", dir.display()))?,
            ),
            other => return Err(format!("no backend {other} in the advisor's host")),
        };
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| dir.display().to_string());
        eprintln!(
            "cantrip-advisor-host: model {name} ({}) on {} in {:?}",
            dir.display(),
            c.backend,
            started.elapsed()
        );
        loaded.push(Loaded { name, evaluator });
    }
    Ok(loaded)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let c = match config(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cantrip-advisor-host: {e}\n{USAGE}");
            std::process::exit(2);
        }
    };
    let worker = std::thread::Builder::new()
        .name("cantrip-advisor-host".into())
        .stack_size(STACK_SIZE)
        .spawn(move || serve(c))
        .expect("the host thread starts");
    match worker.join() {
        Ok(true) => {}
        _ => std::process::exit(1),
    }
}

fn serve(c: Config) -> bool {
    let models = match load(&c) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("cantrip-advisor-host: {e}");
            return false;
        }
    };
    let out = Arc::new(Mutex::new(io::stdout()));
    let write = {
        let out = Arc::clone(&out);
        move |v: &serde_json::Value| {
            let text = serde_json::to_string(v).expect("a line serialises");
            let mut o = out.lock().unwrap_or_else(|e| e.into_inner());
            let _ = writeln!(o, "{text}").and_then(|_| o.flush());
        }
    };
    let sink: Sink = Arc::new(write.clone());
    let mut host = AiHost::new(models, &c.backend, c.workers, sink);
    if let Some(dir) = &c.prior {
        match cantrip_ai::Prior::from_decks(dir) {
            Ok(prior) => host = host.with_prior(prior),
            Err(e) => {
                eprintln!("cantrip-advisor-host: --prior: {e}");
                return false;
            }
        }
    }
    if let Some(dir) = &c.belief {
        match cantrip_ai::TractEvaluator::load(dir, table()) {
            Ok(belief) => host = host.with_belief(belief),
            Err(e) => {
                eprintln!("cantrip-advisor-host: --belief: {e}");
                return false;
            }
        }
    }
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = host.line(&line);
        write(&reply);
    }
    host.finish();
    true
}
