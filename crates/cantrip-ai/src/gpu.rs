//! Evaluating on the GPU without Python (docs/ai-host.md, "The native inference server"): onnxruntime's CUDA execution
//! provider, with the rows of every caller gathered into GPU batches per model and token bucket. One `Engine` holds the
//! models and the batching; it serves the self-play lanes of its own process (`GpuEvaluator`, `--backend gpu`) and the
//! connections of `cantrip-infer`, the native inference server, which speaks `remote`'s protocol (`--backend remote`).
//!
//! **Batching** is `ai/cantrip_ai/serve.py`'s: a row goes to the smallest bucket that holds its real tokens; a GPU call
//! takes up to the bucket's batch size of one queue and is always padded to exactly that many rows, so every call of a
//! bucket has one shape; a queue runs when it holds a full batch, when every client is waiting on the engine and no
//! batch runs (no more rows can come), or when its oldest row has waited the flush time. `pipelines` workers assemble
//! and run batches at once, each with its own sessions (and so its own CUDA stream).
//!
//! **Reproducibility.** A row's outputs never depend on what it was batched with: one shape per bucket, deterministic
//! compute, and no reduction across rows in the network.
//!
//! **Precision.** `fp32` keeps TF32 off; `tf32` allows it in matrix products; `fp16` runs every matrix product (MatMul,
//! Gemm) in half precision with its inputs cast to half and its output cast back, which is what PyTorch's autocast does
//! to them, and keeps everything else (the norms, softmax, the masking, the heads' non-linearities) in fp32. The graph is
//! rewritten in memory as it is loaded; the model directory is never written. Below fp32 a model is held to
//! `REDUCED` on its references at start, as `serve.py` holds it.
//!
//! **Libraries.** onnxruntime is loaded at run time (`ort`'s `load-dynamic`): `ORT_DYLIB_PATH` names an
//! `onnxruntime.dll` built with the CUDA provider (onnxruntime-gpu 1.24, which is built for CUDA 12 and cuDNN 9), and
//! `CANTRIP_CUDA_LIBS` a directory holding CUDA 12's and cuDNN 9's libraries (a PyTorch install's `torch/lib` has them),
//! which is put first on the library search path.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cantrip_obs::encode::{BUCKETS, CATS, Encoded, MAX_TOKENS, NO_POINTER, NUMS, PTRS, UNKNOWN_CARD};
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use prost::Message;
use tract_onnx::pb;

use crate::CardIndex;
use crate::eval::{EvalError, Evaluator, FeatureBatch, MASKED, Model, Outputs, reference};

fn err(e: impl std::fmt::Display) -> EvalError {
    EvalError(e.to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    Fp32,
    Tf32,
    Fp16,
}

impl Precision {
    pub fn parse(s: &str) -> Option<Precision> {
        match s {
            "fp32" => Some(Precision::Fp32),
            "tf32" => Some(Precision::Tf32),
            "fp16" => Some(Precision::Fp16),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Precision::Fp32 => "fp32",
            Precision::Tf32 => "tf32",
            Precision::Fp16 => "fp16",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    /// The CUDA device.
    pub device: i32,
    pub precision: Precision,
    /// Rows a GPU call is padded to, by bucket (`BUCKETS`' order).
    pub batch: [usize; 3],
    /// Batches assembled and run at once.
    pub pipelines: usize,
    /// How long a partial batch waits for more rows.
    pub flush: Duration,
    /// Each session replays its forward as a captured CUDA graph (onnxruntime's `enable_cuda_graph`): one launch a
    /// batch instead of one a node.
    pub cuda_graphs: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            device: 0,
            precision: Precision::Fp32,
            // serve.py's defaults, measured on an RTX 4080.
            batch: [256, 128, 64],
            pipelines: 2,
            flush: Duration::from_micros(2000),
            cuda_graphs: true,
        }
    }
}

/// Puts `CANTRIP_CUDA_LIBS` first on the library search path, so the CUDA provider finds CUDA's and cuDNN's
/// libraries there. Called before onnxruntime is loaded.
pub(crate) fn prepare_libraries() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if let Some(dir) = std::env::var_os("CANTRIP_CUDA_LIBS") {
            let mut paths = vec![std::path::PathBuf::from(dir)];
            if let Some(path) = std::env::var_os("PATH") {
                paths.extend(std::env::split_paths(&path));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                // SAFETY: called once, before onnxruntime (or any thread of ours that reads the environment) starts.
                unsafe { std::env::set_var("PATH", joined) };
            }
        }
    });
}

/// Rows a GPU call is padded to: `N` for every bucket, or `32:N,64:N,128:N`.
pub fn parse_batch(text: &str) -> Result<[usize; 3], String> {
    if !text.contains(':') {
        let n: usize = text.parse().map_err(|_| format!("a batch of {text}"))?;
        return Ok([n; 3]);
    }
    let mut out = [0usize; 3];
    for part in text.split(',') {
        let (t, n) = part.split_once(':').ok_or(format!("a batch of {text}"))?;
        let t: usize = t.parse().map_err(|_| format!("a batch of {text}"))?;
        let k = BUCKETS.iter().position(|&b| b == t).ok_or(format!("no bucket of {t} tokens"))?;
        out[k] = n.parse().map_err(|_| format!("a batch of {text}"))?;
    }
    if out.contains(&0) {
        return Err(format!("a batch of {text} names no size for every bucket"));
    }
    Ok(out)
}

// --- Half precision ------------------------------------------------------------------------------------------------

const FLOAT: i32 = 1;
const FLOAT16: i32 = 10;

/// The graph with every MatMul and Gemm in half precision: float initializers they read converted (a copy; other
/// readers keep the float one), other inputs cast to half before them, and their output cast back to float. Everything
/// else stays as it was.
pub fn half_matmuls(model: &mut pb::ModelProto) -> Result<(), EvalError> {
    let g = model.graph.as_mut().ok_or_else(|| err("a model without a graph"))?;
    let initializers: HashMap<String, usize> = g
        .initializer
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), i))
        .collect();
    let mut halves: HashMap<String, String> = HashMap::new();
    let mut casts: HashMap<String, String> = HashMap::new();
    let mut added = Vec::new();
    let mut nodes = Vec::with_capacity(g.node.len() * 2);
    let cast = |input: &str, output: &str, to: i32| pb::NodeProto {
        input: vec![input.to_string()],
        output: vec![output.to_string()],
        name: format!("{output}__cast"),
        op_type: "Cast".into(),
        attribute: vec![pb::AttributeProto {
            name: "to".into(),
            r#type: pb::attribute_proto::AttributeType::Int as i32,
            i: to as i64,
            ..Default::default()
        }],
        ..Default::default()
    };
    for mut n in std::mem::take(&mut g.node) {
        if !(n.domain.is_empty() && (n.op_type == "MatMul" || n.op_type == "Gemm")) {
            nodes.push(n);
            continue;
        }
        for input in n.input.iter_mut() {
            if input.is_empty() {
                continue;
            }
            if let Some(&i) = initializers.get(input.as_str()) {
                let t = &g.initializer[i];
                if t.data_type != FLOAT {
                    return Err(err(format!("{}: a {} initializer {input}", n.op_type, t.data_type)));
                }
                let name = halves
                    .entry(input.clone())
                    .or_insert_with(|| {
                        let half = format!("{input}__fp16");
                        added.push(to_half(t, &half));
                        half
                    })
                    .clone();
                *input = name;
            } else {
                let name = casts
                    .entry(input.clone())
                    .or_insert_with(|| {
                        let half = format!("{input}__fp16");
                        nodes.push(cast(input, &half, FLOAT16));
                        half
                    })
                    .clone();
                *input = name;
            }
        }
        let out = n.output[0].clone();
        let half = format!("{out}__fp16out");
        n.output[0] = half.clone();
        nodes.push(n);
        nodes.push(cast(&half, &out, FLOAT));
    }
    g.node = nodes;
    for t in added {
        g.initializer.push(t?);
    }
    Ok(())
}

