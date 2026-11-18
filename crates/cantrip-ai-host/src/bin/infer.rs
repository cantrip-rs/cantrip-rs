//! `cantrip-infer`: the native inference server (docs/ai-host.md, "The native inference server"). It serves
//! `ai/cantrip_ai/serve.py`'s protocol (`cantrip_ai::remote`) from `cantrip_ai::gpu`'s engine: onnxruntime's CUDA
//! provider, every connection's rows batched together, no Python on the request path. `--backend remote --server` and
//! BotLab's `--ai-server` use it as they use serve.py.
//!
//! - `cantrip-infer serve --model DIR [--model DIR ...] [--host 127.0.0.1] [--port 7710] [--precision fp32|tf32|fp16]
//!   [--batch N | 32:N,64:N,96:N,128:N] [--shapes N,N,...|T:N/N,...|none] [--flush-ms 2] [--pipelines 2]
//!   [--device 0] [--no-check] [--ready-file PATH] [--log-every 60] [--priority high|normal]` (`serve` may be left out);
//! - `cantrip-infer bench --model DIR [--clients 64] [--rows 8|A-B] [--mix T:share,...] [--think-ms MS] [--seconds 20]
//!   [engine options]`: the engine alone, fed the model's reference positions by `--clients` threads of `--rows` rows a
//!   request (drawn from each bucket at `--mix`'s shares, a client pausing `--think-ms` between requests as a self-play
//!   lane searches): rows a second, the share of padded rows real, and latency;
//! - `cantrip-infer compare --model DIR --a HOST:PORT --b HOST:PORT [--shards DIR] [--limit N]`: the same positions (the
//!   references, and a shard directory's) through two servers: how far apart their outputs are;
//! - `cantrip-infer check --model DIR [--model DIR ...] [engine options]`: the engine started and each model held to its
//!   references, as `serve` starts, and nothing served;
//! - `cantrip-infer time --model DIR [--bucket 128] [--seconds 20] [engine options]`: the network alone on one stream,
//!   one session of the bucket at its `--batch` size, every slot a reference position: rows a second;
//! - `cantrip-infer graph --model DIR [--bucket 128] --out FILE [engine options]`: the bucket's graph as the engine runs
//!   it (narrowed inputs; at fp16 halved, and its attention fused), for a profiler or a viewer.
//!
//! The engine options: `--precision`, `--batch`, `--shapes` (smaller shapes a bucket may also run a partial batch at,
//! each kept only where it computes every row bitwise as the full shape: `gpu::Options::shapes`), `--flush-ms`,
//! `--pipelines`, `--device`; `--fp16-graph` (at fp16, what runs in half besides the matrix products, each with the
//! ones before: `products`, `attention`, `ffn` (the default), `residual` (opt-in: it fails the reduced bar on the
//! templates line's gen2); `gpu::Fp16Graph`), `--unfused-attention` (`--fp16-graph products`), `--gelu` (a fused GELU's kernel, `gelu` (the default) or `biasgelu`;
//! `gpu::GeluKernel`), `--inputs-tf32` (at fp16, the input stage's fp32 products in TF32: `Options::inputs_tf32`),
//! `--no-cuda-graphs` (each batch launched node by node instead of replayed as a CUDA graph).
//!
//! onnxruntime-gpu is loaded at run time: `ORT_DYLIB_PATH` (or `--ort-dylib`) names its `onnxruntime.dll`, and
//! `CANTRIP_CUDA_LIBS` (or `--cuda-libs`) the directory of CUDA 12's and cuDNN 9's libraries.

use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cantrip_ai::eval::{Evaluator, FeatureBatch, reference};
use cantrip_ai::gpu::{self, Engine, Options, Precision, Rows};
use cantrip_ai::remote::{PROTOCOL, read_frame, read_hello, status, write_frame};
use cantrip_ai::{CardIndex, RemoteEvaluator};
use cantrip_obs::encode::{BUCKETS, from_compact};

