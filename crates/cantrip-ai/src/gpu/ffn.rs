//! The feed-forward networks' GELU in half precision, for the GPU engine at fp16 (`Fp16Graph::Ffn`).
//!
//! The exported graph writes each `nn.GELU()` out as erf's, `x * (1 + erf(x / √2)) * 0.5` (Div, Erf, Add, Mul, Mul), and
//! the Linear before it as a MatMul and a bias Add. At fp16 the MatMul's output is half, and today it is cast to float,
//! biased, put through the five nodes and cast back to half for the next MatMul: the widest tensor of the network
//! ([B, T, ffn]) crosses memory in fp32 seven times. `fuse_gelu` makes each GELU fed by a MatMul one node that reads and
//! writes half, as `GeluKernel` says:
//! - `BiasGelu`: onnxruntime's `BiasGelu(y, bias)`, one kernel. Its half kernel adds the bias in half and computes erf's
//!   GELU in half arithmetic around an fp32 erf: a few roundings to half where PyTorch's autocast makes one.
//! - `Gelu`: the bias Add in half (one rounding), then onnxruntime's `Gelu`, whose half kernel computes erf's GELU in
//!   fp32 and rounds once: autocast's numerics, in two kernels. onnxruntime's own `BiasGeluFusion` would make the pair
//!   a `BiasGelu` again, so the engine's session turns it off for this kernel (`gpu::disabled_optimizers`).
//!
//! Both are erf's GELU, never the tanh approximation. A GELU fed by anything else (the value and aux heads' Gemms) stays
//! as exported. Every Erf of the graph must be a GELU found whole, its constants exactly √2, 1 and ½, every tensor inside
//! it read by the next node alone; otherwise nothing is rewritten and the reason is returned.

use std::collections::{HashMap, HashSet};

use tract_onnx::pb;

use super::err;
use super::graph::{Index, MICROSOFT, describe, import_microsoft, input, node, output, splice};
use crate::eval::EvalError;

/// How a fused GELU runs (see the module's documentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeluKernel {
    BiasGelu,
    Gelu,
}

impl GeluKernel {
    pub fn parse(s: &str) -> Option<GeluKernel> {
        match s {
            "biasgelu" => Some(GeluKernel::BiasGelu),
            "gelu" => Some(GeluKernel::Gelu),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            GeluKernel::BiasGelu => "biasgelu",
            GeluKernel::Gelu => "gelu",
        }
    }
}

/// What `fuse_gelu` did: how many GELUs it fused, and the nodes (by first output) the half pass must also run in half
/// (`GeluKernel::Gelu`'s bias Adds).
#[derive(Clone, Debug, Default)]
pub struct Gelus {
    pub fused: usize,
    pub halve: HashSet<String>,
}

/// One GELU found: its nodes from the Div on, its input, and the output it makes.
struct Found {
    nodes: Vec<usize>,
    last: usize,
    input: String,
    out: String,
    /// A MatMul's output and the bias added to it, when the GELU's input is that sum read by the GELU alone.
    biased: Option<(usize, String, String)>,
}