fn to_half(t: &pb::TensorProto, name: &str) -> Result<pb::TensorProto, EvalError> {
    if !t.external_data.is_empty() {
        return Err(err(format!("{}: external data", t.name)));
    }
    let values: Vec<f32> = if !t.raw_data.is_empty() {
        t.raw_data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    } else {
        t.float_data.clone()
    };
    let mut raw = Vec::with_capacity(values.len() * 2);
    for x in values {
        raw.extend(cantrip_obs::encode::f16_bits(x).to_le_bytes());
    }
    Ok(pb::TensorProto {
        dims: t.dims.clone(),
        data_type: FLOAT16,
        name: name.to_string(),
        raw_data: raw,
        ..Default::default()
    })
}

const UINT8: i32 = 2;
const INT32: i32 = 6;
const INT64: i32 = 7;

/// The graph taking its integer inputs as narrow as the encoding makes them - kind, categoricals and pointers as bytes,
/// the card row as i32 - and widening them to the i64 it was exported with on the device, as its first nodes. The values
/// are the same; a batch's copy to the device is a quarter of the size.
pub fn narrow_inputs(model: &mut pb::ModelProto) -> Result<(), EvalError> {
    let g = model.graph.as_mut().ok_or_else(|| err("a model without a graph"))?;
    let mut casts = Vec::new();
    for (name, to) in [("kind", UINT8), ("card", INT32), ("cats", UINT8), ("ptrs", UINT8)] {
        let input = g
            .input
            .iter_mut()
            .find(|i| i.name == name)
            .ok_or_else(|| err(format!("no input {name}")))?;
        let Some(pb::type_proto::Value::TensorType(t)) = input.r#type.as_mut().and_then(|t| t.value.as_mut()) else {
            return Err(err(format!("input {name} is no tensor")));
        };
        if t.elem_type != INT64 {
            return Err(err(format!("input {name} of element type {}, not i64", t.elem_type)));
        }
        t.elem_type = to;
        let wide = format!("{name}__i64");
        for n in &mut g.node {
            for i in n.input.iter_mut().filter(|i| *i == name) {
                *i = wide.clone();
            }
        }
        casts.push(pb::NodeProto {
            input: vec![name.to_string()],
            output: vec![wide.clone()],
            name: format!("{wide}__cast"),
            op_type: "Cast".into(),
            attribute: vec![pb::AttributeProto {
                name: "to".into(),
                r#type: pb::attribute_proto::AttributeType::Int as i32,
                i: INT64 as i64,
                ..Default::default()
            }],
            ..Default::default()
        });
    }
    casts.append(&mut g.node);
    g.node = casts;
    Ok(())
}

/// A bucket's graph as the engine runs it at `precision`.
fn graph_bytes(path: &Path, precision: Precision) -> Result<Vec<u8>, EvalError> {
    let bytes = std::fs::read(path).map_err(|e| err(format!("{}: {e}", path.display())))?;
    let mut model = pb::ModelProto::decode(&bytes[..]).map_err(|e| err(format!("{}: {e}", path.display())))?;
    narrow_inputs(&mut model).map_err(|e| err(format!("{}: {e}", path.display())))?;
    if precision == Precision::Fp16 {
        half_matmuls(&mut model).map_err(|e| err(format!("{}: {e}", path.display())))?;
    }
    Ok(model.encode_to_vec())
}

// --- Rows ----------------------------------------------------------------------------------------------------------

/// Positions as the engine takes them: each row's real tokens end to end (the wire's form, `remote`), the card rows
/// already the model's, and the numbers sparse.
#[derive(Clone, Debug, Default)]
pub struct Rows {
    /// Each row's real tokens (at most `MAX_TOKENS`).
    pub lengths: Vec<usize>,
    pub kind: Vec<u8>,
    pub card: Vec<u32>,
    /// [N, CATS]
    pub cats: Vec<u8>,
    /// [N, PTRS]
    pub ptrs: Vec<u8>,
    pub legal: Vec<bool>,
    /// Token i's non-zero numbers are `num_feature[num_start[i]..num_start[i + 1]]` and their values.
    pub num_start: Vec<u32>,
    pub num_feature: Vec<u8>,
    pub num_value: Vec<f32>,
}

impl Rows {
    pub fn rows(&self) -> usize {
        self.lengths.len()
    }

    fn tokens(&self) -> usize {
        self.kind.len()
    }

    /// Encoded positions, each at its own length, their card rows mapped through `remap` (empty: unmapped): the rows
    /// `from_batch` makes of `FeatureBatch::new(positions)`, without padding them to one size first.
    pub fn from_positions(positions: &[&Encoded], remap: &[u32]) -> Result<Rows, EvalError> {
        let mut r = Rows {
            num_start: vec![0],
            ..Rows::default()
        };
        // `FeatureBatch::set` copies a position's first `len` tokens and its legal bits up to its bucket: a legal bit
        // past `len` (which no encoder sets) is a token of padding and that bit, which the padded path keeps.
        if positions.iter().any(|e| e.legal[e.len.min(e.bucket)..e.bucket].iter().any(|&l| l)) {
            return Rows::from_batch(&FeatureBatch::new(positions), remap);
        }
        for e in positions {
            let pad = |k: usize| {
                e.kind[k] == 0
                    && e.card[k] == UNKNOWN_CARD
                    && !e.legal[k]
                    && e.cats[k * CATS..(k + 1) * CATS].iter().all(|&c| c == 0)
                    && e.ptrs[k * PTRS..(k + 1) * PTRS].iter().all(|&p| p == NO_POINTER)
                    && e.nums[k * NUMS..(k + 1) * NUMS].iter().all(|x| x.to_bits() == 0)
            };
            let len = (0..e.len.min(e.bucket)).rev().find(|&k| !pad(k)).map_or(0, |k| k + 1);
            if len > MAX_TOKENS {
                return Err(err(format!("a row of {len} tokens")));
            }
            r.lengths.push(len);
            r.kind.extend_from_slice(&e.kind[..len]);
            if remap.is_empty() {
                r.card.extend_from_slice(&e.card[..len]);
            } else {
                r.card.extend(e.card[..len].iter().map(|&c| remap.get(c as usize).copied().unwrap_or(UNKNOWN_CARD)));
            }
            r.cats.extend_from_slice(&e.cats[..len * CATS]);
            r.ptrs.extend_from_slice(&e.ptrs[..len * PTRS]);
            r.legal.extend_from_slice(&e.legal[..len]);
            for k in 0..len {
                for (f, &x) in e.nums[k * NUMS..(k + 1) * NUMS].iter().enumerate() {
                    if x.to_bits() != 0 {
                        r.num_feature.push(f as u8);
                        r.num_value.push(x);
                    }
                }
                r.num_start.push(r.num_feature.len() as u32);
            }
        }
        Ok(r)
    }

