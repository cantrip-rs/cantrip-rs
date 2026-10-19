//! The phase-0 spike (docs/ai-player-plan.md): the exported spike model through `ort` (CPU, and CUDA with
//! `--features cuda`) and `tract`, parity against PyTorch's reference outputs and positions a second per bucket.
//!
//!   cargo run --release [-- --out DIR] [--seconds S] [--only ort|tract] [--tokens T] [--batch B]
//!
//! DIR is what `ai/spike/export.py` wrote (default `../out`).

use std::path::{Path, PathBuf};
use std::time::Instant;

use ort::environment::Environment;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::TensorRef;
use tract_onnx::prelude::*;

const TOKENS: [usize; 3] = [32, 64, 128];
const BATCHES: [usize; 4] = [1, 16, 64, 512];
const VALUE_TOLERANCE: f32 = 1e-3;
const SAME_ARGMAX: f64 = 0.995;

type Error = Box<dyn std::error::Error>;

/// A bucket's reference positions and PyTorch's outputs for them.
struct Reference {
    tokens: usize,
    features: usize,
    n: usize,
    x: Vec<f32>,
    kind: Vec<i64>,
    present: Vec<bool>,
    option: Vec<bool>,
    value: Vec<f32>,
    policy: Vec<f32>,
}

impl Reference {
    fn load(dir: &Path, tokens: usize) -> Result<Reference, Error> {
        let dir = dir.join(format!("ref-t{tokens}"));
        let bytes = |name: &str| std::fs::read(dir.join(name));
        let f32s = |b: Vec<u8>| b.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>();
        let value = f32s(bytes("value.f32")?);
        let n = value.len();
        let x = f32s(bytes("features.f32")?);
        Ok(Reference {
            tokens,
            features: x.len() / (n * tokens),
            n,
            x,
            kind: bytes("kind.i64")?.chunks_exact(8).map(|c| i64::from_le_bytes(c.try_into().unwrap())).collect(),
            present: bytes("present.u8")?.into_iter().map(|b| b != 0).collect(),
            option: bytes("option.u8")?.into_iter().map(|b| b != 0).collect(),
            value,
            policy: f32s(bytes("policy.f32")?),
        })
    }

    /// Positions `at..at + batch`, wrapping around.
    fn batch(&self, at: usize, batch: usize) -> Batch {
        let rows: Vec<usize> = (at..at + batch).map(|i| i % self.n).collect();
        let take = |width: usize| rows.iter().flat_map(move |&r| r * width..(r + 1) * width);
        let (t, f) = (self.tokens, self.features);
        Batch {
            rows: rows.clone(),
            x: take(t * f).map(|i| self.x[i]).collect(),
            kind: take(t).map(|i| self.kind[i]).collect(),
            present: take(t).map(|i| self.present[i]).collect(),
            option: take(t).map(|i| self.option[i]).collect(),
        }
    }
}

struct Batch {
    rows: Vec<usize>,
    x: Vec<f32>,
    kind: Vec<i64>,
    present: Vec<bool>,
    option: Vec<bool>,
}

/// One inference backend: a batch in, value [B] and policy logits [B*T] out.
trait Backend {
    fn run(&mut self, b: &Batch, tokens: usize, features: usize) -> Result<(Vec<f32>, Vec<f32>), Error>;
}

struct Ort(Session);

impl Ort {
    fn new(env: &Environment, path: &Path, threads: usize, gpu: bool) -> Result<Ort, Error> {
        let mut builder = Session::builder(env)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| e.to_string())?
            .with_intra_threads(threads)
            .map_err(|e| e.to_string())?
            .with_inter_threads(1)
            .map_err(|e| e.to_string())?;
        if gpu {
            builder = cuda(builder)?;
        }
        Ok(Ort(builder.commit_from_file(path)?))
    }
}

#[cfg(feature = "cuda")]
fn cuda(builder: ort::session::builder::SessionBuilder) -> Result<ort::session::builder::SessionBuilder, Error> {
    Ok(builder.with_execution_providers([ort::ep::CUDA::default().build().error_on_failure()]).map_err(|e| e.to_string())?)
}

#[cfg(not(feature = "cuda"))]
fn cuda(_: ort::session::builder::SessionBuilder) -> Result<ort::session::builder::SessionBuilder, Error> {
    Err("built without the cuda feature".into())
}

impl Backend for Ort {
    fn run(&mut self, b: &Batch, t: usize, f: usize) -> Result<(Vec<f32>, Vec<f32>), Error> {
        let n = b.rows.len();
        let out = self.0.run(ort::inputs![
            "features" => TensorRef::from_array_view(([n, t, f], &b.x[..]))?,
            "kind" => TensorRef::from_array_view(([n, t], &b.kind[..]))?,
            "present" => TensorRef::from_array_view(([n, t], &b.present[..]))?,
            "option" => TensorRef::from_array_view(([n, t], &b.option[..]))?,
        ])?;
        let value = out["value"].try_extract_tensor::<f32>()?.1.to_vec();
        let policy = out["policy"].try_extract_tensor::<f32>()?.1.to_vec();
        Ok((value, policy))
    }
}

/// tract wants concrete shapes for its optimiser: one plan per (T, B) bucket.
struct Tract(Arc<TypedRunnableModel>);

impl Tract {
    fn new(path: &Path, n: usize, t: usize, f: usize) -> Result<Tract, Error> {
        let model = tract_onnx::onnx()
            .model_for_path(path)?
            .with_input_fact(0, f32::fact([n, t, f]).into())?
            .with_input_fact(1, i64::fact([n, t]).into())?
            .with_input_fact(2, bool::fact([n, t]).into())?
            .with_input_fact(3, bool::fact([n, t]).into())?
            .into_optimized()?
            .into_runnable()?;
        Ok(Tract(model))
    }
}

