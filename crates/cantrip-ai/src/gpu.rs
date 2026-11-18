//! Evaluating on the GPU without Python (docs/ai-host.md, "The native inference server"): onnxruntime's CUDA execution
//! provider, with the rows of every caller gathered into GPU batches per model and token bucket. One `Engine` holds the
//! models and the batching; it serves the self-play lanes of its own process (`GpuEvaluator`, `--backend gpu`) and the
//! connections of `cantrip-infer`, the native inference server, which speaks `remote`'s protocol (`--backend remote`).
//!
//! **Batching** is `ai/cantrip_ai/serve.py`'s: a row goes to the smallest bucket (`GRAPH_BUCKETS`) that holds its real
//! tokens and its model has a graph for (a model exported before T96 runs those rows at T128); a GPU call
//! takes up to the bucket's batch size of one queue and is always padded to exactly that many rows, so every call of a
//! bucket has one shape; a queue runs when it holds a full batch, when every client is waiting on the engine and no
//! batch runs (no more rows can come), or when its oldest row has waited the flush time. `pipelines` workers assemble
//! and run batches at once, each with its own sessions (and so its own CUDA stream). Opt-in, a bucket may also run a
//! partial batch at a smaller shape (`Options::shapes`), which gives up the guarantee below.
//!
//! **Reproducibility.** A row's outputs never depend on what it was batched with: one shape per bucket, deterministic
//! compute, and no reduction across rows in the network. (With `Options::shapes`, only as far as the start's probe can
//! tell: see there.)
//!
//! **Precision.** `fp32` keeps TF32 off; `tf32` allows it in matrix products; `fp16` runs every matrix product (MatMul,
//! Gemm) in half precision with its inputs cast to half and its output cast back, which is what PyTorch's autocast does
//! to them, and keeps everything else (the norms, the masking, the heads' non-linearities) in fp32, as it keeps the
//! input stage, everything before the first norm, which reads the raw features and can leave fp16's range
//! (`input_stage`). `Options::fp16` takes more into half precision, each level with the ones before it: the attention fused into one kernel that
//! accumulates in fp32 (`attention`: what PyTorch's `scaled_dot_product_attention` does under autocast); the
//! feed-forward networks' bias and GELU fused, their intermediates half (`ffn`); the residual Adds fused with their norms,
//! the residual stream half (`residual`). Nodes that read and write half read each other's outputs directly. A graph a
//! rewrite does not recognise keeps that part as exported, and the reason is logged. The graph is rewritten in memory
//! as it is loaded; the model directory is never written. Below fp32 a model is held to `REDUCED` on its references at
//! start, as `serve.py` holds it.
//!
//! **Libraries.** onnxruntime is loaded at run time (`ort`'s `load-dynamic`): `ORT_DYLIB_PATH` names an
//! `onnxruntime.dll` built with the CUDA provider (onnxruntime-gpu 1.24, which is built for CUDA 12 and cuDNN 9), and
//! `CANTRIP_CUDA_LIBS` a directory holding CUDA 12's and cuDNN 9's libraries (a PyTorch install's `torch/lib` has them),
//! which is put first on the library search path.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub use cantrip_obs::encode::GRAPH_BUCKETS;
use cantrip_obs::encode::{BUCKETS, CATS, Encoded, MAX_TOKENS, NO_POINTER, NUMS, PTRS, UNKNOWN_CARD};
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use prost::Message;
use tract_onnx::pb;

use crate::CardIndex;
use crate::eval::{EvalError, Evaluator, FeatureBatch, MASKED, Model, Outputs, reference};

mod attention;
mod ffn;
mod graph;
mod residual;
pub use attention::{MASK_FLOOR, fuse_attention};
pub use ffn::{GeluKernel, Gelus, fuse_gelu};
pub use graph::MICROSOFT;
pub use graph::{Shapes, tensor_shapes};
pub use residual::fuse_norms;

fn err(e: impl std::fmt::Display) -> EvalError {
    EvalError(e.to_string())
}

/// The buckets the engine runs graphs at (`GRAPH_BUCKETS`: the encoding's, and T96), each row at the smallest its model
/// has a graph for.
const NB: usize = GRAPH_BUCKETS.len();

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

/// What the fp16 graph runs in half precision besides its matrix products; each level includes the ones before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fp16Graph {
    /// The matrix products alone (MatMul, Gemm), each cast in and out.
    Products,
    /// Each attention fused (`fuse_attention`).
    Attention,
    /// Each GELU after a MatMul fused with its bias, the feed-forward networks' intermediates half (`fuse_gelu`). The
    /// default: within the reduced bar on gen0 and gen1, and half as fast again as `Products` on gen1.
    Ffn,
    /// Each residual Add fused with its norm, the residual stream half (`fuse_norms`). Opt-in only: it fails the
    /// reduced bar on gen2 of the templates line (3 references non-finite; under the policy bar before 2026-11-13 it
    /// failed gen0 too, probability 0.015) and doubles gen1's logit and life swing drift.
    Residual,
}

impl Fp16Graph {
    pub const ALL: [Fp16Graph; 4] = [Fp16Graph::Products, Fp16Graph::Attention, Fp16Graph::Ffn, Fp16Graph::Residual];

    pub fn parse(s: &str) -> Option<Fp16Graph> {
        Fp16Graph::ALL.into_iter().find(|g| g.name() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            Fp16Graph::Products => "products",
            Fp16Graph::Attention => "attention",
            Fp16Graph::Ffn => "ffn",
            Fp16Graph::Residual => "residual",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    /// The CUDA device.
    pub device: i32,
    pub precision: Precision,
    /// Rows a GPU call is padded to, by bucket (`GRAPH_BUCKETS`' order): its full shape.
    pub batch: [usize; NB],
    /// Opt-in, and NOT guaranteed deterministic: smaller shapes a bucket may also run a batch at (those below its full
    /// shape), a partial batch at the smallest that holds it. A shape is kept only where `Engine::start`'s probe saw it
    /// compute every output of every probe row bitwise as the full shape does, but that is evidence, not proof: the
    /// shapes' matrix products run different kernels, and two kernels that add in a different order can agree on most
    /// rows (on gen2 at fp16, T128 at 16 rows agreed with 64 on 64 references of 64 and differed on others). With none
    /// (the default) every batch of a bucket runs at its one shape, and a row's outputs never depend on its companions.
    /// By bucket (`GRAPH_BUCKETS`' order); a shape at or above the bucket's full one is none of its smaller shapes.
    pub shapes: [Vec<usize>; NB],
    /// Batches assembled and run at once.
    pub pipelines: usize,
    /// How long a partial batch waits for more rows.
    pub flush: Duration,
    /// Each session replays its forward as a captured CUDA graph (onnxruntime's `enable_cuda_graph`): one launch a
    /// batch instead of one a node.
    pub cuda_graphs: bool,
    /// At fp16, what runs in half precision besides the matrix products. fp32 and tf32 never rewrite.
    pub fp16: Fp16Graph,
    /// At fp16, the input stage's fp32 products may use TF32 (fp32's range, a 10-bit mantissa as fp16's).
    pub inputs_tf32: bool,
    /// At `Fp16Graph::Ffn` and above, how a fused GELU runs: `Gelu` by default, erf's GELU in fp32 as autocast computes
    /// it (`BiasGelu` is 7% faster on gen1 but drifts 4.5 times as far in probability, margin larger networks need).
    pub gelu: GeluKernel,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            device: 0,
            precision: Precision::Fp32,
            // T96 and T128 at 48 rows: self-play's batches are mostly partial, and a row costs about the same at 48 as
            // at 96 (gen4 at fp16 on an RTX 4080, single stream: T96 21.2k rows a second at 48, 20.9k at 96; T128
            // 14.9k at 48, 16.5k at 64), so a partial batch costs half as much; with two pipelines it measured as fast
            // as 96/64 or faster at every load (docs/ai-host.md, "Batch sizes"). serve.py keeps 96 and 64.
            batch: [256, 128, 48, 48],
            shapes: Default::default(),
            pipelines: 2,
            flush: Duration::from_micros(2000),
            cuda_graphs: true,
            fp16: Fp16Graph::Ffn,
            gelu: GeluKernel::Gelu,
            inputs_tf32: false,
        }
    }
}