fn gelu(ix: &Index, e: usize) -> Result<Found, String> {
    let erf = ix.node(e);
    let scaled = input(erf, 0)?;
    let div = ix.made_by(scaled, "Div")?;
    ix.only_reader(scaled)?;
    let d = ix.node(div);
    let x = input(d, 0)?;
    if ix.scalar(input(d, 1)?) != Some(std::f32::consts::SQRT_2) {
        return Err("x is not divided by √2".into());
    }
    let erf_out = output(erf, 0)?;
    let plus = ix.only_reader(erf_out)?;
    let p = ix.node(plus);
    if p.op_type != "Add" || !p.domain.is_empty() {
        return Err("erf is not added to 1".into());
    }
    let one = if input(p, 0)? == erf_out { input(p, 1)? } else { input(p, 0)? };
    if ix.scalar(one) != Some(1.0) {
        return Err("erf is not added to 1".into());
    }
    let p_out = output(p, 0)?;
    let times = ix.only_reader(p_out)?;
    let m = ix.node(times);
    let (m0, m1) = (input(m, 0)?, input(m, 1)?);
    if m.op_type != "Mul" || !m.domain.is_empty() || !((m0 == x && m1 == p_out) || (m0 == p_out && m1 == x)) {
        return Err("x is not multiplied by 1 + erf".into());
    }
    let m_out = output(m, 0)?;
    let half = ix.only_reader(m_out)?;
    let h = ix.node(half);
    if h.op_type != "Mul" || !h.domain.is_empty() {
        return Err("the product is not halved".into());
    }
    let c = if input(h, 0)? == m_out { input(h, 1)? } else { input(h, 0)? };
    if ix.scalar(c) != Some(0.5) {
        return Err("the product is not halved".into());
    }
    // x is read by the Div and the Mul alone.
    let mut readers = ix.consumers.get(x).cloned().unwrap_or_default();
    readers.sort_unstable();
    let mut want = vec![div, times];
    want.sort_unstable();
    if readers != want || ix.outputs.contains(x) {
        return Err(format!("{x} is read by more than the GELU"));
    }
    let biased = ix.made_by(x, "Add").ok().and_then(|add| {
        let a = ix.node(add);
        let (a0, a1) = (input(a, 0).ok()?, input(a, 1).ok()?);
        let is_bias = |t: &str| ix.initializers.get(t).is_some_and(|t| t.dims.len() == 1 && t.data_type == super::FLOAT);
        let (y, bias) = match (is_bias(a0), is_bias(a1)) {
            (false, true) => (a0, a1),
            (true, false) => (a1, a0),
            _ => return None,
        };
        let mm = ix.made_by(y, "MatMul").ok()?;
        // The bias is the MatMul's last dimension's: the weight's columns.
        let w = ix.initializers.get(input(ix.node(mm), 1).ok()?)?;
        let width = *w.dims.last()?;
        (ix.initializers[bias].dims == [width]).then(|| (add, y.to_string(), bias.to_string()))
    });
    Ok(Found {
        nodes: vec![div, e, plus, times, half],
        last: half,
        input: x.to_string(),
        out: output(h, 0)?.to_string(),
        biased,
    })
}

/// The graph with every GELU fed by a MatMul fused (see the module's documentation). On a graph whose GELUs do not
/// match the pattern, or that has none fed by a MatMul, an error saying why, and the graph untouched.
pub fn fuse_gelu(model: &mut pb::ModelProto, kernel: GeluKernel) -> Result<Gelus, EvalError> {
    let g = model.graph.as_ref().ok_or_else(|| err("a model without a graph"))?;
    let ix = Index::new(g);
    let mut found = Vec::new();
    for (i, n) in g.node.iter().enumerate() {
        if n.op_type == "Erf" {
            found.push(gelu(&ix, i).map_err(|e| err(format!("GELU at {}: {e}", describe(n))))?);
        }
    }
    let mut removed = HashSet::new();
    let mut after: HashMap<usize, Vec<pb::NodeProto>> = HashMap::new();
    let mut gelus = Gelus::default();
    for f in &found {
        let Some((add, y, bias)) = &f.biased else { continue };
        let fused = match kernel {
            GeluKernel::BiasGelu => {
                removed.insert(*add);
                node("BiasGelu", MICROSOFT, &[y, bias], &f.out, vec![])
            }
            GeluKernel::Gelu => {
                gelus.halve.insert(f.input.clone());
                node("Gelu", MICROSOFT, &[&f.input], &f.out, vec![])
            }
        };
        if !f.nodes.iter().all(|&i| removed.insert(i)) {
            return Err(err(format!("two GELUs share a node ({})", f.out)));
        }
        after.entry(f.last).or_default().push(fused);
        gelus.fused += 1;
    }
    if gelus.fused == 0 {
        return Err(err("no GELU fed by a MatMul"));
    }
    let rewritten = splice(g, &removed, after, Vec::new(), &HashSet::new());
    model.graph = Some(rewritten);
    import_microsoft(model);
    Ok(gelus)
}
