//! Attention as one kernel (`com.microsoft::MultiHeadAttention`), for the GPU engine at fp16.
//!
//! The exported graph writes attention out as `Attention.forward` does (`ai/cantrip_ai/model.py`): per layer, the `qkv`
//! projection (MatMul and Add), Reshape to [B, T, 3, heads, head], Transpose, Split and Squeeze into q, k and v, then
//! `MatMul(q, kᵀ) * scale + mask`, Softmax, `MatMul(·, v)`, Transpose and Reshape back to [B, T, width]. `fuse_attention`
//! finds each of those and replaces it with three projections (the `qkv` weight split by its columns into q's, k's and
//! v's) and one `MultiHeadAttention` node, which adds the `qkv` bias itself and takes the padding mask as its
//! `attention_bias`, broadcast over the queries ([B, 1, T, T]). Its CUDA kernel is cutlass's memory-efficient attention
//! where the head size is a multiple of 8 (scores, softmax and the weighted sum accumulated in fp32), else onnxruntime's
//! unfused one (two batched matrix products and a softmax that computes in fp32); either is deterministic and computes
//! each (row, head) on its own, so a row's outputs still never depend on its batch.
//!
//! The function is the graph's: the mask is the graph's own [B, 1, 1, T] tensor, only floored at `MASK_FLOOR` so it
//! stays finite in half precision. It must be the exported `(1 - keep) * MASKED` with `keep` looked up in a table of
//! zeros and ones, so its every value is 0 or -1e9: a padding key's, at -30000, gets a weight of exactly 0 in fp32 all
//! the same, as long as no score is within 29,000 of it, which no trained network's is. A row whose every key is
//! padding - a batch slot holding no position - is the one row that differs (its scores are no longer all equal), and
//! no output of such a row is read. The projections read their input reshaped to [-1, T, width], as the exported
//! Reshape of their product did, so the attention sees the same rows whatever that input's shape.
//!
//! Every Softmax of the graph must belong to an attention found whole, every tensor inside it read by the next node of
//! the pattern alone, every constant the one the pattern names; otherwise nothing is rewritten and the reason is
//! returned.

use std::collections::{HashMap, HashSet};

use tract_onnx::pb;

use super::graph::{
    Index, MICROSOFT, attr_int, attr_ints, describe, float_tensor, floats, import_microsoft, input, node, opset, output,
    splice,
};
use super::{INT64, err};
use crate::eval::EvalError;

/// What a padding key's bias is floored to: finite in fp16 (whose largest is 65504), and far enough below any score that
/// its softmax weight is 0.
pub const MASK_FLOOR: f32 = -30000.0;

/// The graph with every attention fused (see the module's documentation): how many. On a graph whose attention does not
/// match the pattern, an error saying where, and the graph untouched.
pub fn fuse_attention(model: &mut pb::ModelProto) -> Result<usize, EvalError> {
    let opset = opset(model);
    let g = model.graph.as_ref().ok_or_else(|| err("a model without a graph"))?;
    let ix = Index::new(g);
    let mut found = Vec::new();
    for (i, n) in g.node.iter().enumerate() {
        if n.op_type == "Softmax" {
            found.push(attention(&ix, i, opset).map_err(|e| err(format!("attention at {}: {e}", describe(n))))?);
        }
    }
    if found.is_empty() {
        return Err(err("no attention in the graph"));
    }
    let mut claimed = HashSet::new();
    for f in &found {
        if !f.nodes.iter().all(|&i| claimed.insert(i)) {
            return Err(err(format!("two attentions share a node ({})", f.out)));
        }
    }
    let rewritten = rewrite(g, &ix, &found)?;
    let n = found.len();
    model.graph = Some(rewritten);
    import_microsoft(model);
    Ok(n)
}

/// A Reshape's shape: its second input, a constant.
fn reshaped_to(ix: &Index, r: usize) -> Result<Vec<i64>, String> {
    let n = ix.node(r);
    ix.ints(input(n, 1)?).ok_or_else(|| format!("{} reshapes to no constant shape", describe(n)))
}

/// One attention found: the nodes it replaces, and what the fused one is made of.
struct Found {
    nodes: Vec<usize>,
    /// The final Reshape, where the fused nodes go.
    last: usize,
    /// The normalised input the `qkv` projection reads.
    x: String,
    /// The `qkv` weight [width, 3 width] and bias [3 width].
    weight: String,
    bias: String,
    /// The graph's padding mask [B, 1, 1, T], and the node that makes it.
    mask: String,
    mask_at: usize,
    tokens: i64,
    heads: i64,
    width: i64,
    scale: f32,
    /// The attention's output [B, T, width], as the out projection reads it.
    out: String,
}

