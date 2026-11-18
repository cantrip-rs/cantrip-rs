//! The residual stream in half precision, for the GPU engine at fp16 (`Fp16Graph::Residual`).
//!
//! The exported graph adds each sublayer's output to the stream (`x + out(...)`: the projection's MatMul, its bias Add
//! and the residual Add) and normalises the sum for what comes next (LayerNormalization). `fuse_norms` makes each such
//! residual Add and the norm that reads it one `com.microsoft::SkipLayerNormalization(input, skip, gamma, beta, bias)`:
//! the projection's product, the stream, the norm's weights and the projection's bias. It gives the norm's output and
//! (where anything else reads the stream) the sum, the next stream. At fp16 both are half: the stream is rounded to half
//! once a sublayer, where the plain fp16 path keeps it in fp32. The engine runs the node in onnxruntime's strict mode
//! (`enable_skip_layer_norm_strict_mode`), whose kernel adds the three in fp32 and normalises the fp32 sum with fp32
//! statistics (the default kernel accumulates the statistics in half).
//!
//! Every LayerNormalization must normalise the last axis of a residual Add of two computed tensors of one shape [B, T,
//! width] (as tract infers the graph's shapes, `Shapes`), with float weights; otherwise nothing is rewritten and the
//! reason is returned. The fused node's epsilon is the norm's (ONNX's 1e-5 where it gives none; the fused node's own
//! default is 1e-12).

use std::collections::{HashMap, HashSet};

use tract_onnx::pb;

use super::err;
use super::graph::{Index, MICROSOFT, Shapes, attr_int, describe, import_microsoft, input, node, output, splice};
use crate::eval::EvalError;

struct Found {
    nodes: Vec<usize>,
    last: usize,
    fused: pb::NodeProto,
}

/// ONNX's LayerNormalization epsilon where a node gives none.
const EPSILON: f32 = 1e-5;