/// Puts `CANTRIP_CUDA_LIBS` first on the library search path, so the CUDA provider finds CUDA's and cuDNN's
/// libraries there. Called before onnxruntime is loaded.
pub(crate) fn prepare_libraries() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // cuBLAS without a workspace: no product split over K into a workspace (split-K), where the split depends on
        // how many rows the product has. Without it, the products of a bucket at its full shape and at a smaller one
        // (`Options::shapes`) mostly add each output's terms in the same order, where with it nearly every shape
        // differed; it costs nothing measurable at one shape (`cantrip-infer time`, gen4 at fp16). cuBLAS reads it as
        // it loads, so it is set before onnxruntime (and its CUDA provider) loads; a value already set is kept.
        if std::env::var_os("CUBLAS_WORKSPACE_CONFIG").is_none() {
            // SAFETY: as below.
            unsafe { std::env::set_var("CUBLAS_WORKSPACE_CONFIG", ":0:0") };
        }
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

/// The smaller shapes each bucket may also run at (`Options::shapes`): `N,N,...` for every bucket,
/// `T:N/N/...,T:N/...` by bucket (a bucket not named has none), or `none`.
pub fn parse_shapes(text: &str) -> Result<[Vec<usize>; NB], String> {
    let sizes = |list: &str, sep: char| -> Result<Vec<usize>, String> {
        let mut out = Vec::new();
        for part in list.split(sep) {
            let n: usize = part.trim().parse().map_err(|_| format!("shapes {text}: {part} is not a size"))?;
            if n == 0 {
                return Err(format!("shapes {text} name a size of 0"));
            }
            out.push(n);
        }
        out.sort_unstable();
        out.dedup();
        Ok(out)
    };
    if text == "none" {
        return Ok(Default::default());
    }
    if !text.contains(':') {
        let all = sizes(text, ',')?;
        return Ok(std::array::from_fn(|_| all.clone()));
    }
    let mut out: [Vec<usize>; NB] = Default::default();
    for part in text.split(',') {
        let (t, list) = part.split_once(':').ok_or(format!("shapes {text}: {part} names no bucket"))?;
        let t: usize = t.parse().map_err(|_| format!("shapes {text}: no bucket {t}"))?;
        let k = GRAPH_BUCKETS.iter().position(|&b| b == t).ok_or(format!("no bucket of {t} tokens"))?;
        out[k] = sizes(list, '/')?;
    }
    Ok(out)
}

/// Rows a GPU call is padded to: `N` for every bucket, or `32:N,64:N,96:N,128:N`, where a bucket not named keeps its
/// default (`Options::default`), so a list written before T96 still reads.
pub fn parse_batch(text: &str) -> Result<[usize; NB], String> {
    if !text.contains(':') {
        let n: usize = text.parse().map_err(|_| format!("a batch of {text}"))?;
        return Ok([n; NB]);
    }
    let mut out = Options::default().batch;
    for part in text.split(',') {
        let (t, n) = part.split_once(':').ok_or(format!("a batch of {text}"))?;
        let t: usize = t.parse().map_err(|_| format!("a batch of {text}"))?;
        let k = GRAPH_BUCKETS.iter().position(|&b| b == t).ok_or(format!("no bucket of {t} tokens"))?;
        out[k] = n.parse().map_err(|_| format!("a batch of {text}"))?;
    }
    if out.contains(&0) {
        return Err(format!("a batch of {text} names a size of 0"));
    }
    Ok(out)
}

// --- Half precision ------------------------------------------------------------------------------------------------

const FLOAT: i32 = 1;
const FLOAT16: i32 = 10;

/// Whether a node runs in half precision at fp16: a matrix product, or one of the fused nodes the rewrites make (an
/// attention, a GELU, a residual add and its norm). Those exist only where a rewrite asked for them.
fn halved(n: &pb::NodeProto) -> bool {
    (n.domain.is_empty() && (n.op_type == "MatMul" || n.op_type == "Gemm"))
        || (n.domain == MICROSOFT
            && matches!(n.op_type.as_str(), "MultiHeadAttention" | "BiasGelu" | "Gelu" | "SkipLayerNormalization"))
}

/// The input stage: the nodes (by index) the graph's first norm's input is computed from, and that norm itself when it
/// is a fused residual's (`SkipLayerNormalization`, whose sum is the residual stream's start). It is everything that
/// reads the raw features: the card table, the kind and categorical embeddings, the numbers' projection and the
/// pointers' gather and projection. The numbers are unbounded (a count is scaled linearly: 131,700 counters are 16,464),
/// so these projections can leave fp16's range (gen2's pointers projection did, and NaN reached the policy), where
/// everything after the first norm reads normalised activations: the norm's output, a GELU or an attention of it, the
/// residual stream (fp32 below `Fp16Graph::Residual`). The stage runs once a token, a small share of the network's
/// products. A graph with no norm has no stage to tell, and is refused.
pub fn input_stage(model: &pb::ModelProto) -> Result<HashSet<usize>, String> {
    let g = model.graph.as_ref().ok_or("a model without a graph")?;
    let is_norm = |n: &pb::NodeProto| {
        (n.domain.is_empty() && n.op_type == "LayerNormalization")
            || (n.domain == MICROSOFT && n.op_type == "SkipLayerNormalization")
    };
    let first = g
        .node
        .iter()
        .position(is_norm)
        .ok_or("no LayerNormalization: where the raw features end cannot be told")?;
    let producer: HashMap<&str, usize> =
        g.node.iter().enumerate().flat_map(|(i, n)| n.output.iter().map(move |o| (o.as_str(), i))).collect();
    let norm = &g.node[first];
    let mut stage = HashSet::new();
    // A norm's data inputs: LayerNormalization's X; a fused residual's input, skip and bias (it is in the stage).
    let data: &[usize] = if norm.op_type == "SkipLayerNormalization" {
        stage.insert(first);
        &[0, 1, 4]
    } else {
        &[0]
    };
    let mut todo: Vec<&str> =
        data.iter().filter_map(|&k| norm.input.get(k)).map(String::as_str).filter(|s| !s.is_empty()).collect();
    while let Some(t) = todo.pop() {
        if let Some(&i) = producer.get(t)
            && stage.insert(i)
        {
            todo.extend(g.node[i].input.iter().map(String::as_str).filter(|s| !s.is_empty()));
        }
    }
    Ok(stage)
}

/// The graph with every MatMul and Gemm in half precision (and every node `halved` names) but those of its input stage
/// (`input_stage`), which stay fp32: float initializers they read converted (a copy; other readers keep the float one),
/// other inputs cast to half before them, and their outputs cast back to float. An input that another of them made is
/// read in half as it was made (the same values: half to float to half is exact), and a cast back that nothing reads is
/// left out. Everything else stays as it was. Returns how many matrix products the input stage keeps in fp32.
pub fn half_matmuls(model: &mut pb::ModelProto) -> Result<usize, EvalError> {
    let stage = input_stage(model).map_err(err)?;
    half_nodes(model, &HashSet::new(), &stage)
}