/// The exported padding mask: `(1 - keep) * MASKED`, where `keep` is a lookup in a constant table of zeros and ones
/// (`model.py`'s `present_of[kind]`), so its every value is 0 or `MASKED` and flooring it changes no weight.
fn padding_mask(ix: &Index, mask_input: &str) -> Result<(), String> {
    let refuse = |why: &str| format!("the attention bias is not the padding mask: {why}");
    let mul = ix.made_by(mask_input, "Mul").map_err(|e| refuse(&e))?;
    let m = ix.node(mul);
    let (a, b) = (input(m, 0)?, input(m, 1)?);
    let (masked, sub) = match (ix.scalar(a), ix.scalar(b)) {
        (None, Some(c)) => (c, a),
        (Some(c), None) => (c, b),
        _ => return Err(refuse("not one tensor times a constant")),
    };
    if masked != crate::eval::MASKED {
        return Err(refuse(&format!("scaled by {masked}, not MASKED")));
    }
    let sub = ix.made_by(sub, "Sub").map_err(|e| refuse(&e))?;
    let s = ix.node(sub);
    if ix.scalar(input(s, 0)?) != Some(1.0) {
        return Err(refuse("not 1 - keep"));
    }
    let gather = ix.made_by(input(s, 1)?, "Gather").map_err(|e| refuse(&e))?;
    let table = ix
        .constants
        .get(input(ix.node(gather), 0)?)
        .and_then(|t| floats(t))
        .ok_or_else(|| refuse("keep is not looked up in a constant table"))?;
    if table.is_empty() || !table.iter().all(|&x| x == 0.0 || x == 1.0) {
        return Err(refuse("keep's table holds more than zeros and ones"));
    }
    Ok(())
}