impl Backend for Tract {
    fn run(&mut self, b: &Batch, t: usize, f: usize) -> Result<(Vec<f32>, Vec<f32>), Error> {
        let n = b.rows.len();
        let out = self.0.run(tvec!(
            Tensor::from_shape(&[n, t, f], &b.x)?.into(),
            Tensor::from_shape(&[n, t], &b.kind)?.into(),
            Tensor::from_shape(&[n, t], &b.present)?.into(),
            Tensor::from_shape(&[n, t], &b.option)?.into(),
        ))?;
        Ok((out[0].try_as_plain_ram()?.as_slice::<f32>()?.to_vec(), out[1].try_as_plain_ram()?.as_slice::<f32>()?.to_vec()))
    }
}

fn argmax(row: &[f32]) -> usize {
    row.iter().enumerate().fold(0, |best, (i, &v)| if v > row[best] { i } else { best })
}

/// Runs every reference position through the backend in batches of `batch`; the worst value difference and the
/// share of positions whose policy argmax matches PyTorch's.
fn parity(backend: &mut dyn Backend, r: &Reference, batch: usize) -> Result<(f32, f64), Error> {
    let (mut worst, mut same) = (0f32, 0usize);
    let mut at = 0;
    while at < r.n {
        let b = r.batch(at, batch.min(r.n - at));
        let (value, policy) = backend.run(&b, r.tokens, r.features)?;
        for (k, &row) in b.rows.iter().enumerate() {
            worst = worst.max((value[k] - r.value[row]).abs());
            let t = r.tokens;
            if argmax(&policy[k * t..(k + 1) * t]) == argmax(&r.policy[row * t..(row + 1) * t]) {
                same += 1;
            }
        }
        at += b.rows.len();
    }
    Ok((worst, same as f64 / r.n as f64))
}

fn throughput(backend: &mut dyn Backend, r: &Reference, batch: usize, seconds: f64) -> Result<f64, Error> {
    let b = r.batch(0, batch);
    backend.run(&b, r.tokens, r.features)?;
    let (start, mut runs) = (Instant::now(), 0usize);
    while start.elapsed().as_secs_f64() < seconds {
        backend.run(&b, r.tokens, r.features)?;
        runs += 1;
    }
    Ok((runs * batch) as f64 / start.elapsed().as_secs_f64())
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok())
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    let out: PathBuf = arg(&args, "--out").unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../out"));
    let seconds: f64 = arg(&args, "--seconds").unwrap_or(1.0);
    let only: Option<String> = arg(&args, "--only");
    let tokens: Option<usize> = arg(&args, "--tokens");
    let batch: Option<usize> = arg(&args, "--batch");
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let env: &'static Environment = Box::leak(Box::new(ort::init().build()?));
    let references = TOKENS.iter().map(|&t| Reference::load(&out, t)).collect::<Result<Vec<_>, _>>()?;

    let mut targets: Vec<(String, Box<dyn Fn(&Path, &Reference, usize) -> Result<Box<dyn Backend>, Error>>)> = Vec::new();
    if only.as_deref().is_none_or(|o| o == "ort") {
        targets.push(("ort cpu, 1 thread".into(), Box::new(move |p, _, _| Ok(Box::new(Ort::new(env, p, 1, false)?)))));
        targets.push((format!("ort cpu, {cores} threads"), Box::new(move |p, _, _| Ok(Box::new(Ort::new(env, p, cores, false)?)))));
        if cfg!(feature = "cuda") {
            targets.push(("ort cuda".into(), Box::new(move |p, _, _| Ok(Box::new(Ort::new(env, p, 1, true)?)))));
        }
    }
    if only.as_deref().is_none_or(|o| o == "tract") {
        targets.push((
            "tract, 1 thread".into(),
            Box::new(|p, r, b| Ok(Box::new(Tract::new(p, b, r.tokens, r.features)?))),
        ));
    }

    println!("Parity against PyTorch (value within {VALUE_TOLERANCE}, same argmax on {:.1}%+), positions/s:", SAME_ARGMAX * 100.0);
    let mut failed = false;
    for (label, make) in &targets {
        for r in references.iter().filter(|r| tokens.is_none_or(|t| t == r.tokens)) {
            let path = out.join(format!("spike-t{}.onnx", r.tokens));
            for batch in BATCHES.into_iter().filter(|&b| batch.is_none_or(|x| x == b)) {
                let result = make(&path, r, batch).and_then(|mut backend| {
                    let (worst, same) = parity(backend.as_mut(), r, batch)?;
                    Ok((worst, same, throughput(backend.as_mut(), r, batch, seconds)?))
                });
                match result {
                    Ok((worst, same, rate)) => {
                        let ok = worst <= VALUE_TOLERANCE && same >= SAME_ARGMAX;
                        failed |= !ok;
                        println!(
                            "  {label:22} T{:<3} B{batch:<3}  value {worst:.1e}  argmax {:6.2}%  {}  {rate:9.0}/s",
                            r.tokens,
                            same * 100.0,
                            if ok { "ok  " } else { "FAIL" }
                        );
                    }
                    Err(e) => {
                        failed = true;
                        println!("  {label:22} T{:<3} B{batch:<3}  error: {e}", r.tokens);
                    }
                }
            }
        }
    }
    if failed {
        return Err("a backend failed parity or did not run".into());
    }
    Ok(())
}