const USAGE: &str = "usage: cantrip-infer [serve] --model DIR [--model DIR ...] [--host H] [--port N] [--precision fp32|tf32|fp16] [--batch N|32:N,64:N,96:N,128:N] [--shapes N,N,...|T:N/N,...|none] [--flush-ms MS] [--pipelines N] [--device N] [--no-check] [--ready-file PATH] [--log-every S] [--priority high|normal] [--ort-dylib PATH] [--cuda-libs DIR]
       cantrip-infer bench --model DIR [--clients N] [--rows N|A-B] [--mix T:share,...] [--think-ms MS] [--seconds S] [engine options]
       cantrip-infer compare --model DIR --a HOST:PORT --b HOST:PORT [--shards DIR] [--limit N]
       cantrip-infer check --model DIR [--model DIR ...] [engine options]
       cantrip-infer time --model DIR [--bucket 32|64|96|128] [--seconds S] [engine options]
       cantrip-infer graph --model DIR [--bucket 32|64|96|128] --out FILE [engine options]
engine options: [--precision fp32|tf32|fp16] [--batch N|32:N,64:N,96:N,128:N] [--shapes N,N,...|T:N/N,...|none] [--flush-ms MS] [--pipelines N] [--device N] [--fp16-graph products|attention|ffn|residual] [--gelu gelu|biasgelu] [--unfused-attention] [--inputs-tf32] [--no-cuda-graphs]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = go(args) {
        eprintln!("cantrip-infer: {e}\n{USAGE}");
        std::process::exit(2);
    }
}

#[derive(Default)]
struct Args {
    models: Vec<PathBuf>,
    host: String,
    port: u16,
    options: Options,
    check: bool,
    ready: Option<PathBuf>,
    log_every: f64,
    high: bool,
    clients: usize,
    rows: (usize, usize),
    mix: Option<Vec<(usize, f64)>>,
    think_ms: f64,
    seconds: f64,
    a: String,
    b: String,
    shards: Option<PathBuf>,
    limit: usize,
    bucket: usize,
    out: Option<PathBuf>,
}