fn norm(ix: &Index, shapes: &Shapes, l: usize) -> Result<Found, String> {
    let ln = ix.node(l);
    if attr_int(ln, "axis").unwrap_or(-1) != -1 || attr_int(ln, "stash_type").unwrap_or(1) != 1 {
        return Err("not a norm of the last axis in fp32".into());
    }
    if ln.output.iter().skip(1).any(|o| !o.is_empty()) {
        return Err("its statistics are read".into());
    }
    let (x, gamma, out) = (input(ln, 0)?, input(ln, 1)?, output(ln, 0)?);
    let beta = ln.input.get(2).map_or("", String::as_str);
    let Some(width) = ix.initializers.get(gamma).filter(|t| t.dims.len() == 1 && t.data_type == super::FLOAT) else {
        return Err("its scale is no float vector".into());
    };
    let width = width.dims[0];
    if !beta.is_empty() && !ix.initializers.get(beta).is_some_and(|t| t.dims == [width] && t.data_type == super::FLOAT)
    {
        return Err("its bias is no float vector of its width".into());
    }
    let residual = ix.made_by(x, "Add")?;
    let r = ix.node(residual);
    let (p, q) = (input(r, 0)?, input(r, 1)?);
    // Both addends computed (a constant or an input would be broadcast, which the fused node does not do).
    if !(ix.producer.contains_key(p) && ix.producer.contains_key(q)) || ix.constants.contains_key(p) || ix.constants.contains_key(q)
    {
        return Err(format!("{x} is not the sum of two computed tensors"));
    }
    // Both addends [B, T, width] and of one shape: the fused node adds them element by element, without broadcasting.
    let shape = |t: &str| shapes.get(t).cloned().flatten();
    let token_shaped = |t: &str| shape(t).filter(|d| d.len() == 3 && d[2] == width.to_string());
    if token_shaped(p).is_none() || token_shaped(p) != token_shaped(q) {
        let (a, b) = (shape(p), shape(q));
        return Err(format!("{x} is not the sum of two [B, T, {width}] tensors of one shape ({a:?}, {b:?})"));
    }
    let mut nodes = vec![residual, l];
    // A projection's bias Add, read by the residual Add alone, goes into the fused node.
    let biased = |t: &str| -> Option<(usize, String, String)> {
        let add = ix.made_by(t, "Add").ok()?;
        if ix.only_reader(t).ok()? != residual {
            return None;
        }
        let a = ix.node(add);
        let (a0, a1) = (input(a, 0).ok()?, input(a, 1).ok()?);
        let is_bias = |t: &str| ix.initializers.get(t).is_some_and(|t| t.dims == [width] && t.data_type == super::FLOAT);
        let (y, bias) = match (is_bias(a0), is_bias(a1)) {
            (false, true) => (a0, a1),
            (true, false) => (a1, a0),
            _ => return None,
        };
        ix.made_by(y, "MatMul").ok()?;
        // The product the bias is added to is itself the addend's shape (the bias broadcasts only over B and T).
        (token_shaped(y)? == token_shaped(t)?).then(|| (add, y.to_string(), bias.to_string()))
    };
    let (input_, skip, bias) = if let Some((add, y, b)) = biased(p) {
        nodes.push(add);
        (y, q.to_string(), b)
    } else if let Some((add, y, b)) = biased(q) {
        nodes.push(add);
        (y, p.to_string(), b)
    } else {
        (p.to_string(), q.to_string(), String::new())
    };
    // The sum is given only where something besides the norm reads it, and the fused node takes the norm's place, so
    // nothing may read the sum before the norm.
    let readers = ix.consumers.get(x).map(Vec::as_slice).unwrap_or_default();
    if readers.iter().any(|&i| i < l) {
        return Err(format!("{x} is read before its norm"));
    }
    let others = readers.iter().any(|&i| i != l) || ix.outputs.contains(x);
    let mut fused = node(
        "SkipLayerNormalization",
        MICROSOFT,
        &[&input_, &skip, gamma, beta, &bias],
        out,
        vec![pb::AttributeProto {
            name: "epsilon".into(),
            r#type: pb::attribute_proto::AttributeType::Float as i32,
            f: ln.attribute.iter().find(|a| a.name == "epsilon").map_or(EPSILON, |a| a.f),
            ..Default::default()
        }],
    );
    while fused.input.last().is_some_and(String::is_empty) {
        fused.input.pop();
    }
    if others {
        fused.output.extend([String::new(), String::new(), x.to_string()]);
    }
    Ok(Found { nodes, last: l, fused })
}

/// The graph with every residual Add and its norm fused (see the module's documentation): how many. `shapes` are the
/// graph's tensors' shapes, taken before any rewrite (`tensor_shapes`). On a graph whose norms do not match the
/// pattern, an error saying why, and the graph untouched.
pub fn fuse_norms(model: &mut pb::ModelProto, shapes: &Shapes) -> Result<usize, EvalError> {
    let g = model.graph.as_ref().ok_or_else(|| err("a model without a graph"))?;
    let ix = Index::new(g);
    let mut removed = HashSet::new();
    let mut after: HashMap<usize, Vec<pb::NodeProto>> = HashMap::new();
    for (i, n) in g.node.iter().enumerate() {
        if n.op_type != "LayerNormalization" || !n.domain.is_empty() {
            continue;
        }
        let f = norm(&ix, shapes, i).map_err(|e| err(format!("norm at {}: {e}", describe(n))))?;
        if !f.nodes.iter().all(|&k| removed.insert(k)) {
            return Err(err(format!("two norms share a node ({})", describe(n))));
        }
        after.entry(f.last).or_default().push(f.fused);
    }
    let fused = after.len();
    if fused == 0 {
        return Err(err("no norm in the graph"));
    }
    let rewritten = splice(g, &removed, after, Vec::new(), &HashSet::new());
    model.graph = Some(rewritten);
    import_microsoft(model);
    Ok(fused)
}
