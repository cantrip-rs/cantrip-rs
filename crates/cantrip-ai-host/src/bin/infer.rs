//! `cantrip-infer`: the native inference server (docs/ai-host.md, "The native inference server"). It serves
//! `ai/cantrip_ai/serve.py`'s protocol (`cantrip_ai::remote`) from `cantrip_ai::gpu`'s engine: onnxruntime's CUDA
//! provider, every connection's rows batched together, no Python on the request path. `--backend remote --server` and
//! BotLab's `--ai-server` use it as they use serve.py.
//!
//! - `cantrip-infer serve --model DIR [--model DIR ...] [--host 127.0.0.1] [--port 7710] [--precision fp32|tf32|fp16]
//!   [--batch N | 32:N,64:N,128:N] [--flush-ms 2] [--pipelines 2] [--device 0] [--no-check] [--ready-file PATH]
//!   [--log-every 60] [--priority high|normal]` (`serve` may be left out);
//! - `cantrip-infer bench --model DIR [--clients 64] [--rows 8] [--seconds 20] [engine options]`: the engine alone, fed
//!   the model's reference positions by `--clients` threads of `--rows` rows a request: rows a second and latency;
//! - `cantrip-infer compare --model DIR --a HOST:PORT --b HOST:PORT [--shards DIR] [--limit N]`: the same positions (the
//!   references, and a shard directory's) through two servers: how far apart their outputs are.
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
use cantrip_obs::encode::{BUCKETS, SCHEMA_VERSION, encoder_hash, from_compact};

const USAGE: &str = "usage: cantrip-infer [serve] --model DIR [--model DIR ...] [--host H] [--port N] [--precision fp32|tf32|fp16] [--batch N|32:N,64:N,128:N] [--flush-ms MS] [--pipelines N] [--device N] [--no-check] [--ready-file PATH] [--log-every S] [--priority high|normal] [--ort-dylib PATH] [--cuda-libs DIR]
       cantrip-infer bench --model DIR [--clients N] [--rows N] [--seconds S] [engine options]
       cantrip-infer compare --model DIR --a HOST:PORT --b HOST:PORT [--shards DIR] [--limit N]";

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
    rows: usize,
    seconds: f64,
    a: String,
    b: String,
    shards: Option<PathBuf>,
    limit: usize,
}

fn go(mut argv: Vec<String>) -> Result<(), String> {
    let mode = match argv.first().map(String::as_str) {
        Some("serve") | Some("bench") | Some("compare") => argv.remove(0),
        _ => "serve".to_string(),
    };
    let mut a = Args {
        host: "127.0.0.1".into(),
        port: 7710,
        check: true,
        log_every: 60.0,
        high: true,
        clients: 64,
        rows: 8,
        seconds: 20.0,
        limit: 4096,
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
            "--rows" => a.rows = (num(&v)? as usize).max(1),
            "--seconds" => a.seconds = num(&v)?,
            "--a" => a.a = v,
            "--b" => a.b = v,
            "--shards" => a.shards = Some(PathBuf::from(v)),
            "--limit" => a.limit = num(&v)? as usize,
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
        _ => serve(&a),
    }
}

fn start(a: &Args) -> Result<Arc<Engine>, String> {
    let started = Instant::now();
    let dirs: Vec<&Path> = a.models.iter().map(PathBuf::as_path).collect();
    let engine = Arc::new(Engine::start(&dirs, a.options.clone()).map_err(|e| e.to_string())?);
    for (k, m) in engine.models().iter().enumerate() {
        eprintln!("infer: {} ({})", m.name, m.dir.display());
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
    } else if schema != SCHEMA_VERSION || hash != encoder_hash() {
        return refuse(
            &mut writer,
            format!("encoding ({schema}, {hash:016x}); this server's is ({SCHEMA_VERSION}, {:016x})", encoder_hash()),
        );
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
    write_frame(
        &mut writer,
        &status(
            true,
            &format!(
                "{} on cuda:{} (cantrip-infer, onnxruntime CUDA, {})",
                m.name,
                engine.options().device,
                engine.options().precision.name()
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

/// The engine alone: `--clients` threads, each sending requests of `--rows` reference positions (cycling through
/// them), for `--seconds`.
fn bench(a: &Args) -> Result<(), String> {
    if a.high {
        raise_priority();
    }
    let engine = start(a)?;
    let dir = &a.models[0];
    let r = reference::Reference::load(&dir.join("reference")).map_err(|e| e.to_string())?;
    let own = CardIndex::read(&dir.join("cards.json"))?;
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let latencies = Arc::new(std::sync::Mutex::new(Vec::<f64>::new()));
    let positions = Arc::new(r.positions);
    let started = Instant::now();
    std::thread::scope(|s| {
        for c in 0..a.clients {
            let (engine, stop, latencies, positions) = (engine.clone(), stop.clone(), latencies.clone(), positions.clone());
            let own = &own;
            let rows = a.rows;
            s.spawn(move || {
                let e = gpu::GpuEvaluator::new(engine, 0, own).expect("an evaluator");
                let mut at = c * 7919 % positions.len();
                let mut mine = Vec::new();
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let es: Vec<_> = (0..rows).map(|k| &positions[(at + k) % positions.len()]).collect();
                    at = (at + rows) % positions.len();
                    let t0 = Instant::now();
                    e.evaluate_positions(&es).expect("an evaluation");
                    mine.push(t0.elapsed().as_secs_f64());
                }
                latencies.lock().unwrap().extend(mine);
            });
        }
        std::thread::sleep(Duration::from_secs_f64(a.seconds));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let seconds = started.elapsed().as_secs_f64();
    let mut l = latencies.lock().unwrap().clone();
    l.sort_by(f64::total_cmp);
    let pct = |p: f64| l.get(((l.len() as f64 - 1.0) * p) as usize).copied().unwrap_or(0.0) * 1e3;
    println!(
        "bench: {} clients x {} rows: {} requests, {:.0} rows a second; latency ms p50 {:.2} p90 {:.2} p99 {:.2}",
        a.clients,
        a.rows,
        l.len(),
        (l.len() * a.rows) as f64 / seconds,
        pct(0.5),
        pct(0.9),
        pct(0.99)
    );
    println!("{}", engine.summary());
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