fn go(mut argv: Vec<String>) -> Result<(), String> {
    let mode = match argv.first().map(String::as_str) {
        Some("serve" | "bench" | "compare" | "check" | "time" | "graph") => argv.remove(0),
        _ => "serve".to_string(),
    };
    let mut a = Args {
        host: "127.0.0.1".into(),
        port: 7710,
        check: true,
        log_every: 60.0,
        high: true,
        clients: 64,
        rows: (8, 8),
        seconds: 20.0,
        limit: 4096,
        bucket: 128,
        ..Args::default()
    };
    let mut i = 0;
    while i < argv.len() {
        let flag = argv[i].as_str();
        if flag == "--no-check" {
            a.check = false;
            i += 1;
            continue;
        }
        if flag == "--unfused-attention" {
            a.options.fp16 = gpu::Fp16Graph::Products;
            i += 1;
            continue;
        }
        if flag == "--inputs-tf32" {
            a.options.inputs_tf32 = true;
            i += 1;
            continue;
        }
        if flag == "--no-cuda-graphs" {
            a.options.cuda_graphs = false;
            i += 1;
            continue;
        }
        let v = argv.get(i + 1).cloned().ok_or(format!("{flag} needs a value"))?;
        let num = |v: &str| v.parse::<f64>().map_err(|_| format!("{flag} takes a number, not {v}"));
        match flag {
            "--model" => a.models.push(PathBuf::from(v)),
            "--host" => a.host = v,
            "--port" => a.port = num(&v)? as u16,
            "--precision" => {
                a.options.precision = Precision::parse(&v).ok_or(format!("no precision {v} (fp32, tf32, fp16)"))?
            }
            "--batch" => a.options.batch = gpu::parse_batch(&v)?,
            "--shapes" => a.options.shapes = gpu::parse_shapes(&v)?,
            "--flush-ms" => a.options.flush = Duration::from_secs_f64(num(&v)? / 1000.0),
            "--pipelines" => a.options.pipelines = (num(&v)? as usize).max(1),
            "--device" => a.options.device = num(&v)? as i32,
            "--ready-file" => a.ready = Some(PathBuf::from(v)),
            "--log-every" => a.log_every = num(&v)?,
            "--priority" => a.high = v == "high",
            // SAFETY (both): set before onnxruntime is loaded, while this is the only thread.
            "--ort-dylib" => unsafe { std::env::set_var("ORT_DYLIB_PATH", v) },
            "--cuda-libs" => unsafe { std::env::set_var("CANTRIP_CUDA_LIBS", v) },
            "--clients" => a.clients = (num(&v)? as usize).max(1),
            "--rows" => a.rows = parse_rows(&v)?,
            "--mix" => a.mix = Some(parse_mix(&v)?),
            "--think-ms" => a.think_ms = num(&v)?,
            "--seconds" => a.seconds = num(&v)?,
            "--a" => a.a = v,
            "--b" => a.b = v,
            "--shards" => a.shards = Some(PathBuf::from(v)),
            "--limit" => a.limit = num(&v)? as usize,
            "--bucket" => a.bucket = num(&v)? as usize,
            "--out" => a.out = Some(PathBuf::from(v)),
            "--fp16-graph" => {
                a.options.fp16 = gpu::Fp16Graph::parse(&v).ok_or(format!(
                    "no fp16 graph {v} (products, attention, ffn, residual)"
                ))?
            }
            "--gelu" => a.options.gelu = gpu::GeluKernel::parse(&v).ok_or(format!("no GELU kernel {v} (gelu, biasgelu)"))?,
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    if a.models.is_empty() {
        return Err("no --model".into());
    }
    match mode.as_str() {
        "compare" => compare(&a),
        "bench" => bench(&a),
        "check" => start(&a).map(|_| ()),
        "time" => time(&a),
        "graph" => graph(&a),
        _ => serve(&a),
    }
}

fn start(a: &Args) -> Result<Arc<Engine>, String> {
    let started = Instant::now();
    let dirs: Vec<&Path> = a.models.iter().map(PathBuf::as_path).collect();
    let engine = Arc::new(Engine::start(&dirs, a.options.clone()).map_err(|e| e.to_string())?);
    for (k, m) in engine.models().iter().enumerate() {
        eprintln!(
            "infer: {} ({}), graph: {}; shapes {}",
            m.name,
            m.dir.display(),
            m.rewritten_summary(),
            m.shapes_summary()
        );
        if a.check {
            let c = gpu::check(&engine, k).map_err(|e| format!("{}: {e}", m.dir.display()))?;
            eprintln!(
                "infer: {} references {} at {}: {c:?}",
                m.name,
                if c.pass { "pass" } else { "FAIL" },
                a.options.precision.name()
            );
            if !c.pass {
                return Err(format!("{}: the served model is not its references", m.dir.display()));
            }
        }
    }
    eprintln!(
        "infer: {} model(s) on cuda:{}, batch {:?}, flush {:?}, {} pipelines, {}; ready in {:.1} s",
        engine.models().len(),
        a.options.device,
        a.options.batch,
        a.options.flush,
        a.options.pipelines,
        a.options.precision.name(),
        started.elapsed().as_secs_f64()
    );
    Ok(engine)
}

fn serve(a: &Args) -> Result<(), String> {
    if a.high {
        raise_priority();
    }
    let engine = start(a)?;
    let listener = TcpListener::bind((a.host.as_str(), a.port)).map_err(|e| format!("{}:{}: {e}", a.host, a.port))?;
    let local = listener.local_addr().map_err(|e| e.to_string())?;
    eprintln!("infer: listening on {local}");
    if let Some(path) = &a.ready {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, local.to_string()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    }
    if a.log_every > 0.0 {
        let engine = engine.clone();
        let every = Duration::from_secs_f64(a.log_every);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(every);
                eprintln!("{}", engine.summary());
            }
        });
    }
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let engine = engine.clone();
        let _ = std::thread::Builder::new()
            .name("infer-conn".into())
            .spawn(move || {
                if let Err(e) = connection(&engine, stream) {
                    eprintln!("infer: a connection ended: {e}");
                }
            });
    }
    Ok(())
}

