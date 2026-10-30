//! `cantrip-ai-host`: the AI player beside a table, over JSON lines on stdin and stdout (docs/ai-host.md); stderr is
//! the log. The models are loaded once, at start, each checked against this build's encoding (`Model::load`).

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cantrip_ai::Evaluator;
use cantrip_ai::record::table;
use cantrip_ai_host::{AiHost, Loaded, STACK_SIZE, Sink};

const USAGE: &str = "usage: cantrip-ai-host --model DIR [--model DIR ...] [--backend ort|tract|remote] [--server HOST:PORT] [--threads N] [--workers N] [--prior DIR]";

/// What a search holds is counted (`cantrip_ai::alloc`, `AiResult.Search.TreeBytes`).
#[global_allocator]
static ALLOCATOR: cantrip_ai::alloc::Counting = cantrip_ai::alloc::Counting;

struct Config {
    models: Vec<PathBuf>,
    backend: String,
    server: Option<String>,
    threads: usize,
    workers: usize,
    prior: Option<PathBuf>,
}

fn config(args: &[String]) -> Result<Config, String> {
    let default_backend = if cfg!(feature = "ort") {
        "ort"
    } else {
        "tract"
    };
    let mut c = Config {
        models: Vec::new(),
        backend: default_backend.into(),
        server: None,
        threads: 1,
        workers: 1,
        prior: None,
    };
    let mut i = 0;
    while i < args.len() {
        let value = args
            .get(i + 1)
            .cloned()
            .ok_or(format!("{} needs a value", args[i]))?;
        let number = |v: &str| {
            v.parse::<usize>()
                .map_err(|_| format!("{} takes a number, not {v}", args[i]))
        };
        match args[i].as_str() {
            "--model" => c.models.push(PathBuf::from(value)),
            "--backend" => c.backend = value,
            "--server" => c.server = Some(value),
            "--threads" => c.threads = number(&value)?.max(1),
            "--workers" => c.workers = number(&value)?.max(1),
            "--prior" => c.prior = Some(PathBuf::from(value)),
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
            // The workers share the evaluator, each on a connection of its own while it evaluates.
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
            #[cfg(feature = "ort")]
            "ort" => Arc::new(
                cantrip_ai::eval::OrtEvaluator::load(dir, table(), c.threads)
                    .map_err(|e| format!("{}: {e}", dir.display()))?,
            ),
            other => return Err(format!("no backend {other} in this build")),
        };
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| dir.display().to_string());
        eprintln!(
            "cantrip-ai-host: model {name} ({}) on {} in {:?}",
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
            eprintln!("cantrip-ai-host: {e}\n{USAGE}");
            std::process::exit(2);
        }
    };
    let worker = std::thread::Builder::new()
        .name("cantrip-ai-host".into())
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
            eprintln!("cantrip-ai-host: {e}");
            return false;
        }
    };
    // Replies and results share stdout, a line at a time.
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
                eprintln!("cantrip-ai-host: --prior: {e}");
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