/// The attention whose Softmax is node `s`.
fn attention(ix: &Index, s: usize, opset: i64) -> Result<Found, String> {
    let sm = ix.node(s);
    let axis = attr_int(sm, "axis").unwrap_or(if opset >= 13 { -1 } else { 1 });
    if axis != -1 && axis != 3 {
        return Err(format!("a softmax over axis {axis}"));
    }
    let (sm_in, sm_out) = (input(sm, 0)?, output(sm, 0)?);
    // mask + scores * scale
    let add = ix.made_by(sm_in, "Add")?;
    ix.only_reader(sm_in)?;
    let a = ix.node(add);
    let (a0, a1) = (input(a, 0)?, input(a, 1)?);
    let is_mask = |t: &str| {
        ix.made_by(t, "Reshape")
            .is_ok_and(|r| reshaped_to(ix, r).is_ok_and(|v| v.len() == 4 && v[..3] == [-1, 1, 1]))
    };
    let (scored, mask) = match (is_mask(a0), is_mask(a1)) {
        (false, true) => (a0, a1),
        (true, false) => (a1, a0),
        _ => return Err("no padding mask [-1, 1, 1, T] beside the scores".into()),
    };
    let mask_at = ix.made_by(mask, "Reshape")?;
    let tokens = reshaped_to(ix, mask_at)?[3];
    padding_mask(ix, input(ix.node(mask_at), 0)?)?;
    let mul = ix.made_by(scored, "Mul")?;
    ix.only_reader(scored)?;
    let m = ix.node(mul);
    let (m0, m1) = (input(m, 0)?, input(m, 1)?);
    let (scores, scale) = match (ix.scalar(m0), ix.scalar(m1)) {
        (None, Some(c)) => (m0, c),
        (Some(c), None) => (m1, c),
        _ => return Err("the scores are not scaled by one constant".into()),
    };
    // MultiHeadAttention reads a scale of 0 as "the default", 1/sqrt(head).
    if !(scale.is_finite() && scale > 0.0) {
        return Err(format!("a scale of {scale}"));
    }
    let qk = ix.made_by(scores, "MatMul")?;
    ix.only_reader(scores)?;
    let (q, kt) = (input(ix.node(qk), 0)?, input(ix.node(qk), 1)?);
    let kt_at = ix.made_by(kt, "Transpose")?;
    ix.only_reader(kt)?;
    if attr_ints(ix.node(kt_at), "perm").as_deref() != Some(&[0, 1, 3, 2]) {
        return Err("k is not transposed as kᵀ".into());
    }
    let k = input(ix.node(kt_at), 0)?;
    // softmax · v, back to [B, T, width]
    let pv = ix.only_reader(sm_out)?;
    let p = ix.node(pv);
    if p.op_type != "MatMul" || !p.domain.is_empty() || input(p, 0)? != sm_out {
        return Err("the softmax is not multiplied by v".into());
    }
    let (v, pv_out) = (input(p, 1)?, output(p, 0)?);
    let t2 = ix.only_reader(pv_out)?;
    if ix.node(t2).op_type != "Transpose" || attr_ints(ix.node(t2), "perm").as_deref() != Some(&[0, 2, 1, 3]) {
        return Err("the heads' outputs are not transposed back".into());
    }
    let t2_out = output(ix.node(t2), 0)?;
    let last = ix.only_reader(t2_out)?;
    let r2 = ix.node(last);
    if r2.op_type != "Reshape" || !r2.domain.is_empty() || input(r2, 0)? != t2_out {
        return Err("the heads' outputs are not reshaped".into());
    }
    let Ok(&[-1, t, width]) = reshaped_to(ix, last).as_deref() else {
        return Err("the heads' outputs are not reshaped to [-1, T, width]".into());
    };
    // q, k and v: Squeeze(Split(Transpose(Reshape(x · W + b))))
    let mut squeezes = Vec::new();
    let mut split = None;
    for (slot, (tensor, reader)) in [(q, qk), (k, kt_at), (v, pv)].into_iter().enumerate() {
        let sq = ix.made_by(tensor, "Squeeze")?;
        if ix.only_reader(tensor)? != reader {
            return Err(format!("{tensor} is read by more than the attention"));
        }
        let n = ix.node(sq);
        let axes = match n.input.get(1).filter(|s| !s.is_empty()) {
            Some(a) => ix.ints(a),
            None => attr_ints(n, "axes"),
        };
        if axes.as_deref() != Some(&[0]) {
            return Err(format!("{tensor} is not squeezed on axis 0"));
        }
        let part = input(n, 0)?;
        let sp = ix.made_by(part, "Split")?;
        ix.only_reader(part)?;
        if ix.node(sp).output.iter().position(|o| o == part) != Some(slot) || *split.get_or_insert(sp) != sp {
            return Err("q, k and v are not one split's three parts in order".into());
        }
        squeezes.push(sq);
    }
    let sp = split.ok_or("no split")?;
    let n = ix.node(sp);
    let sizes = match n.input.get(1).filter(|s| !s.is_empty()) {
        Some(a) => Some(ix.ints(a).ok_or("the qkv split's sizes are no constant")?),
        None => attr_ints(n, "split").filter(|v| !v.is_empty()),
    };
    if attr_int(n, "axis").unwrap_or(0) != 0
        || n.output.len() != 3
        || !matches!(sizes.as_deref(), None | Some(&[1, 1, 1]))
    {
        return Err("the qkv split is not three ones on axis 0".into());
    }
    let packed = input(n, 0)?;
    let t1 = ix.made_by(packed, "Transpose")?;
    ix.only_reader(packed)?;
    if attr_ints(ix.node(t1), "perm").as_deref() != Some(&[2, 0, 3, 1, 4]) {
        return Err("qkv is not transposed to [3, B, heads, T, head]".into());
    }
    let unpacked = input(ix.node(t1), 0)?;
    let r1 = ix.made_by(unpacked, "Reshape")?;
    ix.only_reader(unpacked)?;
    let Ok(&[-1, t1_tokens, 3, heads, head]) = reshaped_to(ix, r1).as_deref() else {
        return Err("qkv is not reshaped to [-1, T, 3, heads, head]".into());
    };
    // Every size given, none of them 0 (which a Reshape reads as "copy this dimension").
    if tokens < 1 || heads < 1 || head < 1 || t != tokens || t1_tokens != tokens || heads * head != width {
        return Err(format!("shapes that disagree: T {tokens}/{t}/{t1_tokens}, {heads} heads of {head}, width {width}"));
    }
    let qkv = input(ix.node(r1), 0)?;
    let ab = ix.made_by(qkv, "Add")?;
    ix.only_reader(qkv)?;
    let b = ix.node(ab);
    let (b0, b1) = (input(b, 0)?, input(b, 1)?);
    let (product, bias) = if ix.weight(b1, &[3 * width]).is_some() {
        (b0, b1)
    } else if ix.weight(b0, &[3 * width]).is_some() {
        (b1, b0)
    } else {
        return Err(format!("the qkv bias is no float initializer of {}", 3 * width));
    };
    let mm = ix.made_by(product, "MatMul")?;
    ix.only_reader(product)?;
    let weight = input(ix.node(mm), 1)?;
    if ix.weight(weight, &[width, 3 * width]).is_none() {
        return Err(format!("the qkv weight is no float initializer of [{width}, {}]", 3 * width));
    }
    let mut nodes = vec![mm, ab, r1, t1, sp, kt_at, qk, mul, add, s, pv, t2, last];
    nodes.extend(squeezes);
    Ok(Found {
        nodes,
        last,
        x: input(ix.node(mm), 0)?.to_string(),
        weight: weight.to_string(),
        bias: bias.to_string(),
        mask: mask.to_string(),
        mask_at,
        tokens,
        heads,
        width,
        scale,
        out: output(r2, 0)?.to_string(),
    })
}