fn connection(engine: &Engine, stream: TcpStream) -> Result<(), String> {
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    let mut reader = BufReader::with_capacity(1 << 20, stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = BufWriter::with_capacity(1 << 20, stream);
    let hello = read_frame(&mut reader).map_err(|e| e.to_string())?;
    let refuse = |w: &mut BufWriter<TcpStream>, why: String| {
        let _ = write_frame(w, &status(false, &why));
        Err(why)
    };
    let (protocol, schema, hash, identity, name) = match read_hello(&hello) {
        Ok(h) => h,
        Err(e) => return refuse(&mut writer, e.to_string()),
    };
    let model = if protocol != PROTOCOL {
        return refuse(&mut writer, format!("protocol {protocol}; this server speaks {PROTOCOL}"));
    } else if let Some(k) = engine.model(identity) {
        k
    } else {
        let have: Vec<String> = engine
            .models()
            .iter()
            .map(|m| format!("{} ({})", m.name, m.dir.display()))
            .collect();
        return refuse(
            &mut writer,
            format!("no model {name} with that manifest here; this server has {}", have.join(", ")),
        );
    };
    let m = &engine.models()[model];
    if schema != m.schema || hash != m.encoder_hash {
        return refuse(&mut writer, format!("encoding ({schema}, {hash:016x}); model's is ({}, {:016x})", m.schema, m.encoder_hash));
    }
    write_frame(
        &mut writer,
        &status(
            true,
            &format!(
                "{} on cuda:{} (cantrip-infer, onnxruntime CUDA, {}); input {:?}",
                m.name,
                engine.options().device,
                engine.options().precision.name(),
                m.input_compatibility
            ),
        ),
    )
    .map_err(|e| e.to_string())?;
    let _client = engine.client();
    loop {
        let payload = match read_frame(&mut reader) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        let reply = match Rows::from_wire(&payload).and_then(|rows| {
            let legal = rows.legal.clone();
            let out = engine.evaluate(model, rows)?;
            Ok(out.reply(&legal))
        }) {
            Ok(r) => r,
            Err(e) => return refuse(&mut writer, e.to_string()),
        };
        write_frame(&mut writer, &reply).map_err(|e| e.to_string())?;
    }
}

/// `N` or `A-B` rows a request (`A-B`: each request draws its size uniformly).
fn parse_rows(v: &str) -> Result<(usize, usize), String> {
    let n = |x: &str| x.parse::<usize>().map_err(|_| format!("--rows takes N or A-B, not {v}"));
    let (lo, hi) = match v.split_once('-') {
        Some((a, b)) => (n(a)?, n(b)?),
        None => (n(v)?, n(v)?),
    };
    if lo == 0 || hi < lo {
        return Err(format!("--rows takes N or A-B with 0 < A <= B, not {v}"));
    }
    Ok((lo, hi))
}

/// `T:share,...`: the share of a request's rows drawn from each bucket's references.
fn parse_mix(v: &str) -> Result<Vec<(usize, f64)>, String> {
    let mut out = Vec::new();
    for part in v.split(',') {
        let (t, w) = part.split_once(':').ok_or(format!("--mix takes T:share,..., not {v}"))?;
        let t: usize = t.parse().map_err(|_| format!("--mix: no bucket {t}"))?;
        if !gpu::GRAPH_BUCKETS.contains(&t) {
            return Err(format!("--mix: no bucket of {t} tokens"));
        }
        let w: f64 = w.parse().map_err(|_| format!("--mix: no share {w}"))?;
        out.push((t, w));
    }
    Ok(out)
}

/// A client's own random stream (splitmix64): the bench's draws, the same every run.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// The engine alone: `--clients` threads, each sending requests of `--rows` reference positions for `--seconds`. By
/// default a client cycles through the references and asks again at once (the GPU the limit); `--mix` draws each row
/// from a bucket's references at the given shares, and `--think-ms` has a client spend about that long (uniformly half
/// to one and a half times it) between its requests, as a self-play lane searches between evaluations.
fn bench(a: &Args) -> Result<(), String> {
    if a.high {
        raise_priority();
    }
    let engine = start(a)?;
    let dir = &a.models[0];
    let r = reference::Reference::load(&dir.join("reference")).map_err(|e| e.to_string())?;
    let own = CardIndex::read(&dir.join("cards.json"))?;
    // The references by the bucket the engine runs them at (the smallest graph bucket holding their tokens).
    let nb = gpu::GRAPH_BUCKETS.len();
    let mut by_bucket: Vec<Vec<usize>> = vec![Vec::new(); nb];
    for (i, p) in r.positions.iter().enumerate() {
        let k = gpu::GRAPH_BUCKETS.iter().position(|&t| t >= p.len.max(1)).unwrap_or(nb - 1);
        by_bucket[k].push(i);
    }
    let mix: Option<Vec<(usize, f64)>> = match &a.mix {
        None => None,
        Some(m) => {
            let total: f64 = m.iter().map(|&(_, w)| w).sum();
            let mut acc = 0.0;
            let mut out = Vec::new();
            for &(t, w) in m {
                let k = gpu::GRAPH_BUCKETS.iter().position(|&b| b == t).unwrap_or(0);
                if by_bucket[k].is_empty() {
                    return Err(format!("{}: no reference runs at T{t}", dir.display()));
                }
                acc += w / total;
                out.push((k, acc));
            }
            Some(out)
        }
    };
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let latencies = Arc::new(std::sync::Mutex::new(Vec::<(f64, usize)>::new()));
    let positions = Arc::new(r.positions);
    let started = Instant::now();
    std::thread::scope(|s| {
        for c in 0..a.clients {
            let (engine, stop, latencies, positions) = (engine.clone(), stop.clone(), latencies.clone(), positions.clone());
            let (own, mix, by_bucket) = (&own, &mix, &by_bucket);
            let (rows, think) = (a.rows, a.think_ms / 1e3);
            s.spawn(move || {
                let e = gpu::GpuEvaluator::new(engine, 0, own).expect("an evaluator");
                let mut draw = Draw(c as u64 * 0x5851_f42d_4c95_7f2d + 1);
                let mut at = c * 7919 % positions.len();
                let mut mine = Vec::new();
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let n = rows.0 + draw.below(rows.1 - rows.0 + 1);
                    let idx: Vec<usize> = match mix {
                        None => {
                            let idx = (0..n).map(|k| (at + k) % positions.len()).collect();
                            at = (at + n) % positions.len();
                            idx
                        }
                        Some(m) => (0..n)
                            .map(|_| {
                                let u = draw.unit();
                                let k = m.iter().find(|&&(_, acc)| u < acc).map_or(m[m.len() - 1].0, |&(k, _)| k);
                                by_bucket[k][draw.below(by_bucket[k].len())]
                            })
                            .collect(),
                    };
                    let es: Vec<_> = idx.iter().map(|&i| &positions[i]).collect();
                    let t0 = Instant::now();
                    e.evaluate_positions(&es).expect("an evaluation");
                    mine.push((t0.elapsed().as_secs_f64(), n));
                    if think > 0.0 {
                        std::thread::sleep(Duration::from_secs_f64(think * (0.5 + draw.unit())));
                    }
                }
                latencies.lock().unwrap().extend(mine);
            });
        }
        std::thread::sleep(Duration::from_secs_f64(a.seconds));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let seconds = started.elapsed().as_secs_f64();
    let mut l = latencies.lock().unwrap().clone();
    let rows: usize = l.iter().map(|&(_, n)| n).sum();
    l.sort_by(|x, y| x.0.total_cmp(&y.0));
    let pct = |p: f64| l.get(((l.len() as f64 - 1.0) * p) as usize).map_or(0.0, |x| x.0) * 1e3;
    let st = engine.stats();
    println!(
        "bench: {} clients x {}-{} rows, think {} ms: {} requests, {:.0} rows a second, {:.0}% of the padded rows real; \
         latency ms p50 {:.2} p90 {:.2} p99 {:.2}",
        a.clients,
        a.rows.0,
        a.rows.1,
        a.think_ms,
        l.len(),
        rows as f64 / seconds,
        100.0 * st.rows as f64 / st.padded.max(1) as f64,
        pct(0.5),
        pct(0.9),
        pct(0.99)
    );
    println!("{}", engine.summary());
    Ok(())
}

/// The bucket's file in the first model's manifest.
fn bucket_file(a: &Args) -> Result<PathBuf, String> {
    let dir = &a.models[0];
    let text = std::fs::read(dir.join("manifest.json")).map_err(|e| format!("{}: {e}", dir.display()))?;
    let manifest: serde_json::Value = serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = manifest["buckets"][a.bucket.to_string()]
        .as_str()
        .ok_or(format!("{}: no graph for {} tokens", dir.display(), a.bucket))?;
    Ok(dir.join(file))
}

/// The network alone on one stream (`gpu::time_bucket`).
fn time(a: &Args) -> Result<(), String> {
    let k = gpu::GRAPH_BUCKETS.iter().position(|&b| b == a.bucket).ok_or(format!("no bucket of {} tokens", a.bucket))?;
    let batch = a.options.batch[k];
    let timing = gpu::time_bucket(&a.models[0], &a.options, a.bucket, batch, a.seconds).map_err(|e| e.to_string())?;
    let graph = if a.options.precision == Precision::Fp16 {
        format!(" {} ({}): {}", a.options.fp16.name(), a.options.gelu.name(), timing.rewritten.summary())
    } else {
        String::new()
    };
    println!(
        "time: {} T{} batch {batch} {}{graph}: {:.0} rows a second; a slot of padding alone {}; a batch: assembled \
         {:.2} ms, copied in {:.2} ms, network {:.2} ms, copied out {:.2} ms",
        a.models[0].display(),
        a.bucket,
        a.options.precision.name(),
        timing.rows_per_second,
        if timing.padding_finite { "finite" } else { "NOT FINITE" },
        1e3 * timing.assemble,
        1e3 * timing.run.copy_in,
        1e3 * timing.run.network,
        1e3 * timing.run.copy_out,
    );
    Ok(())
}

/// The bucket's graph as the engine runs it, written to `--out`.
fn graph(a: &Args) -> Result<(), String> {
    let out = a.out.as_ref().ok_or("graph needs --out")?;
    let (bytes, rewritten) = gpu::graph_bytes(&bucket_file(a)?, &a.options).map_err(|e| e.to_string())?;
    std::fs::write(out, bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!("infer: wrote {} ({})", out.display(), rewritten.summary());
    Ok(())
}

/// The same positions through the servers at `--a` and `--b`.
fn compare(a: &Args) -> Result<(), String> {
    let dir = &a.models[0];
    let own = CardIndex::read(&dir.join("cards.json"))?;
    let r = reference::Reference::load(&dir.join("reference")).map_err(|e| e.to_string())?;
    let mut sets: Vec<(String, CardIndex, Vec<cantrip_obs::encode::Encoded>)> =
        vec![("references".into(), own, r.positions)];
    if let Some(shards) = &a.shards {
        let mut positions = Vec::new();
        let mut index = None;
        for path in cantrip_ai::shard::shards(shards).map_err(|e| e.to_string())? {
            // A game whose record does not read back (a non-finite root value is written as null) ends its shard here.
            let mut reader = cantrip_ai::shard::ShardReader::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            index.get_or_insert_with(|| CardIndex::from_names(reader.header.cards.clone()));
            while let Ok(Some(g)) = reader.game() {
                for s in g.samples {
                    if positions.len() >= a.limit {
                        break;
                    }
                    if let Some(e) = from_compact(&s.position) {
                        positions.push(e);
                    }
                }
            }
            if positions.len() >= a.limit {
                break;
            }
        }
        if let Some(index) = index {
            sets.push((format!("shards ({})", shards.display()), index, positions));
        }
    }
    for (name, table, positions) in &sets {
        let ea = RemoteEvaluator::connect(&a.a, dir, table).map_err(|e| e.to_string())?;
        let eb = RemoteEvaluator::connect(&a.b, dir, table).map_err(|e| e.to_string())?;
        let (mut value, mut swing, mut dead, mut logits, mut probability) = (0f32, 0f32, 0f32, 0f32, 0f32);
        let (mut same, mut counted, mut bitwise) = (0usize, 0usize, 0usize);
        let mut sums = (0f64, 0f64);
        for bucket in BUCKETS {
            let rows: Vec<usize> = (0..positions.len()).filter(|&i| positions[i].bucket == bucket).collect();
            for chunk in rows.chunks(64) {
                let es: Vec<_> = chunk.iter().map(|&i| &positions[i]).collect();
                let batch = FeatureBatch::new(&es);
                let (oa, ob) = (
                    ea.evaluate(&batch).map_err(|e| e.to_string())?,
                    eb.evaluate(&batch).map_err(|e| e.to_string())?,
                );
                for (j, &i) in chunk.iter().enumerate() {
                    let p = &positions[i];
                    let dv = (oa.value[j] - ob.value[j]).abs();
                    value = value.max(dv);
                    sums.0 += dv as f64;
                    swing = swing.max((oa.aux[j][0] - ob.aux[j][0]).abs());
                    dead = dead.max((oa.aux[j][1] - ob.aux[j][1]).abs());
                    let legal: Vec<usize> = (0..p.options).filter(|&k| p.legal[p.first_option + k]).collect();
                    let ga: Vec<f32> = legal.iter().map(|&k| oa.policy[j][k]).collect();
                    let gb: Vec<f32> = legal.iter().map(|&k| ob.policy[j][k]).collect();
                    let mut row = 0f32;
                    for (x, y) in ga.iter().zip(&gb) {
                        row = row.max((x - y).abs());
                    }
                    logits = logits.max(row);
                    sums.1 += row as f64;
                    for (x, y) in gpu::softmax(&ga).iter().zip(gpu::softmax(&gb)) {
                        probability = probability.max((x - y).abs() as f32);
                    }
                    let best = |x: &[f32]| (0..x.len()).max_by(|&u, &v| x[u].total_cmp(&x[v]).then(v.cmp(&u)));
                    same += (best(&ga) == best(&gb)) as usize;
                    bitwise += (oa.value[j].to_bits() == ob.value[j].to_bits()
                        && ga.iter().zip(&gb).all(|(x, y)| x.to_bits() == y.to_bits()))
                        as usize;
                    counted += 1;
                }
            }
        }
        println!(
            "compare {name}: {counted} positions, {} vs {}: value max {value:.5} mean {:.6}; legal logits max {logits:.5} mean of row max {:.6}; probability max {probability:.5}; swing max {swing:.4}; dead max {dead:.5}; same best {same}/{counted}; bit-identical {bitwise}/{counted}",
            ea.server(),
            eb.server(),
            sums.0 / counted.max(1) as f64,
            sums.1 / counted.max(1) as f64,
        );
    }
    Ok(())
}

/// Above the lanes that feed it, where the OS allows it (as serve.py raises itself).
fn raise_priority() {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn SetPriorityClass(process: *mut std::ffi::c_void, class: u32) -> i32;
        }
        // SAFETY: plain Win32 calls on this process's own pseudo-handle.
        if unsafe { SetPriorityClass(GetCurrentProcess(), 0x80) } == 0 {
            eprintln!("infer: running at normal priority");
        }
    }
}