    /// A batch's rows (`FeatureBatch`, padded), its card rows mapped through `remap` (empty: unmapped). A row's real
    /// tokens are those up to its last one that is not padding, as `remote::request` counts them.
    pub fn from_batch(b: &FeatureBatch, remap: &[u32]) -> Result<Rows, EvalError> {
        let t = b.tokens;
        let mut r = Rows {
            num_start: vec![0],
            ..Rows::default()
        };
        for i in 0..b.rows {
            let len = (0..t)
                .rev()
                .find(|&k| !padding(b, i * t + k))
                .map_or(0, |k| k + 1);
            if len > MAX_TOKENS {
                return Err(err(format!("a row of {len} tokens")));
            }
            r.lengths.push(len);
            for at in i * t..i * t + len {
                r.kind.push(byte(b.kind[at], "a kind")?);
                let c = u32::try_from(b.card[at])
                    .map_err(|_| err(format!("a card row {} outside the index", b.card[at])))?;
                r.card.push(if remap.is_empty() {
                    c
                } else {
                    remap.get(c as usize).copied().unwrap_or(UNKNOWN_CARD)
                });
                for &c in &b.cats[at * CATS..(at + 1) * CATS] {
                    r.cats.push(byte(c, "a categorical")?);
                }
                for &p in &b.ptrs[at * PTRS..(at + 1) * PTRS] {
                    r.ptrs.push(byte(p, "a pointer")?);
                }
                r.legal.push(b.legal[at]);
                for (k, &x) in b.nums[at * NUMS..(at + 1) * NUMS].iter().enumerate() {
                    if x.to_bits() != 0 {
                        r.num_feature.push(k as u8);
                        r.num_value.push(x);
                    }
                }
                r.num_start.push(r.num_feature.len() as u32);
            }
        }
        Ok(r)
    }

    /// An evaluate frame of `remote`'s protocol (its rows in the frame's order).
    pub fn from_wire(bytes: &[u8]) -> Result<Rows, EvalError> {
        use crate::remote::{EVALUATE, NUMS_DENSE, NUMS_SPARSE};
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], EvalError> {
            let s = bytes
                .get(at..at.saturating_add(n))
                .ok_or_else(|| err("a frame cut short"))?;
            at += n;
            Ok(s)
        };
        if take(1)?[0] != EVALUATE {
            return Err(err("not an evaluate frame"));
        }
        let rows = u32::from_le_bytes(take(4)?.try_into().unwrap()) as usize;
        let width = take(1)?[0] as usize;
        let form = take(1)?[0];
        if width != 2 && width != 4 {
            return Err(err(format!("a card width of {width}")));
        }
        let lengths: Vec<usize> = take(rows)?.iter().map(|&l| l as usize).collect();
        if lengths.iter().any(|&l| l > MAX_TOKENS) {
            return Err(err("a row longer than the largest bucket"));
        }
        let n: usize = lengths.iter().sum();
        let kind = take(n)?.to_vec();
        let cards = take(n * width)?;
        let card: Vec<u32> = if width == 2 {
            cards
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]) as u32)
                .collect()
        } else {
            cards
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
                .collect()
        };
        let cats = take(n * CATS)?.to_vec();
        let ptrs = take(n * PTRS)?.to_vec();
        let bits = take(n.div_ceil(8))?;
        let legal = (0..n).map(|i| bits[i / 8] & (1 << (i % 8)) != 0).collect();
        let mut num_start = Vec::with_capacity(n + 1);
        num_start.push(0u32);
        let (num_feature, num_value) = match form {
            NUMS_SPARSE => {
                let counts = take(n)?;
                let mut total = 0u32;
                for &c in counts {
                    total += c as u32;
                    num_start.push(total);
                }
                let k = total as usize;
                let features = take(k)?.to_vec();
                if let Some(&f) = features.iter().find(|&&f| f as usize >= NUMS) {
                    return Err(err(format!("a number's feature {f}")));
                }
                let values = take(2 * k)?
                    .chunks_exact(2)
                    .map(|c| cantrip_obs::encode::f16_value(u16::from_le_bytes([c[0], c[1]])))
                    .collect();
                (features, values)
            }
            NUMS_DENSE => {
                let dense = take(4 * n * NUMS)?;
                let (mut features, mut values) = (Vec::new(), Vec::new());
                for i in 0..n {
                    for k in 0..NUMS {
                        let at = 4 * (i * NUMS + k);
                        let x = f32::from_le_bytes(dense[at..at + 4].try_into().unwrap());
                        if x.to_bits() != 0 {
                            features.push(k as u8);
                            values.push(x);
                        }
                    }
                    num_start.push(features.len() as u32);
                }
                (features, values)
            }
            other => return Err(err(format!("a numbers form {other}"))),
        };
        if at != bytes.len() {
            return Err(err("a frame with bytes left over"));
        }
        Ok(Rows {
            lengths,
            kind,
            card,
            cats,
            ptrs,
            legal,
            num_start,
            num_feature,
            num_value,
        })
    }
}

fn byte(v: i64, what: &str) -> Result<u8, EvalError> {
    u8::try_from(v).map_err(|_| err(format!("{what} {v} does not fit a byte")))
}

/// Whether token `at` of a batch is all padding.
fn padding(b: &FeatureBatch, at: usize) -> bool {
    b.kind[at] == 0
        && b.card[at] == UNKNOWN_CARD as i64
        && !b.legal[at]
        && b.cats[at * CATS..(at + 1) * CATS].iter().all(|&c| c == 0)
        && b.ptrs[at * PTRS..(at + 1) * PTRS]
            .iter()
            .all(|&p| p == NO_POINTER as i64)
        && b.nums[at * NUMS..(at + 1) * NUMS]
            .iter()
            .all(|x| x.to_bits() == 0)
}

/// What the network makes of a request's rows: value [rows], aux [rows, 2], and the policy logit of every real token
/// (`MASKED` where it is no legal option), end to end as the rows' tokens are.
#[derive(Clone, Debug, Default)]
pub struct RowOutputs {
    pub value: Vec<f32>,
    pub aux: Vec<f32>,
    pub logits: Vec<f32>,
}

impl RowOutputs {
    /// The outputs frame's payload of `remote`'s protocol: value, aux and the logits at the legal tokens.
    pub fn reply(&self, legal: &[bool]) -> Vec<u8> {
        let n = legal.iter().filter(|&&l| l).count();
        let mut v = Vec::with_capacity(1 + 4 * (3 * self.value.len() + n));
        v.push(0u8);
        for x in &self.value {
            v.extend(x.to_le_bytes());
        }
        for x in &self.aux {
            v.extend(x.to_le_bytes());
        }
        for (x, &l) in self.logits.iter().zip(legal) {
            if l {
                v.extend(x.to_le_bytes());
            }
        }
        v
    }

    /// As the local evaluators give them: row `i` of `lengths` real tokens has the options `options[i]` (its first
    /// option's token and how many).
    pub fn outputs(&self, lengths: &[usize], options: impl Iterator<Item = (usize, usize)>) -> Outputs {
        let mut start = 0;
        let mut policy = Vec::with_capacity(lengths.len());
        for (&len, (first, n)) in lengths.iter().zip(options) {
            policy.push(
                (first..first + n)
                    .map(|k| if k < len { self.logits[start + k] } else { MASKED })
                    .collect(),
            );
            start += len;
        }
        Outputs {
            value: self.value.clone(),
            policy,
            aux: self.aux.chunks_exact(2).map(|a| [a[0], a[1]]).collect(),
        }
    }
}

