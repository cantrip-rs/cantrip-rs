//! The ONNX graph as the GPU engine's rewrites read it: who makes and who reads each tensor, its constants, and the
//! splicing of new nodes in place of old ones. Every lookup a pattern needs is checked: a graph that is not what a
//! pattern expects is refused with a reason, never a panic.

use std::collections::{HashMap, HashSet};

use tract_onnx::pb;

use super::{FLOAT, INT32, INT64};

/// The domain of onnxruntime's own operators.
pub const MICROSOFT: &str = "com.microsoft";

pub(super) fn describe(n: &pb::NodeProto) -> String {
    if n.name.is_empty() {
        format!("{} -> {:?}", n.op_type, n.output)
    } else {
        n.name.clone()
    }
}

/// Who makes and who reads each tensor, and the constants.
pub(super) struct Index<'a> {
    pub(super) g: &'a pb::GraphProto,
    pub(super) producer: HashMap<&'a str, usize>,
    pub(super) consumers: HashMap<&'a str, Vec<usize>>,
    pub(super) outputs: HashSet<&'a str>,
    pub(super) constants: HashMap<&'a str, &'a pb::TensorProto>,
    pub(super) initializers: HashMap<&'a str, &'a pb::TensorProto>,
}

impl<'a> Index<'a> {
    pub(super) fn new(g: &'a pb::GraphProto) -> Index<'a> {
        let mut ix = Index {
            g,
            producer: HashMap::new(),
            consumers: HashMap::new(),
            outputs: g.output.iter().map(|o| o.name.as_str()).collect(),
            constants: HashMap::new(),
            initializers: HashMap::new(),
        };
        for t in &g.initializer {
            ix.initializers.insert(&t.name, t);
            ix.constants.insert(&t.name, t);
        }
        for (i, n) in g.node.iter().enumerate() {
            for o in &n.output {
                ix.producer.insert(o, i);
            }
            for input in n.input.iter().filter(|s| !s.is_empty()) {
                ix.consumers.entry(input).or_default().push(i);
            }
            if n.op_type == "Constant"
                && n.domain.is_empty()
                && let Some(t) = n.attribute.iter().find(|a| a.name == "value").and_then(|a| a.t.as_ref())
                && let Some(o) = n.output.first()
            {
                ix.constants.insert(o, t);
            }
        }
        ix
    }

    pub(super) fn node(&self, i: usize) -> &'a pb::NodeProto {
        &self.g.node[i]
    }

    /// The node of `op` (the default domain) that makes `tensor`.
    pub(super) fn made_by(&self, tensor: &str, op: &str) -> Result<usize, String> {
        match self.producer.get(tensor) {
            Some(&i) if self.g.node[i].op_type == op && self.g.node[i].domain.is_empty() => Ok(i),
            Some(&i) => Err(format!("{tensor} is made by {}, not {op}", describe(&self.g.node[i]))),
            None => Err(format!("{tensor} is made by no node, not {op}")),
        }
    }

    /// The one node that reads `tensor` (which is no output of the graph).
    pub(super) fn only_reader(&self, tensor: &str) -> Result<usize, String> {
        match self.consumers.get(tensor).map(Vec::as_slice) {
            Some(&[i]) if !self.outputs.contains(tensor) => Ok(i),
            _ => Err(format!("{tensor} is read by more than the pattern's next node")),
        }
    }

    pub(super) fn ints(&self, tensor: &str) -> Option<Vec<i64>> {
        let t = self.constants.get(tensor)?;
        match t.data_type {
            INT64 if !t.int64_data.is_empty() => Some(t.int64_data.clone()),
            INT64 => Some(
                t.raw_data
                    .chunks_exact(8)
                    .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
                    .collect(),
            ),
            INT32 if !t.int32_data.is_empty() => Some(t.int32_data.iter().map(|&x| x as i64).collect()),
            INT32 => Some(
                t.raw_data
                    .chunks_exact(4)
                    .map(|c| i32::from_le_bytes(c.try_into().unwrap()) as i64)
                    .collect(),
            ),
            _ => None,
        }
    }

    pub(super) fn scalar(&self, tensor: &str) -> Option<f32> {
        let t = self.constants.get(tensor)?;
        match floats(t)?.as_slice() {
            &[x] => Some(x),
            _ => None,
        }
    }

    /// A float initializer of exactly these dimensions.
    pub(super) fn weight(&self, tensor: &str, dims: &[i64]) -> Option<&'a pb::TensorProto> {
        let t = *self.initializers.get(tensor)?;
        (t.data_type == FLOAT && t.dims == dims && t.external_data.is_empty()).then_some(t)
    }
}

pub(super) fn floats(t: &pb::TensorProto) -> Option<Vec<f32>> {
    if t.data_type != FLOAT || !t.external_data.is_empty() {
        return None;
    }
    if !t.float_data.is_empty() {
        return Some(t.float_data.clone());
    }
    Some(t.raw_data.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect())
}

pub(super) fn attr_ints(n: &pb::NodeProto, name: &str) -> Option<Vec<i64>> {
    n.attribute.iter().find(|a| a.name == name).map(|a| a.ints.clone())
}