/// `half_matmuls`, and also the nodes whose first output `also` names; the nodes `stage` holds (by index) stay as they
/// are.
fn half_nodes(model: &mut pb::ModelProto, also: &HashSet<String>, stage: &HashSet<usize>) -> Result<usize, EvalError> {
    let g = model.graph.as_mut().ok_or_else(|| err("a model without a graph"))?;
    let kept = stage
        .iter()
        .filter_map(|&i| g.node.get(i))
        .filter(|n| n.domain.is_empty() && matches!(n.op_type.as_str(), "MatMul" | "Gemm"))
        .count();
    let initializers: HashMap<String, usize> = g
        .initializer
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), i))
        .collect();
    let mut halves: HashMap<String, String> = HashMap::new();
    let mut casts: HashMap<String, String> = HashMap::new();
    // A halved node's float output, and the half one it was cast back from.
    let mut made_half: HashMap<String, String> = HashMap::new();
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
    for (index, mut n) in std::mem::take(&mut g.node).into_iter().enumerate() {
        if stage.contains(&index) {
            nodes.push(n);
            continue;
        }
        // A node that only moves values (Reshape, Transpose, ...) moves a half tensor as it is: the same values.
        let moves = n.domain.is_empty()
            && matches!(n.op_type.as_str(), "Reshape" | "Transpose" | "Squeeze" | "Unsqueeze" | "Identity")
            && n.input.first().is_some_and(|i| made_half.contains_key(i));
        if moves {
            n.input[0] = made_half[&n.input[0]].clone();
            let mut backs = Vec::new();
            for out in n.output.iter_mut().filter(|o| !o.is_empty()) {
                let half = format!("{out}__fp16out");
                backs.push(cast(&half, out, FLOAT));
                made_half.insert(std::mem::replace(out, half.clone()), half);
            }
            nodes.push(n);
            nodes.extend(backs);
            continue;
        }
        if !(halved(&n) || n.output.first().is_some_and(|o| also.contains(o))) {
            nodes.push(n);
            continue;
        }
        for input in n.input.iter_mut() {
            if input.is_empty() {
                continue;
            }
            if let Some(half) = made_half.get(input.as_str()) {
                *input = half.clone();
            } else if let Some(&i) = initializers.get(input.as_str()) {
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
        // A norm's mean and inverse deviation (outputs 1 and 2) stay float at any precision: none may be asked for.
        let norm = n.op_type == "SkipLayerNormalization" || n.op_type == "LayerNormalization";
        if norm && n.output.iter().enumerate().any(|(j, o)| (j == 1 || j == 2) && !o.is_empty()) {
            return Err(err(format!("{}: a norm whose statistics are read", n.name)));
        }
        let mut backs = Vec::new();
        for out in n.output.iter_mut().filter(|o| !o.is_empty()) {
            let half = format!("{out}__fp16out");
            backs.push(cast(&half, out, FLOAT));
            made_half.insert(std::mem::replace(out, half.clone()), half);
        }
        nodes.push(n);
        nodes.extend(backs);
    }
    let read: HashSet<&str> = nodes
        .iter()
        .flat_map(|n| n.input.iter().map(String::as_str))
        .chain(g.output.iter().map(|o| o.name.as_str()))
        .collect();
    let unread: HashSet<String> = made_half
        .keys()
        .filter(|out| !read.contains(out.as_str()))
        .cloned()
        .collect();
    nodes.retain(|n| !(n.op_type == "Cast" && unread.contains(&n.output[0])));
    g.node = nodes;
    for t in added {
        g.initializer.push(t?);
    }
    Ok(kept)
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

/// What the rewrites asked for did to a graph: for each, `None` not asked, `Ok(n)` n places rewritten, `Err(why)` that
/// part left as exported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rewritten {
    pub attention: Option<Result<usize, String>>,
    pub gelus: Option<Result<usize, String>>,
    pub norms: Option<Result<usize, String>>,
    /// At fp16, the input stage (`input_stage`): `Ok(n)` its n matrix products kept fp32; `Err(why)`, nothing halved.
    pub inputs: Option<Result<usize, String>>,
}

impl Rewritten {
    fn parts(&self) -> [(&'static str, &Option<Result<usize, String>>); 3] {
        [("attentions", &self.attention), ("GELUs", &self.gelus), ("norms", &self.norms)]
    }

    /// Whether a rewrite asked for left its part as exported.
    pub fn refused(&self) -> bool {
        self.parts().iter().any(|(_, r)| matches!(r, Some(Err(_)))) || matches!(self.inputs, Some(Err(_)))
    }

    /// In a few words: "8 attentions, 9 GELUs fused", "as exported", or what was refused and why.
    pub fn summary(&self) -> String {
        let fused: Vec<String> = self
            .parts()
            .iter()
            .filter_map(|(what, r)| match r {
                Some(Ok(n)) => Some(format!("{n} {what}")),
                _ => None,
            })
            .collect();
        let mut text = if fused.is_empty() { "as exported".to_string() } else { format!("{} fused", fused.join(", ")) };
        for (what, r) in self.parts() {
            if let Some(Err(why)) = r {
                text.push_str(&format!("; {what} as exported ({why})"));
            }
        }
        match &self.inputs {
            Some(Ok(n)) => text.push_str(&format!("; the input stage fp32 ({n} products)")),
            Some(Err(why)) => text.push_str(&format!("; nothing in half: {why}")),
            None => {}
        }
        text
    }
}

/// A bucket's graph as the engine runs it with `options`, and what the rewrites did to it.
pub fn graph_bytes(path: &Path, options: &Options) -> Result<(Vec<u8>, Rewritten), EvalError> {
    let bytes = std::fs::read(path).map_err(|e| err(format!("{}: {e}", path.display())))?;
    let mut model = pb::ModelProto::decode(&bytes[..]).map_err(|e| err(format!("{}: {e}", path.display())))?;
    narrow_inputs(&mut model).map_err(|e| err(format!("{}: {e}", path.display())))?;
    let mut done = Rewritten::default();
    // The residual rewrite checks its addends' shapes, inferred on the graph before any rewrite.
    let residual = options.precision == Precision::Fp16 && options.fp16 >= Fp16Graph::Residual;
    let shapes = residual.then(|| tensor_shapes(&model));
    if options.precision == Precision::Fp16 {
        if options.fp16 >= Fp16Graph::Attention {
            done.attention = Some(fuse_attention(&mut model).map_err(|e| e.to_string()));
        }
        let mut halve = HashSet::new();
        if options.fp16 >= Fp16Graph::Ffn {
            done.gelus = Some(fuse_gelu(&mut model, options.gelu).map(|g| {
                halve = g.halve;
                g.fused
            }).map_err(|e| e.to_string()));
        }
        if options.fp16 >= Fp16Graph::Residual {
            done.norms = Some(match shapes {
                Some(Ok(shapes)) => fuse_norms(&mut model, &shapes).map_err(|e| e.to_string()),
                Some(Err(e)) => Err(format!("the graph's shapes: {e}")),
                None => Err("no shapes".to_string()),
            });
        }
        // The input stage stays fp32; a graph whose stage cannot be told is not halved at all.
        match input_stage(&model) {
            Ok(stage) => {
                let kept = half_nodes(&mut model, &halve, &stage).map_err(|e| err(format!("{}: {e}", path.display())))?;
                done.inputs = Some(Ok(kept));
            }
            Err(why) => done.inputs = Some(Err(why)),
        }
    }
    Ok((model.encode_to_vec(), done))
}

/// A model directory's graphs (`GRAPH_BUCKETS`' order; `None` for a bucket between the encoding's that it has no graph
/// for) as the engine runs them with `options`, and what the rewrites did to each. Every one of the encoding's
/// `BUCKETS` must have a graph. The encoding is this build's: a model of another schema or encoder hash is refused.
#[allow(clippy::type_complexity)]
fn bucket_graphs(dir: &Path, options: &Options) -> Result<([Option<Vec<u8>>; NB], [Rewritten; NB]), EvalError> {
    let text = std::fs::read(dir.join("manifest.json"))
        .map_err(|e| err(format!("{}: {e}", dir.join("manifest.json").display())))?;
    let manifest: crate::Manifest =
        serde_json::from_slice(&text).map_err(|e| err(format!("{}: {e}", dir.display())))?;
    crate::eval::check_encoding(&manifest).map_err(|e| err(format!("{}: {e}", dir.display())))?;
    let mut bytes: [Option<Vec<u8>>; NB] = Default::default();
    let mut rewritten: [Rewritten; NB] = Default::default();
    for (k, t) in GRAPH_BUCKETS.iter().enumerate() {
        let Some(file) = manifest.buckets.get(&t.to_string()) else {
            if BUCKETS.contains(t) {
                return Err(err(format!("{}: no graph for {t} tokens", dir.display())));
            }
            continue;
        };
        let (b, r) = graph_bytes(&dir.join(file), options)?;
        bytes[k] = Some(b);
        rewritten[k] = r;
    }
    Ok((bytes, rewritten))
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
    pub rows_by_bucket: [u64; NB],
    pub batches_by_bucket: [u64; NB],
    /// Batches by bucket and the shape they ran at: (shape, batches), smallest shape first.
    pub batches_by_shape: [Vec<(usize, u64)>; NB],
    /// Seconds the workers spent writing batches into their pinned buffers, copying them in, in the network itself,
    /// copying the outputs back, and answering the requests, summed: where a batch's time goes.
    pub assemble: f64,
    pub copy_in: f64,
    pub network: f64,
    pub copy_out: f64,
    pub answer: f64,
    /// Rows by bucket and real tokens, in eights: `real_tokens[k][j]` rows of bucket `k` had `8j - 7..=8j` real tokens.
    pub real_tokens: [[u64; MAX_TOKENS / 8 + 1]; NB],
}

impl Stats {
    /// A bucket's rows' real tokens: the mean, and the share at most `at` (a multiple of 8).
    pub fn real_tokens(&self, bucket: usize, at: usize) -> (f64, f64) {
        let h = &self.real_tokens[bucket];
        let rows: u64 = h.iter().sum();
        let tokens: f64 = h.iter().enumerate().map(|(j, &n)| n as f64 * (8 * j) as f64).sum();
        let under: u64 = h[..=at / 8].iter().sum();
        // The mean takes each row at its eight's top: an overestimate of at most 7 tokens.
        (tokens / rows.max(1) as f64, under as f64 / rows.max(1) as f64)
    }
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
    /// Which buckets it has a graph for (`GRAPH_BUCKETS`' order).
    pub graphs: [bool; NB],
    /// What the rewrites did to its graphs, by bucket.
    pub rewritten: [Rewritten; NB],
    /// The shapes each bucket runs batches at, smallest first, the full shape last (`Options::shapes`, the ones the
    /// probe kept); set once the workers have warmed up.
    pub shapes: [std::sync::OnceLock<Vec<usize>>; NB],
    pub schema: u32,
    pub encoder_hash: u64,
    pub input_compatibility: crate::eval::InputCompatibility,
}

impl Served {
    /// The shapes each bucket runs at, as `T96 48/64/96`.
    pub fn shapes_summary(&self) -> String {
        let parts: Vec<String> = (0..NB)
            .filter(|&k| self.graphs[k])
            .map(|k| {
                let shapes = self.shapes[k].get().cloned().unwrap_or_default();
                let shapes: Vec<String> = shapes.iter().map(|n| n.to_string()).collect();
                format!("T{} {}", GRAPH_BUCKETS[k], shapes.join("/"))
            })
            .collect();
        parts.join(", ")
    }

    /// What the rewrites did, in a few words (once if every bucket's is the same).
    pub fn rewritten_summary(&self) -> String {
        let had: Vec<(usize, &Rewritten)> =
            (0..NB).filter(|&k| self.graphs[k]).map(|k| (GRAPH_BUCKETS[k], &self.rewritten[k])).collect();
        let buckets: Vec<String> = had.iter().map(|(t, _)| format!("T{t}")).collect();
        let buckets = buckets.join("/");
        if had.windows(2).all(|w| w[0].1 == w[1].1) {
            return format!("{buckets}: {}", had.first().map_or("no graph".to_string(), |(_, r)| r.summary()));
        }
        let parts: Vec<String> = had.iter().map(|(t, r)| format!("T{t} {}", r.summary())).collect();
        parts.join(" | ")
    }
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

/// A model's bucket at one batch size: its inputs and outputs bound to device buffers (so it can replay a captured CUDA
/// graph), and their pinned host twins the batch is written into and read from. A bucket's shapes share one session,
/// each its own CUDA graph in it (onnxruntime's `gpu_graph_id`), so they share its weights and its memory arena (a
/// session a shape held its own activations, and its memory grew with every shape).
struct Runner {
    session: Arc<Mutex<Session>>,
    /// Which of the session's CUDA graphs this shape replays.
    run_options: ort::session::RunOptions,
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

/// Seconds a run spent copying the inputs in, in the network (the session's run, which waits for it), and copying the
/// outputs back.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunTimes {
    pub copy_in: f64,
    pub network: f64,
    pub copy_out: f64,
}

// SAFETY: the raw pointers are to buffers and a stream the runner owns; a runner is used by one worker at a time.
unsafe impl Send for Runner {}

const INPUTS: [&str; 6] = ["kind", "card", "cats", "ptrs", "nums", "legal"];

impl Runner {
    /// Shape `graph` (counting from 0) of `shared`, a session of the bucket of `t` tokens, at `b` rows.
    fn new(shared: Arc<Mutex<Session>>, graph: usize, options: &Options, t: usize, b: usize) -> Result<Runner, EvalError> {
        use ort::memory::{AllocationDevice, Allocator, AllocatorType, MemoryInfo, MemoryType};
        let mut run_options = ort::session::RunOptions::new().map_err(err)?;
        if options.cuda_graphs {
            run_options.set("gpu_graph_id", graph.to_string()).map_err(err)?;
        }
        let guard = lock(&shared);
        let session = &*guard;
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
        drop(guard);
        let mut r = Runner {
            session: shared,
            run_options,
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
    fn run(&mut self) -> Result<RunTimes, EvalError> {
        let rt = cuda::runtime()?;
        let started = Instant::now();
        for b in &self.inputs {
            // SAFETY: both sides hold `bytes`; the host side is pinned.
            unsafe { rt.copy(b.device_ptr, b.host_ptr, b.bytes, cuda::HOST_TO_DEVICE, self.stream)? };
        }
        rt.synchronize(self.stream)?;
        let copied = Instant::now();
        lock(&self.session).run_binding_with_options(&self.binding, &self.run_options).map_err(err)?;
        let ran = Instant::now();
        for b in &self.outputs {
            // SAFETY: as above, the other way.
            unsafe { rt.copy(b.host_ptr, b.device_ptr, b.bytes, cuda::DEVICE_TO_HOST, self.stream)? };
        }
        rt.synchronize(self.stream)?;
        Ok(RunTimes {
            copy_in: (copied - started).as_secs_f64(),
            network: (ran - copied).as_secs_f64(),
            copy_out: ran.elapsed().as_secs_f64(),
        })
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

/// onnxruntime's own graph rewrites a session at `options` turns off: with `GeluKernel::Gelu`, `BiasGeluFusion`, which
/// would make the half bias Add and `Gelu` the rewrite chose back into `BiasGelu` (half arithmetic).
pub fn disabled_optimizers(options: &Options) -> Option<&'static str> {
    let gelu =
        options.precision == Precision::Fp16 && options.fp16 >= Fp16Graph::Ffn && options.gelu == GeluKernel::Gelu;
    gelu.then_some("BiasGeluFusion")
}

fn session(bytes: &[u8], options: &Options) -> Result<Session, EvalError> {
    let env = crate::eval::ort_environment()?;
    // A fused residual's norm adds and normalises in fp32 only in strict mode (`residual`).
    let strict = options.precision == Precision::Fp16 && options.fp16 >= Fp16Graph::Residual;
    // At fp16 the only fp32 products are the input stage's.
    let tf32 = options.precision == Precision::Tf32 || (options.precision == Precision::Fp16 && options.inputs_tf32);
    let cuda = ort::ep::CUDA::default()
        .with_device_id(options.device)
        .with_tf32(tf32)
        .with_skip_layer_norm_strict_mode(strict)
        .with_cuda_graph(options.cuda_graphs)
        .build()
        .error_on_failure();
    let mut builder = Session::builder(env).map_err(err)?;
    // onnxruntime's profile of every node (a JSON trace a session, written when it ends).
    if let Some(prefix) = std::env::var_os("CANTRIP_ORT_PROFILE") {
        builder = builder.with_profiling(prefix).map_err(|e| err(e.to_string()))?;
    }
    if let Some(off) = disabled_optimizers(options) {
        builder = builder.with_disabled_optimizers(off).map_err(|e| err(e.to_string()))?;
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

/// The tokens row `i` of `rows` (its tokens from `first`) must run with to compute what it does at its encoded bucket
/// (the smallest of `BUCKETS` that holds it): its real tokens, and every token one of them points at. A pointer is
/// looked up as a row of an identity [256, T], so at a smaller bucket than the encoded one a pointer past its real
/// tokens would read zeros where the encoded bucket reads a padding token. A pointer at or past the encoded bucket
/// reads zeros at either, and is no reason to grow. The encoder never points past a row's real tokens; this keeps the
/// row exact if it ever does.
fn tokens_needed(rows: &Rows, i: usize, first: usize) -> usize {
    let len = rows.lengths[i];
    let encoded = BUCKETS.iter().copied().find(|&b| b >= len.max(1)).unwrap_or(MAX_TOKENS);
    let pointed = rows.ptrs[first * PTRS..(first + len) * PTRS]
        .iter()
        .map(|&p| p as usize)
        .filter(|&p| p != NO_POINTER as usize && p < encoded)
        .max()
        .map_or(0, |p| p + 1);
    len.max(pointed)
}

/// The bucket a row needing `len` tokens (`tokens_needed`) runs at: the smallest one its model has a graph for that
/// holds them.
fn bucket_of(graphs: &[bool; NB], len: usize) -> usize {
    (0..NB)
        .find(|&k| graphs[k] && GRAPH_BUCKETS[k] >= len.max(1))
        .unwrap_or(NB - 1)
}

/// The shapes bucket `k` is asked to run at: its `Options::shapes` below its full shape, smallest first, then the full
/// one.
fn shapes_of(options: &Options, k: usize) -> Vec<usize> {
    let full = options.batch[k].max(1);
    let mut out: Vec<usize> = options.shapes[k].iter().copied().filter(|&n| n > 0 && n < full).collect();
    out.sort_unstable();
    out.dedup();
    out.push(full);
    out
}

/// How many rows a bucket's smaller shapes are held to its full shape on, at most.
const PROBE_ROWS: usize = 512;

/// Rows to hold bucket `k`'s shapes to each other on: the model's references that run at it, then, while there are
/// fewer than `PROBE_ROWS`, longer references cut to its tokens (a pointer past them made none), so that a bucket few
/// references run at is still probed on real positions.
fn probe_rows(r: &reference::Reference, graphs: &[bool; NB], k: usize) -> Result<Rows, EvalError> {
    let t = GRAPH_BUCKETS[k];
    let at = |e: &Encoded| bucket_of(graphs, e.len.min(e.bucket));
    let mut chosen: Vec<&Encoded> = r.positions.iter().filter(|e| at(e) == k).take(PROBE_ROWS).collect();
    chosen.extend(r.positions.iter().filter(|e| at(e) > k).take(PROBE_ROWS - chosen.len()));
    let mut out = Rows {
        num_start: vec![0],
        ..Rows::default()
    };
    for e in chosen {
        let one = Rows::from_positions(&[e], &[])?;
        let len = one.lengths[0].min(t);
        out.lengths.push(len);
        out.kind.extend_from_slice(&one.kind[..len]);
        out.card.extend_from_slice(&one.card[..len]);
        out.cats.extend_from_slice(&one.cats[..len * CATS]);
        out.ptrs.extend(one.ptrs[..len * PTRS].iter().map(|&p| if (p as usize) < len { p } else { NO_POINTER }));
        out.legal.extend_from_slice(&one.legal[..len]);
        let base = out.num_feature.len() as u32;
        let end = one.num_start[len] as usize;
        out.num_feature.extend_from_slice(&one.num_feature[..end]);
        out.num_value.extend_from_slice(&one.num_value[..end]);
        out.num_start.extend(one.num_start[1..=len].iter().map(|&n| base + n));
    }
    Ok(out)
}

/// Every output the engine answers with for each of `rows`' rows (its value, aux and real tokens' policy), as bits,
/// run through `runner` a batch of its shape at a time, every slot filled.
fn probe_outputs(runner: &mut Runner, rows: &Rows) -> Result<Vec<Vec<u32>>, EvalError> {
    let mut first = Vec::with_capacity(rows.rows());
    let mut at = 0;
    for &l in &rows.lengths {
        first.push(at);
        at += l;
    }
    let (n, t) = (rows.rows(), runner.t);
    let mut out = vec![Vec::new(); n];
    let mut done = 0;
    while done < n {
        // Every slot a row: the last batch takes its spare slots from the first rows again.
        for s in 0..runner.b {
            let i = (done + s) % n;
            runner.fill(s, Some((rows, i, first[i])));
        }
        runner.used = runner.b;
        runner.run()?;
        for s in 0..runner.b.min(n - done) {
            let i = done + s;
            let mut bits = vec![runner.value()[s].to_bits()];
            bits.extend(runner.aux()[2 * s..2 * s + 2].iter().map(|x| x.to_bits()));
            bits.extend(runner.policy()[s * t..s * t + rows.lengths[i]].iter().map(|x| x.to_bits()));
            out[i] = bits;
        }
        done += runner.b;
    }
    Ok(out)
}

/// Holds a bucket's smaller shapes (`at`, the full shape last) to the full shape on `rows`, and drops each one whose
/// outputs for any row are not bitwise the full shape's. Without rows to hold them to, only the full shape stays.
fn probe(at: &mut Vec<Runner>, rows: Option<&Rows>) -> Result<(), EvalError> {
    let Some(rows) = rows.filter(|r| r.rows() > 0) else {
        at.drain(..at.len() - 1);
        return Ok(());
    };
    let full = probe_outputs(at.last_mut().expect("a full shape"), rows)?;
    let mut keep = Vec::with_capacity(at.len());
    for j in 0..at.len() - 1 {
        keep.push(probe_outputs(&mut at[j], rows)? == full);
    }
    keep.push(true);
    let mut it = keep.into_iter();
    at.retain(|_| it.next().unwrap_or(true));
    // Each slot back to padding, as a runner starts.
    for r in at.iter_mut() {
        for s in 0..r.b {
            r.fill(s, None);
        }
        r.used = 0;
    }
    Ok(())
}

impl Engine {
    /// The models in `dirs`, each bucket's graph on the GPU `options.pipelines` times (once a shape:
    /// `Options::shapes`).
    pub fn start(dirs: &[&Path], options: Options) -> Result<Engine, EvalError> {
        fine_timer();
        let mut models = Vec::new();
        let mut graphs = Vec::new();
        for dir in dirs {
            let (bytes, rewritten) = bucket_graphs(dir, &options)?;
            // The manifest `bucket_graphs` checked: the model's encoding, and how its inputs are adapted to this build's.
            let manifest: crate::Manifest = serde_json::from_slice(
                &std::fs::read(dir.join("manifest.json")).map_err(|e| err(format!("{}: {e}", dir.join("manifest.json").display())))?,
            )
            .map_err(|e| err(format!("{}: {e}", dir.display())))?;
            let has = std::array::from_fn(|k| bytes[k].is_some());
            graphs.push(bytes);
            let served = Served {
                name: dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                dir: dir.to_path_buf(),
                identity: crate::remote::model_identity(dir)?,
                schema: manifest.schema,
                encoder_hash: u64::from_str_radix(&manifest.encoder_hash, 16).map_err(err)?,
                input_compatibility: crate::eval::input_compatibility(&manifest)?,
                graphs: has,
                rewritten,
                shapes: Default::default(),
            };
            if served.rewritten.iter().any(Rewritten::refused) {
                eprintln!("gpu: {}: not every rewrite asked for applies: {}", dir.display(), served.rewritten_summary());
            }
            models.push(served);
        }
        let mut seen = std::collections::HashSet::new();
        if !models.iter().all(|m| seen.insert(m.identity)) {
            return Err(err("the same model twice"));
        }
        let pipelines = options.pipelines.max(1);
        // The runners of each model and bucket it has a graph for, at the queue's key (`model * NB + bucket`): one a
        // shape, smallest first, the full shape last (none where the model has no graph).
        let mut runners: Vec<Vec<Vec<Runner>>> = Vec::new();
        for _ in 0..pipelines {
            let mut mine = Vec::new();
            for bytes in &graphs {
                for (k, t) in GRAPH_BUCKETS.iter().enumerate() {
                    let mut at = Vec::new();
                    if let Some(b) = &bytes[k] {
                        let shared = Arc::new(Mutex::new(session(b, &options)?));
                        for (g, n) in shapes_of(&options, k).into_iter().enumerate() {
                            at.push(Runner::new(shared.clone(), g, &options, *t, n)?);
                        }
                    }
                    mine.push(at);
                }
            }
            runners.push(mine);
        }
        // The rows each smaller shape is held to the full shape's on, by key (only where there is a smaller shape).
        let mut probes: Vec<Option<Rows>> = Vec::new();
        for m in &models {
            let reference = if options.shapes.iter().all(Vec::is_empty) {
                None
            } else {
                reference::Reference::load(&m.dir.join("reference")).ok()
            };
            for k in 0..NB {
                probes.push(match &reference {
                    Some(r) if m.graphs[k] && shapes_of(&options, k).len() > 1 => Some(probe_rows(r, &m.graphs, k)?),
                    _ => None,
                });
            }
        }
        let probes = Arc::new(probes);
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                queues: (0..models.len() * NB).map(|_| Queue::default()).collect(),
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
        // Then each smaller shape is held to the full shape on the bucket's probe rows, and kept only if every output
        // of every row is bitwise the full shape's (`probe`).
        let (ready, warmed) = std::sync::mpsc::channel::<Result<Vec<Vec<usize>>, EvalError>>();
        let go = Arc::new(std::sync::Barrier::new(pipelines + 1));
        let capture = Arc::new(Mutex::new(()));
        let mut workers = Vec::new();
        for (k, mut mine) in runners.into_iter().enumerate() {
            let (shared, ready, go, capture, probes) =
                (shared.clone(), ready.clone(), go.clone(), capture.clone(), probes.clone());
            workers.push(
                std::thread::Builder::new()
                    .name(format!("infer-{k}"))
                    .spawn(move || {
                        raise_thread_priority();
                        let warm = (|| {
                            cuda::runtime()?.use_device(shared.options.device)?;
                            let _one = lock(&capture);
                            for r in mine.iter_mut().flatten() {
                                for _ in 0..3 {
                                    r.run()?;
                                }
                            }
                            for (key, at) in mine.iter_mut().enumerate() {
                                if at.len() > 1 {
                                    probe(at, probes[key].as_ref())?;
                                }
                            }
                            Ok(mine.iter().map(|at| at.iter().map(|r| r.b).collect()).collect())
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
        let mut kept: Option<Vec<Vec<usize>>> = None;
        for _ in 0..pipelines {
            match warmed.recv() {
                Ok(Ok(shapes)) => {
                    // The shapes every worker kept (each worker's runners are its own sessions).
                    kept = Some(match kept {
                        None => shapes,
                        Some(k) => k
                            .into_iter()
                            .zip(shapes)
                            .map(|(a, b)| a.into_iter().filter(|n| b.contains(n)).collect())
                            .collect(),
                    });
                }
                Ok(Err(e)) => failed = failed.or(Some(e)),
                Err(_) => failed = failed.or(Some(err("an inference worker ended while warming up"))),
            }
        }
        if failed.is_some() {
            lock(&shared.state).stopping = true;
        }
        if let Some(kept) = kept {
            if kept.iter().any(|shapes| shapes.len() > 1) {
                eprintln!(
                    "gpu: smaller batch shapes in use (Options::shapes): each passed the probe, but a row's outputs are                      not guaranteed to be the same whatever its batch's size"
                );
            }
            for (key, shapes) in kept.into_iter().enumerate() {
                let (m, k) = (key / NB, key % NB);
                if shared.models[m].graphs[k] {
                    let asked = shapes_of(&shared.options, k);
                    if asked.len() > shapes.len() {
                        let dropped: Vec<String> =
                            asked.iter().filter(|n| !shapes.contains(n)).map(|n| n.to_string()).collect();
                        eprintln!(
                            "gpu: {} T{}: shape(s) {} dropped: not bitwise the full shape's ({}) on the probe rows",
                            shared.models[m].name,
                            GRAPH_BUCKETS[k],
                            dropped.join(", "),
                            shared.options.batch[k]
                        );
                    }
                    let _ = shared.models[m].shapes[k].set(shapes);
                }
            }
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
            "infer: {} requests, {} rows ({:.0} a second) in {} batches ({}), {:.0}% of the padded rows real; \
             workers busy {:.0}%, in the network {:.0}%; a request {:.1} ms in the engine, a row group queued {:.1} ms; \
             {} clients",
            s.requests,
            s.rows,
            s.rows as f64 / seconds,
            s.batches,
            (0..NB)
                .map(|k| {
                    let shapes = &s.batches_by_shape[k];
                    let at = if shapes.len() > 1 {
                        let each: Vec<String> = shapes.iter().map(|(n, c)| format!("{c} at {n}")).collect();
                        format!(" ({})", each.join(", "))
                    } else {
                        String::new()
                    };
                    format!("T{}: {} rows in {}{at}", GRAPH_BUCKETS[k], s.rows_by_bucket[k], s.batches_by_bucket[k])
                })
                .collect::<Vec<_>>()
                .join(", "),
            100.0 * s.rows as f64 / s.padded.max(1) as f64,
            100.0 * s.busy / seconds,
            100.0 * s.gpu / seconds,
            1e3 * s.latency / s.requests.max(1) as f64,
            1e3 * s.queued / s.groups.max(1) as f64,
            st.clients
        ) + &{
            let per = |x: f64| 1e3 * x / s.batches.max(1) as f64;
            let means: Vec<String> = (0..NB)
                .filter(|&k| s.rows_by_bucket[k] > 0)
                .map(|k| format!("T{} {:.0}", GRAPH_BUCKETS[k], s.real_tokens(k, GRAPH_BUCKETS[k]).0))
                .collect();
            let (_, under) = s.real_tokens(NB - 1, 96);
            format!(
                "; a batch: assembled {:.2} ms, copied in {:.2} ms, network {:.2} ms, copied out {:.2} ms, answered {:.2} ms; \
                 real tokens a row by bucket (at most, by eights): {}; T128 rows at most 96: {:.0}%",
                per(s.assemble),
                per(s.copy_in),
                per(s.network),
                per(s.copy_out),
                per(s.answer),
                means.join(", "),
                100.0 * under
            )
        }
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

fn evaluate(shared: &Shared, model: usize, mut rows: Rows) -> Result<RowOutputs, EvalError> {
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
    let compatibility = shared.models[model].input_compatibility;
    for (token, &kind) in rows.kind.iter().enumerate() {
        for slot in 0..CATS {
            let at = token * CATS + slot;
            rows.cats[at] = compatibility.category(slot, rows.cats[at]);
        }
        for at in rows.num_start[token] as usize..rows.num_start[token + 1] as usize {
            if compatibility.masks(kind, rows.num_feature[at] as usize) {
                rows.num_value[at] = 0.0;
            }
        }
    }
    let tokens = at;
    let mut by_bucket: [Vec<usize>; NB] = Default::default();
    let graphs = &shared.models[model].graphs;
    for i in 0..n {
        by_bucket[bucket_of(graphs, tokens_needed(&rows, i, first[i]))].push(i);
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
            let q = &mut st.queues[model * NB + k];
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
        if q.rows >= options.batch[key % NB].max(1) {
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
    let room = options.batch[key % NB].max(1);
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

fn work(shared: &Shared, mut runners: Vec<Vec<Runner>>) {
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
        // A queue only gets rows for a bucket its model has a graph for (`bucket_of`), so its runners exist. The batch
        // runs at the smallest shape that holds it.
        let used = batch.len();
        let Some(runner) = runners[key].iter_mut().find(|r| r.b >= used) else {
            let mut st = lock(&shared.state);
            st.running -= 1;
            let mut finished = 0;
            for (r, _) in &batch {
                let mut p = lock(&r.out);
                if p.error.is_none() {
                    p.error = Some(format!("no graph for bucket {}", GRAPH_BUCKETS[key % NB]));
                }
                if !p.answered {
                    p.answered = true;
                    finished += 1;
                    r.done.notify_all();
                }
            }
            st.waiting -= finished;
            drop(st);
            shared.work.notify_all();
            continue;
        };
        let mut real = [0u64; MAX_TOKENS / 8 + 1];
        for (s, (r, i)) in batch.iter().enumerate() {
            runner.fill(s, Some((&r.rows, *i, r.first[*i])));
            real[r.rows.lengths[*i].div_ceil(8).min(MAX_TOKENS / 8)] += 1;
        }
        for s in used..runner.used {
            runner.fill(s, None);
        }
        runner.used = used;
        let ran = Instant::now();
        let result = runner.run();
        let gpu = ran.elapsed().as_secs_f64();
        let times = result.as_ref().copied().unwrap_or_default();
        let mut finished = 0;
        let t = runner.t;
        for (s, (r, i)) in batch.iter().enumerate() {
            let mut p = lock(&r.out);
            match &result {
                Ok(_) => {
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
        let answered = (ran.elapsed().as_secs_f64() - gpu).max(0.0);
        let mut st = lock(&shared.state);
        st.running -= 1;
        st.waiting -= finished;
        let s = &mut st.stats;
        s.batches += 1;
        s.padded += runner.b as u64;
        s.gpu += gpu;
        s.busy += started.elapsed().as_secs_f64();
        s.assemble += (ran - started).as_secs_f64();
        s.copy_in += times.copy_in;
        s.network += times.network;
        s.copy_out += times.copy_out;
        s.answer += answered;
        for (k, n) in real.into_iter().enumerate() {
            s.real_tokens[key % NB][k] += n;
        }
        s.rows_by_bucket[key % NB] += used as u64;
        s.batches_by_bucket[key % NB] += 1;
        let by_shape = &mut s.batches_by_shape[key % NB];
        match by_shape.iter_mut().find(|(n, _)| *n == runner.b) {
            Some((_, c)) => *c += 1,
            None => {
                by_shape.push((runner.b, 1));
                by_shape.sort_unstable();
            }
        }
        drop(st);
        shared.work.notify_all();
    }
}

/// What `time_bucket` measured.
#[derive(Clone, Debug)]
pub struct Timing {
    pub rows_per_second: f64,
    /// What the rewrites did to the bucket's graph.
    pub rewritten: Rewritten,
    /// Whether a batch slot holding no position (every key padding) still gives finite outputs.
    pub padding_finite: bool,
    /// Seconds a batch spends, on average: written into the pinned buffers from the rows (as a worker assembles it),
    /// and in the run (copies and network).
    pub assemble: f64,
    pub run: RunTimes,
}

/// The network alone on one stream: one session of `dir`'s bucket of `t` tokens at `batch` rows on this thread, every
/// slot a reference position the engine would run at that bucket, run back to back for `seconds` (each batch copied
/// in, run and copied out, as a pipeline runs it). Then once more with the last slot all padding.
pub fn time_bucket(dir: &Path, options: &Options, t: usize, batch: usize, seconds: f64) -> Result<Timing, EvalError> {
    let k = GRAPH_BUCKETS.iter().position(|&b| b == t).ok_or_else(|| err(format!("no bucket of {t} tokens")))?;
    let (graphs, rewritten) = bucket_graphs(dir, options)?;
    let has = std::array::from_fn(|k| graphs[k].is_some());
    let graph = graphs[k].as_ref().ok_or_else(|| err(format!("{}: no graph for {t} tokens", dir.display())))?;
    let r = reference::Reference::load(&dir.join("reference"))?;
    let fit: Vec<&Encoded> = r.positions.iter().filter(|e| bucket_of(&has, e.len.min(e.bucket)) == k).collect();
    if fit.is_empty() {
        return Err(err(format!("{}: no reference of {t} tokens", dir.display())));
    }
    let rows = Rows::from_positions(&fit, &[])?;
    let mut first = Vec::with_capacity(rows.rows());
    let mut at = 0;
    for &l in &rows.lengths {
        first.push(at);
        at += l;
    }
    cuda::runtime()?.use_device(options.device)?;
    let mut runner = Runner::new(Arc::new(Mutex::new(session(graph, options)?)), 0, options, t, batch)?;
    for s in 0..batch {
        let i = s % rows.rows();
        runner.fill(s, Some((&rows, i, first[i])));
    }
    runner.used = batch;
    for _ in 0..3 {
        runner.run()?;
    }
    let started = Instant::now();
    let mut runs = 0usize;
    let mut run = RunTimes::default();
    while started.elapsed().as_secs_f64() < seconds {
        let r = runner.run()?;
        run.copy_in += r.copy_in;
        run.network += r.network;
        run.copy_out += r.copy_out;
        runs += 1;
    }
    let rows_per_second = (runs * batch) as f64 / started.elapsed().as_secs_f64();
    let per = runs.max(1) as f64;
    let run = RunTimes { copy_in: run.copy_in / per, network: run.network / per, copy_out: run.copy_out / per };
    // Assembling a batch: every slot written again from the rows (a slot first put back to padding, as a slot whose
    // last row was longer is), the CPU work a worker does before each run.
    let refills = 20;
    let filled = Instant::now();
    for k in 0..refills {
        for s in 0..batch {
            let i = (s + k + 1) % rows.rows();
            runner.fill(s, Some((&rows, i, first[i])));
        }
    }
    let assemble = filled.elapsed().as_secs_f64() / refills as f64;
    runner.fill(batch - 1, None);
    runner.run()?;
    let last = batch - 1;
    let padding_finite = runner.value()[last].is_finite()
        && runner.aux()[2 * last..2 * last + 2].iter().all(|x| x.is_finite())
        && runner.policy()[last * t..(last + 1) * t].iter().all(|x| x.is_finite());
    Ok(Timing {
        rows_per_second,
        rewritten: rewritten[k].clone(),
        padding_finite,
        assemble,
        run,
    })
}

// --- In-process evaluation -----------------------------------------------------------------------------------------

/// A model evaluated by an engine in this process (self-play's `--backend gpu`): a lane an evaluator, all of them
/// sharing one engine, so every lane's rows are batched together without leaving the process.
pub struct GpuEvaluator {
    engine: Arc<Engine>,
    model: usize,
    remap: Vec<u32>,
    /// Counted as the engine's client while it evaluates; none while idle (`Evaluator::idle`).
    client: Mutex<Option<Client>>,
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
            client: Mutex::new(Some(client)),
        })
    }

    /// Counted as a client again, if it was idle.
    fn active(&self) {
        let mut client = lock(&self.client);
        if client.is_none() {
            *client = Some(self.engine.client());
        }
    }
}

impl Evaluator for GpuEvaluator {
    fn evaluate(&self, batch: &FeatureBatch) -> Result<Outputs, EvalError> {
        if batch.rows == 0 {
            return Ok(Outputs::default());
        }
        self.active();
        let rows = Rows::from_batch(batch, &self.remap)?;
        let lengths = rows.lengths.clone();
        let out = self.engine.evaluate(self.model, rows)?;
        Ok(out.outputs(&lengths, batch.options.iter().copied()))
    }

    fn evaluate_positions(&self, positions: &[&Encoded]) -> Result<Outputs, EvalError> {
        if positions.is_empty() {
            return Ok(Outputs::default());
        }
        self.active();
        let rows = Rows::from_positions(positions, &self.remap)?;
        let lengths = rows.lengths.clone();
        let out = self.engine.evaluate(self.model, rows)?;
        Ok(out.outputs(&lengths, positions.iter().map(|e| (e.first_option, e.options))))
    }

    /// No longer the engine's client until it next evaluates: a partial batch runs once every active client waits.
    fn idle(&self) {
        lock(&self.client).take();
    }
}

// --- Checking the references ---------------------------------------------------------------------------------------

pub use crate::eval::reference::{
    PolicyDrift, REDUCED_SWING, REDUCED_TV_MAX, REDUCED_TV_MEAN, REDUCED_TV_P99, REDUCED_VALUE, SAME_ARGMAX,
    SWING_RELATIVE, TIE, TOLERANCE, softmax,
};

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
    /// Over the legal options' probabilities: the largest change of one (shown; the reduced bar is `tv_*`).
    pub probability: f32,
    /// The policy's total-variation distance from PyTorch's (`PolicyDrift`): mean, 99th percentile, largest.
    pub tv_mean: f64,
    pub tv_p99: f64,
    pub tv_max: f64,
    /// Best options changed where PyTorch's top two are `TIE` apart or more.
    pub decided_flips: usize,
    pub same_argmax: f64,
    /// References with an output that is not finite (the value, either aux output, or a legal option's logit): the
    /// drifts above skip a NaN, so these are counted apart, and any one fails the check.
    pub non_finite: usize,
    /// References with a legal option whose best option is none of them (every legal logit masked or NaN), as
    /// `reference::parity` counts them: any one fails the check.
    pub illegal_argmax: usize,
    pub pass: bool,
}

/// A model's references (`reference/`, PyTorch's outputs at fp32) through the engine, held to export's tolerance at
/// fp32 and to the reduced bar below it (`score`).
pub fn check(engine: &Arc<Engine>, model: usize) -> Result<Check, EvalError> {
    let dir = engine.models()[model].dir.clone();
    let r = reference::Reference::load(&dir.join("reference"))?;
    let own = CardIndex::read(&dir.join("cards.json")).map_err(err)?;
    let e = GpuEvaluator::new(engine.clone(), model, &own)?;
    let mut all = Outputs::default();
    for chunk in r.positions.chunks(64) {
        let es: Vec<_> = chunk.iter().collect();
        let out = e.evaluate(&FeatureBatch::new(&es))?;
        all.value.extend(out.value);
        all.aux.extend(out.aux);
        all.policy.extend(out.policy);
    }
    Ok(score(&r, &all, engine.options().precision))
}

/// The references' outputs `out` (in the references' order, as `Evaluator::evaluate` gives them) held to the
/// references: export's tolerance at fp32, the reduced bar below it; at either, every output finite and every
/// reference with a legal option giving one of them its best.
pub fn score(r: &reference::Reference, out: &Outputs, precision: Precision) -> Check {
    let mut c = Check {
        positions: r.positions.len(),
        ..Check::default()
    };
    let mut same = 0usize;
    let mut drift = PolicyDrift::default();
    for (i, p) in r.positions.iter().enumerate() {
        let legal: Vec<usize> = (0..p.options).filter(|&k| p.legal[p.first_option + k]).collect();
        let finite = out.value[i].is_finite()
            && out.aux[i].iter().all(|x| x.is_finite())
            && legal.iter().all(|&k| out.policy[i][k].is_finite());
        c.non_finite += !finite as usize;
        if !legal.is_empty() && !out.argmax(i).is_some_and(|k| legal.contains(&k)) {
            c.illegal_argmax += 1;
        }
        c.value = c.value.max((out.value[i] - r.value[i]).abs());
        let d = (out.aux[i][0] - r.aux[i][0]).abs();
        c.swing = c.swing.max(d);
        c.swing_reduced = c.swing_reduced.max(d / (r.aux[i][0].abs() / SWING_RELATIVE).max(1.0));
        c.dead = c.dead.max((out.aux[i][1] - r.aux[i][1]).abs());
        if legal.is_empty() {
            same += 1;
            continue;
        }
        let got: Vec<f32> = legal.iter().map(|&k| out.policy[i][k]).collect();
        let want: Vec<f32> = legal.iter().map(|&k| r.policy[i][k]).collect();
        for (a, b) in got.iter().zip(&want) {
            c.logits = c.logits.max((a - b).abs());
        }
        for (a, b) in softmax(&got).iter().zip(softmax(&want)) {
            c.probability = c.probability.max((a - b).abs() as f32);
        }
        drift.add(&got, &want);
        let best = |x: &[f32]| (0..x.len()).max_by(|&a, &b| x[a].total_cmp(&x[b]).then(b.cmp(&a)));
        same += (best(&got) == best(&want)) as usize;
    }
    c.same_argmax = same as f64 / c.positions.max(1) as f64;
    (c.tv_mean, c.tv_p99, c.tv_max, c.decided_flips) = (drift.mean(), drift.p99(), drift.max(), drift.decided_flips);
    c.pass = c.non_finite == 0
        && c.illegal_argmax == 0
        && c.same_argmax >= SAME_ARGMAX
        && if precision == Precision::Fp32 {
            c.value < TOLERANCE && c.swing_reduced < TOLERANCE && c.dead < TOLERANCE && c.logits < TOLERANCE
        } else {
            c.value < REDUCED_VALUE
                && c.dead < REDUCED_VALUE
                && drift.within_reduced()
                && c.swing_reduced < REDUCED_SWING
        };
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_runs_at_the_smallest_bucket_its_model_has_a_graph_for() {
        let all = [true; NB];
        let before_t96 = [true, true, false, true];
        let at = |graphs: &[bool; NB], len| GRAPH_BUCKETS[bucket_of(graphs, len)];
        assert_eq!([0, 1, 32, 33, 64, 65, 96, 97, 128].map(|l| at(&all, l)), [32, 32, 32, 64, 64, 96, 96, 128, 128]);
        assert_eq!([64, 65, 96, 97, 128].map(|l| at(&before_t96, l)), [64, 128, 128, 128, 128]);
    }

    /// One row of `len` tokens whose first token points at `p`.
    fn row(len: usize, p: u8) -> Rows {
        let mut ptrs = vec![NO_POINTER; len * PTRS];
        ptrs[0] = p;
        Rows {
            lengths: vec![len],
            kind: vec![1; len],
            card: vec![0; len],
            cats: vec![0; len * CATS],
            ptrs,
            legal: vec![false; len],
            num_start: vec![0; len + 1],
            ..Rows::default()
        }
    }

    #[test]
    fn a_row_pointing_past_its_tokens_runs_where_the_pointer_reads_what_it_read() {
        let all = [true; NB];
        let at = |len, p| GRAPH_BUCKETS[bucket_of(&all, tokens_needed(&row(len, p), 0, 0))];
        assert_eq!(at(70, 3), 96);
        assert_eq!(at(70, NO_POINTER), 96);
        // Encoded at 128: a pointer at 80 reads a padding token at 96 as at 128; one at 100 needs 128.
        assert_eq!(at(70, 80), 96);
        assert_eq!(at(70, 100), 128);
        // Encoded at 64: a pointer at 70 reads zeros there, as at 64, and moves nothing.
        assert_eq!(at(40, 50), 64);
        assert_eq!(at(40, 70), 64);
    }

    #[test]
    fn a_batch_list_from_before_t96_still_reads() {
        assert_eq!(parse_batch("32:200,64:100,128:50").unwrap(), [200, 100, Options::default().batch[2], 50]);
        assert_eq!(parse_batch("16").unwrap(), [16; NB]);
        assert!(parse_batch("48:10").is_err());
        assert!(parse_batch("96:0").is_err());
    }

    #[test]
    fn a_bucket_runs_at_its_smaller_shapes_then_its_full_one() {
        let every = |v: Vec<usize>| -> [Vec<usize>; NB] { std::array::from_fn(|_| v.clone()) };
        assert_eq!(parse_shapes("64,48,48").unwrap(), every(vec![48, 64]));
        assert_eq!(parse_shapes("none").unwrap(), every(vec![]));
        assert_eq!(parse_shapes("96:64/48,128:48").unwrap(), [vec![], vec![], vec![48, 64], vec![48]]);
        assert!(parse_shapes("0").is_err() && parse_shapes("48,x").is_err() && parse_shapes("40:8").is_err());
        let o = Options {
            batch: [256, 128, 96, 64],
            shapes: every(vec![48, 64, 96, 200]),
            ..Options::default()
        };
        // A shape at or above the full one is not a smaller shape.
        assert_eq!(shapes_of(&o, 0), [48, 64, 96, 200, 256]);
        assert_eq!(shapes_of(&o, 2), [48, 64, 96]);
        assert_eq!(shapes_of(&o, 3), [48, 64]);
        assert_eq!(shapes_of(&Options::default(), 3), [48]);
    }

    #[test]
    fn a_bucket_s_probe_rows_are_rows_of_its_tokens() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny/reference");
        let r = reference::Reference::load(&dir).unwrap();
        for k in 0..NB {
            let rows = probe_rows(&r, &[true; NB], k).unwrap();
            // The references that run at the bucket or a larger one.
            let fit = r.positions.iter().filter(|e| bucket_of(&[true; NB], e.len.min(e.bucket)) >= k).count();
            assert!(fit > 0);
            assert_eq!(rows.rows(), PROBE_ROWS.min(fit), "T{}", GRAPH_BUCKETS[k]);
            let tokens: usize = rows.lengths.iter().sum();
            assert!(rows.lengths.iter().all(|&l| l <= GRAPH_BUCKETS[k]));
            assert_eq!((rows.kind.len(), rows.ptrs.len(), rows.num_start.len()), (tokens, tokens * PTRS, tokens + 1));
            assert_eq!(*rows.num_start.last().unwrap() as usize, rows.num_value.len());
            let mut at = 0;
            for &l in &rows.lengths {
                let ptrs = &rows.ptrs[at * PTRS..(at + l) * PTRS];
                assert!(ptrs.iter().all(|&p| p == NO_POINTER || (p as usize) < l), "a pointer past the row's tokens");
                at += l;
            }
        }
    }
}