// --- The engine ----------------------------------------------------------------------------------------------------

struct Request {
    rows: Rows,
    /// Each row's first token.
    first: Vec<usize>,
    out: Mutex<Pending>,
    done: Condvar,
}

struct Pending {
    out: RowOutputs,
    remaining: usize,
    error: Option<String>,
    /// Whether the engine has stopped counting the request as waiting.
    answered: bool,
}

/// Rows `rows[next..]` of a request, bound for one queue.
struct Group {
    request: Arc<Request>,
    rows: Vec<usize>,
    next: usize,
    arrived: Instant,
}

#[derive(Default)]
struct Queue {
    groups: VecDeque<Group>,
    rows: usize,
    oldest: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub requests: u64,
    pub rows: u64,
    pub batches: u64,
    pub padded: u64,
    /// Seconds a batch was being assembled, run or answered, summed over the workers.
    pub busy: f64,
    /// Seconds in the session's run, summed.
    pub gpu: f64,
    /// Seconds a request spent in the engine, summed over requests.
    pub latency: f64,
    /// Seconds a row group spent queued, summed over groups, and the groups.
    pub queued: f64,
    pub groups: u64,
    pub rows_by_bucket: [u64; 3],
    pub batches_by_bucket: [u64; 3],
}

struct State {
    queues: Vec<Queue>,
    clients: usize,
    waiting: usize,
    running: usize,
    stopping: bool,
    stats: Stats,
}

/// A model the engine serves.
pub struct Served {
    pub name: String,
    pub dir: std::path::PathBuf,
    /// FNV-1a 64 of its `manifest.json` (`remote::model_identity`).
    pub identity: u64,
}

struct Shared {
    options: Options,
    models: Vec<Served>,
    state: Mutex<State>,
    work: Condvar,
    started: Instant,
}

/// The models on the GPU and the batching of every caller's rows; its workers stop when it is dropped.
pub struct Engine {
    shared: Arc<Shared>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

/// CUDA's runtime (`cudart`), loaded at run time beside onnxruntime's CUDA provider (which loads the same library):
/// the engine copies each batch's inputs into device buffers itself, on a stream of its own, so the session can run
/// them as a captured CUDA graph.
mod cuda {
    use std::ffi::{c_char, c_void};
    use std::sync::OnceLock;

    use super::{EvalError, err};

    type Stream = *mut c_void;

    pub struct Runtime {
        _lib: libloading::Library,
        memcpy_async: unsafe extern "C" fn(*mut c_void, *const c_void, usize, i32, Stream) -> i32,
        stream_synchronize: unsafe extern "C" fn(Stream) -> i32,
        stream_create: unsafe extern "C" fn(*mut Stream, u32) -> i32,
        set_device: unsafe extern "C" fn(i32) -> i32,
        error_string: unsafe extern "C" fn(i32) -> *const c_char,
    }

    pub const HOST_TO_DEVICE: i32 = 1;
    pub const DEVICE_TO_HOST: i32 = 2;
    const NON_BLOCKING: u32 = 1;

    #[cfg(windows)]
    const LIBRARY: &str = "cudart64_12.dll";
    #[cfg(not(windows))]
    const LIBRARY: &str = "libcudart.so.12";

    pub fn runtime() -> Result<&'static Runtime, EvalError> {
        static RT: OnceLock<Result<Runtime, String>> = OnceLock::new();
        RT.get_or_init(|| {
            super::prepare_libraries();
            let path = std::env::var_os("CANTRIP_CUDA_LIBS")
                .map(|d| std::path::PathBuf::from(d).join(LIBRARY))
                .filter(|p| p.exists())
                .unwrap_or_else(|| LIBRARY.into());
            // SAFETY: CUDA's runtime library; its symbols are looked up with their C signatures.
            unsafe {
                let lib = libloading::Library::new(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                macro_rules! symbol {
                    ($name:literal) => {
                        *lib.get($name).map_err(|e| format!("{LIBRARY}: {e}"))?
                    };
                }
                let memcpy_async = symbol!(b"cudaMemcpyAsync\0");
                let stream_synchronize = symbol!(b"cudaStreamSynchronize\0");
                let stream_create = symbol!(b"cudaStreamCreateWithFlags\0");
                let set_device = symbol!(b"cudaSetDevice\0");
                let error_string = symbol!(b"cudaGetErrorString\0");
                Ok(Runtime {
                    memcpy_async,
                    stream_synchronize,
                    stream_create,
                    set_device,
                    error_string,
                    _lib: lib,
                })
            }
        })
        .as_ref()
        .map_err(|e| err(e.clone()))
    }

    impl Runtime {
        fn check(&self, what: &str, code: i32) -> Result<(), EvalError> {
            if code == 0 {
                return Ok(());
            }
            // SAFETY: cudaGetErrorString returns a static C string for any code.
            let text = unsafe { std::ffi::CStr::from_ptr((self.error_string)(code)) };
            Err(err(format!("{what}: CUDA error {code} ({})", text.to_string_lossy())))
        }

        /// A non-blocking stream on `device` (never destroyed: a runner's lives as long as the process's engine).
        pub fn stream(&self, device: i32) -> Result<Stream, EvalError> {
            let mut s: Stream = std::ptr::null_mut();
            self.use_device(device)?;
            // SAFETY: a plain runtime call with a valid out-pointer.
            self.check("cudaStreamCreateWithFlags", unsafe { (self.stream_create)(&mut s, NON_BLOCKING) })?;
            Ok(s)
        }

        /// Makes `device` this thread's current device (a worker's copies run on it).
        pub fn use_device(&self, device: i32) -> Result<(), EvalError> {
            // SAFETY: a plain runtime call.
            self.check("cudaSetDevice", unsafe { (self.set_device)(device) })
        }

        /// # Safety
        /// `dst` and `src` hold `bytes` bytes on the sides `kind` says; `stream` is one of `stream`'s.
        pub unsafe fn copy(&self, dst: *mut c_void, src: *const c_void, bytes: usize, kind: i32, stream: Stream) -> Result<(), EvalError> {
            self.check("cudaMemcpyAsync", unsafe { (self.memcpy_async)(dst, src, bytes, kind, stream) })
        }

        pub fn synchronize(&self, stream: Stream) -> Result<(), EvalError> {
            // SAFETY: a stream this runtime made.
            self.check("cudaStreamSynchronize", unsafe { (self.stream_synchronize)(stream) })
        }
    }
}

/// A tensor on the device and its pinned twin on the host: the engine writes the host one and copies it over (an
/// input), or the session writes the device one and the engine copies it back (an output).
struct Buffer {
    /// Owns the pinned memory host_ptr points into.
    _host: ort::value::DynTensor,
    /// Bound to the session (an output's is moved into the binding).
    device: Option<ort::value::DynTensor>,
    host_ptr: *mut std::ffi::c_void,
    device_ptr: *mut std::ffi::c_void,
    bytes: usize,
}

impl Buffer {
    fn new<T: ort::value::PrimitiveTensorElementType + std::fmt::Debug>(
        pinned: &ort::memory::Allocator,
        device: &ort::memory::Allocator,
        shape: &[usize],
    ) -> Result<Buffer, EvalError> {
        let mut host = ort::value::Tensor::<T>::new(pinned, shape.to_vec()).map_err(err)?.upcast();
        let mut dev = ort::value::Tensor::<T>::new(device, shape.to_vec()).map_err(err)?.upcast();
        let bytes = shape.iter().product::<usize>() * size_of::<T>();
        let (host_ptr, device_ptr) = (host.data_ptr_mut(), dev.data_ptr_mut());
        Ok(Buffer {
            _host: host,
            device: Some(dev),
            host_ptr,
            device_ptr,
            bytes,
        })
    }