fn rewrite(g: &pb::GraphProto, ix: &Index, found: &[Found]) -> Result<pb::GraphProto, EvalError> {
    use pb::attribute_proto::AttributeType;
    let mut added = Vec::new();
    let mut split_weights = HashSet::new();
    let mut masks: HashMap<&str, String> = HashMap::new();
    let mut after: HashMap<usize, Vec<pb::NodeProto>> = HashMap::new();
    let mut at: HashMap<usize, Vec<pb::NodeProto>> = HashMap::new();
    for f in found {
        let w = f.width as usize;
        if split_weights.insert(f.weight.as_str()) {
            let values = ix
                .initializers
                .get(f.weight.as_str())
                .and_then(|t| floats(t))
                .ok_or_else(|| err("the qkv weight"))?;
            for (part, name) in ["q", "k", "v"].iter().enumerate() {
                let cols: Vec<f32> = values
                    .chunks_exact(3 * w)
                    .flat_map(|row| row[part * w..(part + 1) * w].iter().copied())
                    .collect();
                added.push(float_tensor(&format!("{}__{name}", f.weight), vec![f.width, f.width], &cols));
            }
        }
        let mask = masks
            .entry(f.mask.as_str())
            .or_insert_with(|| {
                let floor = format!("{}__floor", f.mask);
                let square = format!("{}__square", f.mask);
                added.push(float_tensor(&floor, vec![], &[MASK_FLOOR]));
                added.push(pb::TensorProto {
                    name: square.clone(),
                    dims: vec![4],
                    data_type: INT64,
                    int64_data: vec![1, 1, f.tokens, f.tokens],
                    ..Default::default()
                });
                let floored = format!("{}__floored", f.mask);
                let attention = format!("{}__attention", f.mask);
                after.entry(f.mask_at).or_default().extend([
                    node("Max", "", &[&f.mask, &floor], &floored, vec![]),
                    node("Expand", "", &[&floored, &square], &attention, vec![]),
                ]);
                attention
            })
            .clone();
        // The projections read x as [B, T, width], as the exported Reshape of their product read it, whatever x's own
        // shape: the attention sees the rows the graph's attention saw.
        let rows = format!("{}__rows", f.out);
        let x = format!("{}__x", f.out);
        added.push(pb::TensorProto {
            name: rows.clone(),
            dims: vec![3],
            data_type: INT64,
            int64_data: vec![-1, f.tokens, f.width],
            ..Default::default()
        });
        let mut fused = vec![node("Reshape", "", &[&f.x, &rows], &x, vec![])];
        for name in ["q", "k", "v"] {
            let (w, out) = (format!("{}__{name}", f.weight), format!("{}__{name}", f.out));
            fused.push(node("MatMul", "", &[&x, &w], &out, vec![]));
        }
        let (q, k, v) = (format!("{}__q", f.out), format!("{}__k", f.out), format!("{}__v", f.out));
        fused.push(node(
            "MultiHeadAttention",
            MICROSOFT,
            &[&q, &k, &v, &f.bias, "", &mask],
            &f.out,
            vec![
                pb::AttributeProto {
                    name: "num_heads".into(),
                    r#type: AttributeType::Int as i32,
                    i: f.heads,
                    ..Default::default()
                },
                pb::AttributeProto {
                    name: "scale".into(),
                    r#type: AttributeType::Float as i32,
                    f: f.scale,
                    ..Default::default()
                },
            ],
        ));
        at.insert(f.last, fused);
    }
    let removed: HashSet<usize> = found.iter().flat_map(|f| f.nodes.iter().copied()).collect();
    for (i, nodes) in at {
        after.entry(i).or_default().extend(nodes);
    }
    Ok(splice(g, &removed, after, added, &split_weights))
}