pub(super) fn attr_int(n: &pb::NodeProto, name: &str) -> Option<i64> {
    n.attribute.iter().find(|a| a.name == name).map(|a| a.i)
}

/// Input `k` of a node, which it must have.
pub(super) fn input(n: &pb::NodeProto, k: usize) -> Result<&str, String> {
    match n.input.get(k) {
        Some(s) if !s.is_empty() => Ok(s),
        _ => Err(format!("{} has no input {k}", describe(n))),
    }
}

/// Output `k` of a node, which it must have.
pub(super) fn output(n: &pb::NodeProto, k: usize) -> Result<&str, String> {
    match n.output.get(k) {
        Some(s) if !s.is_empty() => Ok(s),
        _ => Err(format!("{} has no output {k}", describe(n))),
    }
}

pub(super) fn float_tensor(name: &str, dims: Vec<i64>, values: &[f32]) -> pb::TensorProto {
    pb::TensorProto {
        name: name.into(),
        dims,
        data_type: FLOAT,
        raw_data: values.iter().flat_map(|x| x.to_le_bytes()).collect(),
        ..Default::default()
    }
}

pub(super) fn node(op: &str, domain: &str, inputs: &[&str], output: &str, attribute: Vec<pb::AttributeProto>) -> pb::NodeProto {
    pb::NodeProto {
        input: inputs.iter().map(|s| s.to_string()).collect(),
        output: vec![output.to_string()],
        name: format!("{output}__node"),
        op_type: op.into(),
        domain: domain.into(),
        attribute,
        ..Default::default()
    }
}

/// The graph with the nodes `removed` taken out, `after[i]` put in after node `i` (in its place if it is removed), the
/// initializers `added` added; then the Constant nodes and the `orphans` (initializers) nothing reads any more left out,
/// and the value information of tensors no longer made.
pub(super) fn splice(
    g: &pb::GraphProto,
    removed: &HashSet<usize>,
    mut after: HashMap<usize, Vec<pb::NodeProto>>,
    added: Vec<pb::TensorProto>,
    orphans: &HashSet<&str>,
) -> pb::GraphProto {
    let mut out = g.clone();
    out.node.clear();
    for (i, n) in g.node.iter().enumerate() {
        if !removed.contains(&i) {
            out.node.push(n.clone());
        }
        out.node.extend(after.remove(&i).unwrap_or_default());
    }
    out.initializer.extend(added);
    let read: HashSet<String> = out
        .node
        .iter()
        .flat_map(|n| n.input.iter().cloned())
        .chain(out.output.iter().map(|o| o.name.clone()))
        .collect();
    out.node
        .retain(|n| !(n.op_type == "Constant" && n.domain.is_empty() && n.output.iter().all(|o| !read.contains(o))));
    out.initializer.retain(|t| !orphans.contains(t.name.as_str()) || read.contains(&t.name));
    let known: HashSet<&str> = out.node.iter().flat_map(|n| n.output.iter().map(String::as_str)).collect();
    let kept: Vec<_> = out.value_info.iter().filter(|v| known.contains(v.name.as_str())).cloned().collect();
    out.value_info = kept;
    out
}

/// Each tensor's shape, its dimensions written out ("batch" for the free batch), as tract infers them: `None` where a
/// dimension or the rank is left open.
pub type Shapes = HashMap<String, Option<Vec<String>>>;

/// The graph's tensors' shapes (`Shapes`), inferred by tract: only on a graph of standard operators, so before any
/// rewrite into onnxruntime's own.
pub fn tensor_shapes(model: &pb::ModelProto) -> Result<Shapes, String> {
    use tract_onnx::prelude::*;
    use tract_onnx::tract_hir::infer::Factoid;
    let mut m = tract_onnx::onnx().model_for_proto_model(model).map_err(|e| format!("tract: {e}"))?;
    m.analyse(false).map_err(|e| format!("tract: {e}"))?;
    let mut out = HashMap::new();
    for node in m.nodes() {
        for (ix, o) in node.outputs.iter().enumerate() {
            let Some(label) = m.outlet_label(OutletId::new(node.id, ix)) else { continue };
            let dims = if o.fact.shape.is_open() {
                None
            } else {
                o.fact.shape.dims().map(|d| d.concretize().map(|t| t.to_string())).collect::<Option<Vec<_>>>()
            };
            out.insert(label.to_string(), dims);
        }
    }
    Ok(out)
}

/// The model importing onnxruntime's operators.
pub(super) fn import_microsoft(model: &mut pb::ModelProto) {
    if !model.opset_import.iter().any(|o| o.domain == MICROSOFT) {
        model.opset_import.push(pb::OperatorSetIdProto {
            domain: MICROSOFT.into(),
            version: 1,
        });
    }
}

/// The default domain's opset.
pub(super) fn opset(model: &pb::ModelProto) -> i64 {
    model
        .opset_import
        .iter()
        .find(|o| o.domain.is_empty() || o.domain == "ai.onnx")
        .map_or(0, |o| o.version)
}