    /// The host side as a slice of `T`.
    ///
    /// # Safety
    /// `T` is the element type the buffer was made with.
    unsafe fn host<T>(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.host_ptr.cast::<T>(), self.bytes / size_of::<T>()) }
    }
}

/// One session: a model's bucket at one batch size, its inputs and outputs bound to device buffers (so it can replay a
/// captured CUDA graph), and their pinned host twins the batch is written into and read from.
struct Runner {
    session: Session,
    binding: ort::session::IoBinding,
    stream: *mut std::ffi::c_void,
    t: usize,
    b: usize,
    /// kind u8, card i32, cats u8, ptrs u8, nums f32, legal bool: `INPUTS`' order.
    inputs: Vec<Buffer>,
    /// value, aux, policy (belief is bound on the device only).
    outputs: Vec<Buffer>,
    /// Each slot's tokens the last batch wrote, to be put back to padding.
    written: Vec<usize>,
    /// Slots the last batch filled.
    used: usize,
}

// SAFETY: the raw pointers are to buffers and a stream the runner owns; a runner is used by one worker at a time.
unsafe impl Send for Runner {}

const INPUTS: [&str; 6] = ["kind", "card", "cats", "ptrs", "nums", "legal"];

impl Runner {
    fn new(bytes: &[u8], options: &Options, t: usize, b: usize) -> Result<Runner, EvalError> {
        use ort::memory::{AllocationDevice, Allocator, AllocatorType, MemoryInfo, MemoryType};
        let session = session(bytes, options)?;
        let device = Allocator::new(
            &session,
            MemoryInfo::new(AllocationDevice::CUDA, options.device, AllocatorType::Device, MemoryType::Default).map_err(err)?,
        )
        .map_err(err)?;
        let pinned = Allocator::new(
            &session,
            MemoryInfo::new(AllocationDevice::CUDA_PINNED, 0, AllocatorType::Device, MemoryType::CPUInput).map_err(err)?,
        )
        .map_err(err)?;
        let inputs = vec![
            Buffer::new::<u8>(&pinned, &device, &[b, t])?,
            Buffer::new::<i32>(&pinned, &device, &[b, t])?,
            Buffer::new::<u8>(&pinned, &device, &[b, t, CATS])?,
            Buffer::new::<u8>(&pinned, &device, &[b, t, PTRS])?,
            Buffer::new::<f32>(&pinned, &device, &[b, t, NUMS])?,
            Buffer::new::<bool>(&pinned, &device, &[b, t])?,
        ];
        let mut outputs = vec![
            Buffer::new::<f32>(&pinned, &device, &[b])?,
            Buffer::new::<f32>(&pinned, &device, &[b, 2])?,
            Buffer::new::<f32>(&pinned, &device, &[b, t])?,
        ];
        let belief = ort::value::Tensor::<f32>::new(&device, vec![b, t]).map_err(err)?.upcast();
        let mut binding = session.create_binding().map_err(err)?;
        for (name, buffer) in INPUTS.iter().zip(&inputs) {
            let d = buffer.device.as_ref().ok_or_else(|| err("an input without its device tensor"))?;
            binding.bind_input(*name, d).map_err(err)?;
        }
        for (name, buffer) in ["value", "aux", "policy"].iter().zip(&mut outputs) {
            let d = buffer.device.take().ok_or_else(|| err("an output without its device tensor"))?;
            binding.bind_output(*name, d).map_err(err)?;
        }
        binding.bind_output("belief", belief).map_err(err)?;
        let stream = cuda::runtime()?.stream(options.device)?;
        let mut r = Runner {
            session,
            binding,
            stream,
            t,
            b,
            inputs,
            outputs,
            written: vec![t; b],
            used: b,
        };
        // Every slot padding to begin with.
        for s in 0..b {
            r.fill(s, None);
        }
        r.used = 0;
        Ok(r)
    }

    /// Slot `s` as a row of `rows` (its tokens from `first`), or as padding (`None`). Only what the slot's last row
    /// wrote is put back to padding.
    fn fill(&mut self, s: usize, row: Option<(&Rows, usize, usize)>) {
        let t = self.t;
        let len = row.map_or(0, |(r, i, _)| r.lengths[i]);
        let base = s * t;
        let clear = self.written[s].max(len);
        // SAFETY: each buffer is read as the element type it was made with (`INPUTS`' order).
        let (kind, card, cats, ptrs, nums, legal) = unsafe {
            let [k, c, ca, p, n, l] = &mut self.inputs[..] else { unreachable!() };
            (k.host::<u8>(), c.host::<i32>(), ca.host::<u8>(), p.host::<u8>(), n.host::<f32>(), l.host::<bool>())
        };
        nums[base * NUMS..(base + clear) * NUMS].fill(0.0);
        if let Some((r, _, first)) = row {
            kind[base..base + len].copy_from_slice(&r.kind[first..first + len]);
            for k in 0..len {
                card[base + k] = r.card[first + k] as i32;
            }
            cats[base * CATS..(base + len) * CATS].copy_from_slice(&r.cats[first * CATS..(first + len) * CATS]);
            ptrs[base * PTRS..(base + len) * PTRS].copy_from_slice(&r.ptrs[first * PTRS..(first + len) * PTRS]);
            legal[base..base + len].copy_from_slice(&r.legal[first..first + len]);
            for k in 0..len {
                let at = (base + k) * NUMS;
                let (n0, n1) = (r.num_start[first + k] as usize, r.num_start[first + k + 1] as usize);
                for j in n0..n1 {
                    nums[at + r.num_feature[j] as usize] = r.num_value[j];
                }
            }
        }
        let (p0, p1) = (base + len, base + clear);
        kind[p0..p1].fill(0);
        card[p0..p1].fill(UNKNOWN_CARD as i32);
        cats[p0 * CATS..p1 * CATS].fill(0);
        ptrs[p0 * PTRS..p1 * PTRS].fill(NO_POINTER);
        legal[p0..p1].fill(false);
        self.written[s] = len;
    }

    /// Runs the network over the padded inputs: copied in, run, and the outputs copied back to `value`, `aux` and
    /// `policy`.
    fn run(&mut self) -> Result<(), EvalError> {
        let rt = cuda::runtime()?;
        for b in &self.inputs {
            // SAFETY: both sides hold `bytes`; the host side is pinned.
            unsafe { rt.copy(b.device_ptr, b.host_ptr, b.bytes, cuda::HOST_TO_DEVICE, self.stream)? };
        }
        rt.synchronize(self.stream)?;
        self.session.run_binding(&self.binding).map_err(err)?;
        for b in &self.outputs {
            // SAFETY: as above, the other way.
            unsafe { rt.copy(b.host_ptr, b.device_ptr, b.bytes, cuda::DEVICE_TO_HOST, self.stream)? };
        }
        rt.synchronize(self.stream)
    }

    fn value(&mut self) -> &[f32] {
        // SAFETY: an f32 buffer.
        unsafe { self.outputs[0].host::<f32>() }
    }

    fn aux(&mut self) -> &[f32] {
        // SAFETY: an f32 buffer.
        unsafe { self.outputs[1].host::<f32>() }
    }

    fn policy(&mut self) -> &[f32] {
        // SAFETY: an f32 buffer.
        unsafe { self.outputs[2].host::<f32>() }
    }
}

fn session(bytes: &[u8], options: &Options) -> Result<Session, EvalError> {
    let env = crate::eval::ort_environment()?;
    let cuda = ort::ep::CUDA::default()
        .with_device_id(options.device)
        .with_tf32(options.precision == Precision::Tf32)
        .with_cuda_graph(options.cuda_graphs)
        .build()
        .error_on_failure();
    let mut builder = Session::builder(env).map_err(err)?;
    // onnxruntime's profile of every node (a JSON trace a session, written when it ends).
    if let Some(prefix) = std::env::var_os("CANTRIP_ORT_PROFILE") {
        builder = builder.with_profiling(prefix).map_err(|e| err(e.to_string()))?;
    }
    builder
        .with_execution_providers([cuda])
        .map_err(|e| err(e.to_string()))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| err(e.to_string()))?
        .with_deterministic_compute(true)
        .map_err(|e| err(e.to_string()))?
        .with_intra_threads(1)
        .map_err(|e| err(e.to_string()))?
        .with_inter_threads(1)
        .map_err(|e| err(e.to_string()))?
        .commit_from_memory(bytes)
        .map_err(err)
}

/// Windows rounds a timed wait up to its timer's 15.6 ms unless a process asks for 1 ms; the flush time is a few
/// milliseconds.
fn fine_timer() {
    #[cfg(windows)]
    {
        #[link(name = "winmm")]
        unsafe extern "system" {
            fn timeBeginPeriod(period: u32) -> u32;
        }
        // SAFETY: a plain Win32 call; the period stays asked for as long as the process lives.
        unsafe { timeBeginPeriod(1) };
    }
}

/// A worker above the threads that feed it (self-play runs more lanes than cores, and a worker starved of the CPU
/// starves every lane), where the OS allows it: the in-process engine's counterpart of the server's raised priority.
fn raise_thread_priority() {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn GetCurrentThread() -> *mut std::ffi::c_void;
            fn SetThreadPriority(thread: *mut std::ffi::c_void, priority: i32) -> i32;
        }
        const THREAD_PRIORITY_HIGHEST: i32 = 2;
        // SAFETY: a plain Win32 call on this thread's own pseudo-handle.
        unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) };
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn bucket_of(len: usize) -> usize {
    BUCKETS
        .iter()
        .position(|&b| b >= len.max(1))
        .unwrap_or(BUCKETS.len() - 1)
}

impl Engine {
    /// The models in `dirs`, each bucket's graph on the GPU `options.pipelines` times.
    pub fn start(dirs: &[&Path], options: Options) -> Result<Engine, EvalError> {
        fine_timer();
        let mut models = Vec::new();
        let mut graphs = Vec::new();
        for dir in dirs {
            let text = std::fs::read(dir.join("manifest.json"))
                .map_err(|e| err(format!("{}: {e}", dir.join("manifest.json").display())))?;
            let manifest: crate::Manifest = serde_json::from_slice(&text)
                .map_err(|e| err(format!("{}: {e}", dir.display())))?;
            // The encoding is this build's (a model of another schema or encoder hash is refused).
            crate::eval::check_encoding(&manifest).map_err(|e| err(format!("{}: {e}", dir.display())))?;
            let mut bytes = Vec::new();
            for t in BUCKETS {
                let file = manifest
                    .buckets
                    .get(&t.to_string())
                    .ok_or_else(|| err(format!("{}: no graph for {t} tokens", dir.display())))?;
                bytes.push(graph_bytes(&dir.join(file), options.precision)?);
            }
            graphs.push(bytes);
            models.push(Served {
                name: dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                dir: dir.to_path_buf(),
                identity: crate::remote::model_identity(dir)?,
            });
        }
        let mut seen = std::collections::HashSet::new();
        if !models.iter().all(|m| seen.insert(m.identity)) {
            return Err(err("the same model twice"));
        }
        let pipelines = options.pipelines.max(1);
        let mut runners: Vec<Vec<Runner>> = Vec::new();
        for _ in 0..pipelines {
            let mut mine = Vec::new();
            for bytes in &graphs {
                for (k, t) in BUCKETS.iter().enumerate() {
                    mine.push(Runner::new(&bytes[k], &options, *t, options.batch[k].max(1))?);
                }
            }
            runners.push(mine);
        }
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                queues: (0..models.len() * BUCKETS.len()).map(|_| Queue::default()).collect(),
                clients: 0,
                waiting: 0,
                running: 0,
                stopping: false,
                stats: Stats::default(),
            }),
            options,
            models,
            work: Condvar::new(),
            started: Instant::now(),
        });
        // Each worker warms its sessions up on its own thread (onnxruntime's CUDA provider keeps a CUDA graph per thread):
        // the first run allocates, the second captures the graph. A capture forbids every other thread's stream
        // synchronisation while it lasts, so the workers capture one at a time, and none serves a batch before all
        // have captured.
        let (ready, warmed) = std::sync::mpsc::channel::<Result<(), EvalError>>();
        let go = Arc::new(std::sync::Barrier::new(pipelines + 1));
        let capture = Arc::new(Mutex::new(()));
        let mut workers = Vec::new();
        for (k, mut mine) in runners.into_iter().enumerate() {
            let (shared, ready, go, capture) = (shared.clone(), ready.clone(), go.clone(), capture.clone());
            workers.push(
                std::thread::Builder::new()
                    .name(format!("infer-{k}"))
                    .spawn(move || {
                        raise_thread_priority();
                        let warm = (|| {
                            cuda::runtime()?.use_device(shared.options.device)?;
                            let _one = lock(&capture);
                            for r in &mut mine {
                                for _ in 0..3 {
                                    r.run()?;
                                }
                            }
                            Ok(())
                        })();
                        let ok = warm.is_ok();
                        let _ = ready.send(warm);
                        go.wait();
                        if ok {
                            work(&shared, mine);
                        }
                    })
                    .map_err(err)?,
            );
        }
        drop(ready);
        let mut failed = None;
        for _ in 0..pipelines {
            match warmed.recv() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => failed = failed.or(Some(e)),
                Err(_) => failed = failed.or(Some(err("an inference worker ended while warming up"))),
            }
        }
        if failed.is_some() {
            lock(&shared.state).stopping = true;
        }
        go.wait();
        let engine = Engine { shared, workers };
        match failed {
            Some(e) => Err(e),
            None => Ok(engine),
        }
    }

    pub fn models(&self) -> &[Served] {
        &self.shared.models
    }

    pub fn options(&self) -> &Options {
        &self.shared.options
    }

    /// The model with this identity (`remote::model_identity`).
    pub fn model(&self, identity: u64) -> Option<usize> {
        self.shared.models.iter().position(|m| m.identity == identity)
    }

    /// A caller of the engine, counted while it lives: when every caller is waiting, a partial batch runs at once.
    pub fn client(&self) -> Client {
        lock(&self.shared.state).clients += 1;
        Client {
            shared: self.shared.clone(),
        }
    }

    pub fn stats(&self) -> Stats {
        lock(&self.shared.state).stats.clone()
    }

    pub fn summary(&self) -> String {
        let s = self.stats();
        let seconds = self.shared.started.elapsed().as_secs_f64().max(1e-9);
        let st = lock(&self.shared.state);
        format!(
            "infer: {} requests, {} rows ({:.0} a second) in {} batches (T32: {} rows in {}, T64: {} in {}, T128: {} in {}), \
             {:.0}% of the padded rows real; workers busy {:.0}%, in the network {:.0}%; a request {:.1} ms in the engine, a \
             row group queued {:.1} ms; {} clients",
            s.requests,
            s.rows,
            s.rows as f64 / seconds,
            s.batches,
            s.rows_by_bucket[0],
            s.batches_by_bucket[0],
            s.rows_by_bucket[1],
            s.batches_by_bucket[1],
            s.rows_by_bucket[2],
            s.batches_by_bucket[2],
            100.0 * s.rows as f64 / s.padded.max(1) as f64,
            100.0 * s.busy / seconds,
            100.0 * s.gpu / seconds,
            1e3 * s.latency / s.requests.max(1) as f64,
            1e3 * s.queued / s.groups.max(1) as f64,
            st.clients
        )
    }

    /// The rows through model `model`, as soon as their batches have run.
    pub fn evaluate(&self, model: usize, rows: Rows) -> Result<RowOutputs, EvalError> {
        evaluate(&self.shared, model, rows)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        lock(&self.shared.state).stopping = true;
        self.shared.work.notify_all();
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

/// See `Engine::client`.
pub struct Client {
    shared: Arc<Shared>,
}

impl Drop for Client {
    fn drop(&mut self) {
        lock(&self.shared.state).clients -= 1;
        self.shared.work.notify_all();
    }
}

fn evaluate(shared: &Shared, model: usize, rows: Rows) -> Result<RowOutputs, EvalError> {
    let n = rows.rows();
    if model >= shared.models.len() {
        return Err(err(format!("no model {model}")));
    }
    if n == 0 {
        return Ok(RowOutputs::default());
    }
    let arrived = Instant::now();
    let mut first = Vec::with_capacity(n);
    let mut at = 0;
    for &l in &rows.lengths {
        first.push(at);
        at += l;
    }
    if at != rows.tokens() || rows.num_start.len() != at + 1 {
        return Err(err("rows whose arrays disagree"));
    }
    let tokens = at;
    let mut by_bucket: [Vec<usize>; 3] = Default::default();
    for (i, &l) in rows.lengths.iter().enumerate() {
        by_bucket[bucket_of(l)].push(i);
    }
    let request = Arc::new(Request {
        rows,
        first,
        out: Mutex::new(Pending {
            out: RowOutputs {
                value: vec![0.0; n],
                aux: vec![0.0; 2 * n],
                logits: vec![MASKED; tokens],
            },
            remaining: n,
            error: None,
            answered: false,
        }),
        done: Condvar::new(),
    });
    {
        let mut st = lock(&shared.state);
        if st.stopping {
            return Err(err("the inference engine has stopped"));
        }
        st.stats.requests += 1;
        st.stats.rows += n as u64;
        st.waiting += 1;
        for (k, rows) in by_bucket.into_iter().enumerate() {
            if rows.is_empty() {
                continue;
            }
            let q = &mut st.queues[model * BUCKETS.len() + k];
            if q.rows == 0 {
                q.oldest = Some(arrived);
            }
            q.rows += rows.len();
            q.groups.push_back(Group {
                request: request.clone(),
                rows,
                next: 0,
                arrived,
            });
        }
    }
    shared.work.notify_all();
    let mut p = lock(&request.out);
    while p.remaining > 0 && p.error.is_none() {
        p = request.done.wait(p).unwrap_or_else(|e| e.into_inner());
    }
    let latency = arrived.elapsed().as_secs_f64();
    lock(&shared.state).stats.latency += latency;
    if let Some(e) = p.error.take() {
        return Err(err(e));
    }
    Ok(std::mem::take(&mut p.out))
}

/// The queue to run now, or how long to wait for one.
fn ready(st: &State, options: &Options, now: Instant) -> (Option<usize>, Option<Duration>) {
    let mut best: Option<(usize, Instant)> = None;
    for (key, q) in st.queues.iter().enumerate() {
        if q.rows == 0 {
            continue;
        }
        if q.rows >= options.batch[key % BUCKETS.len()].max(1) {
            return (Some(key), None);
        }
        let oldest = q.oldest.unwrap_or(now);
        if best.is_none_or(|(_, o)| oldest < o) {
            best = Some((key, oldest));
        }
    }
    let Some((key, oldest)) = best else {
        return (None, None);
    };
    let waited = now.saturating_duration_since(oldest);
    // Every client waiting and nothing running: no row can come until this runs. (While a batch runs, its answers bring
    // more rows, and a partial batch costs the GPU as much as a full one.)
    if (st.waiting >= st.clients && st.running == 0) || waited >= options.flush {
        return (Some(key), None);
    }
    (None, Some(options.flush - waited))
}

/// Up to a batch of queue `key`'s rows: (request, row) pairs in order.
fn take(st: &mut State, options: &Options, key: usize, now: Instant) -> (Vec<(Arc<Request>, usize)>, f64) {
    let room = options.batch[key % BUCKETS.len()].max(1);
    let q = &mut st.queues[key];
    let mut out = Vec::with_capacity(room.min(q.rows));
    let mut queued = 0.0;
    let mut groups = 0;
    while out.len() < room {
        let Some(g) = q.groups.front_mut() else { break };
        groups += 1;
        queued += now.saturating_duration_since(g.arrived).as_secs_f64();
        while out.len() < room && g.next < g.rows.len() {
            out.push((g.request.clone(), g.rows[g.next]));
            g.next += 1;
        }
        if g.next == g.rows.len() {
            q.groups.pop_front();
        }
    }
    q.rows -= out.len();
    q.oldest = q.groups.front().map(|g| g.arrived);
    st.stats.groups += groups;
    st.stats.queued += queued;
    (out, queued)
}

fn work(shared: &Shared, mut runners: Vec<Runner>) {
    let options = &shared.options;
    loop {
        let (key, batch) = {
            let mut st = lock(&shared.state);
            let key = loop {
                if st.stopping {
                    return;
                }
                let (key, wait) = ready(&st, options, Instant::now());
                if let Some(key) = key {
                    break key;
                }
                st = match wait {
                    Some(w) => shared.work.wait_timeout(st, w).unwrap_or_else(|e| e.into_inner()).0,
                    None => shared.work.wait(st).unwrap_or_else(|e| e.into_inner()),
                };
            };
            let (batch, _) = take(&mut st, options, key, Instant::now());
            st.running += 1;
            (key, batch)
        };
        let started = Instant::now();
        let runner = &mut runners[key];
        let used = batch.len();
        for (s, (r, i)) in batch.iter().enumerate() {
            runner.fill(s, Some((&r.rows, *i, r.first[*i])));
        }
        for s in used..runner.used {
            runner.fill(s, None);
        }
        runner.used = used;
        let ran = Instant::now();
        let result = runner.run();
        let gpu = ran.elapsed().as_secs_f64();
        let mut finished = 0;
        let t = runner.t;
        for (s, (r, i)) in batch.iter().enumerate() {
            let mut p = lock(&r.out);
            match &result {
                Ok(()) => {
                    p.out.value[*i] = runner.value()[s];
                    let aux = runner.aux();
                    p.out.aux[2 * i] = aux[2 * s];
                    p.out.aux[2 * i + 1] = aux[2 * s + 1];
                    let (first, len) = (r.first[*i], r.rows.lengths[*i]);
                    p.out.logits[first..first + len].copy_from_slice(&runner.policy()[s * t..s * t + len]);
                }
                Err(e) => {
                    if p.error.is_none() {
                        p.error = Some(format!("the inference engine's batch failed: {e}"));
                    }
                }
            }
            p.remaining -= 1;
            if (p.remaining == 0 || p.error.is_some()) && !p.answered {
                p.answered = true;
                finished += 1;
                r.done.notify_all();
            }
        }
        let mut st = lock(&shared.state);
        st.running -= 1;
        st.waiting -= finished;
        let s = &mut st.stats;
        s.batches += 1;
        s.padded += runner.b as u64;
        s.gpu += gpu;
        s.busy += started.elapsed().as_secs_f64();
        s.rows_by_bucket[key % BUCKETS.len()] += used as u64;
        s.batches_by_bucket[key % BUCKETS.len()] += 1;
        drop(st);
        shared.work.notify_all();
    }
}

// --- In-process evaluation -----------------------------------------------------------------------------------------

/// A model evaluated by an engine in this process (self-play's `--backend gpu`): a lane an evaluator, all of them
/// sharing one engine, so every lane's rows are batched together without leaving the process.
pub struct GpuEvaluator {
    engine: Arc<Engine>,
    model: usize,
    remap: Vec<u32>,
    _client: Client,
}

impl GpuEvaluator {
    /// Model `model` of the engine, its card rows mapped from `table`'s by the model directory's index.
    pub fn new(engine: Arc<Engine>, model: usize, table: &CardIndex) -> Result<GpuEvaluator, EvalError> {
        let dir = engine
            .models()
            .get(model)
            .ok_or_else(|| err(format!("no model {model}")))?
            .dir
            .clone();
        let remap = Model::load(&dir, table)?.remap;
        let client = engine.client();
        Ok(GpuEvaluator {
            engine,
            model,
            remap,
            _client: client,
        })
    }
}

impl Evaluator for GpuEvaluator {
    fn evaluate(&self, batch: &FeatureBatch) -> Result<Outputs, EvalError> {
        if batch.rows == 0 {
            return Ok(Outputs::default());
        }
        let rows = Rows::from_batch(batch, &self.remap)?;
        let lengths = rows.lengths.clone();
        let out = self.engine.evaluate(self.model, rows)?;
        Ok(out.outputs(&lengths, batch.options.iter().copied()))
    }

    fn evaluate_positions(&self, positions: &[&Encoded]) -> Result<Outputs, EvalError> {
        if positions.is_empty() {
            return Ok(Outputs::default());
        }
        let rows = Rows::from_positions(positions, &self.remap)?;
        let lengths = rows.lengths.clone();
        let out = self.engine.evaluate(self.model, rows)?;
        Ok(out.outputs(&lengths, positions.iter().map(|e| (e.first_option, e.options))))
    }
}

// --- Checking the references ---------------------------------------------------------------------------------------

/// What a precision below fp32 is held to (`serve.py`'s `REDUCED`): value and dead-next-turn within 1e-2, each legal
/// option's probability within 1e-2, the life swing within half a point (or 1% of it above `SWING_RELATIVE` points:
/// fp16's own spacing there is a quarter point and more); and the same best option on 99.5%.
pub const REDUCED_VALUE: f32 = 1e-2;
pub const REDUCED_PROBABILITY: f32 = 1e-2;
pub const REDUCED_SWING: f32 = 0.5;
pub const SWING_RELATIVE: f32 = 50.0;
pub const TOLERANCE: f32 = 1e-3;
pub const SAME_ARGMAX: f64 = 0.995;

#[derive(Clone, Debug, Default)]
pub struct Check {
    pub positions: usize,
    pub value: f32,
    pub swing: f32,
    /// The swing's drift, a swing above `SWING_RELATIVE` points measured relative to its size (to 1% of it below fp32):
    /// what `TOLERANCE` and `REDUCED_SWING` hold.
    pub swing_reduced: f32,
    pub dead: f32,
    /// Over the legal options' logits.
    pub logits: f32,
    /// Over the legal options' probabilities.
    pub probability: f32,
    pub same_argmax: f64,
    pub pass: bool,
}

/// A model's references (`reference/`, PyTorch's outputs at fp32) through the engine, held to export's tolerance at
/// fp32 and to the reduced bar below it.
pub fn check(engine: &Arc<Engine>, model: usize) -> Result<Check, EvalError> {
    let dir = engine.models()[model].dir.clone();
    let r = reference::Reference::load(&dir.join("reference"))?;
    let own = CardIndex::read(&dir.join("cards.json")).map_err(err)?;
    let e = GpuEvaluator::new(engine.clone(), model, &own)?;
    let mut c = Check {
        positions: r.positions.len(),
        ..Check::default()
    };
    let mut same = 0usize;
    for chunk in (0..r.positions.len()).collect::<Vec<_>>().chunks(64) {
        let es: Vec<_> = chunk.iter().map(|&i| &r.positions[i]).collect();
        let out = e.evaluate(&FeatureBatch::new(&es))?;
        for (j, &i) in chunk.iter().enumerate() {
            let p = &r.positions[i];
            c.value = c.value.max((out.value[j] - r.value[i]).abs());
            let d = (out.aux[j][0] - r.aux[i][0]).abs();
            c.swing = c.swing.max(d);
            c.swing_reduced = c.swing_reduced.max(d / (r.aux[i][0].abs() / SWING_RELATIVE).max(1.0));
            c.dead = c.dead.max((out.aux[j][1] - r.aux[i][1]).abs());
            let legal: Vec<usize> = (0..p.options).filter(|&k| p.legal[p.first_option + k]).collect();
            if legal.is_empty() {
                same += 1;
                continue;
            }
            let got: Vec<f32> = legal.iter().map(|&k| out.policy[j][k]).collect();
            let want: Vec<f32> = legal.iter().map(|&k| r.policy[i][k]).collect();
            for (a, b) in got.iter().zip(&want) {
                c.logits = c.logits.max((a - b).abs());
            }
            for (a, b) in softmax(&got).iter().zip(softmax(&want)) {
                c.probability = c.probability.max((a - b).abs() as f32);
            }
            let best = |x: &[f32]| (0..x.len()).max_by(|&a, &b| x[a].total_cmp(&x[b]).then(b.cmp(&a)));
            same += (best(&got) == best(&want)) as usize;
        }
    }
    c.same_argmax = same as f64 / c.positions.max(1) as f64;
    c.pass = c.same_argmax >= SAME_ARGMAX
        && if engine.options().precision == Precision::Fp32 {
            c.value < TOLERANCE && c.swing_reduced < TOLERANCE && c.dead < TOLERANCE && c.logits < TOLERANCE
        } else {
            c.value < REDUCED_VALUE
                && c.dead < REDUCED_VALUE
                && c.probability < REDUCED_PROBABILITY
                && c.swing_reduced < REDUCED_SWING
        };
    Ok(c)
}

pub fn softmax(z: &[f32]) -> Vec<f64> {
    let m = z.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
    let e: Vec<f64> = z.iter().map(|&x| (x as f64 - m).exp()).collect();
    let s: f64 = e.iter().sum();
    e.into_iter().map(|x| x / s).collect()
}
