//! The GPU engine (`cantrip_ai::gpu`, the `gpu` feature): `cargo test -p cantrip-ai --features gpu --test gpu`.
//!
//! Without a GPU:
//! - the rows it takes are the same whether they come from positions, a padded batch or the wire;
//! - the graph it loads takes narrow integer inputs and makes bitwise what the exported graph makes (tract runs both);
//! - its half-precision graph (matrix products in fp16) stays within the reduced bar of the references;
//! - every bucket's attention is fused, and the rewritten graph at each level (attention, GELUs, residual norms)
//!   computes what the graph does (onnxruntime's CPU provider at fp32: `ORT_DYLIB_PATH` names an onnxruntime; nothing
//!   runs on the GPU); a pattern out of place is left alone, the graph untouched; at fp16 the half nodes pass
//!   half-precision tensors to each other directly, and no half tensor goes to float and back.
//!   `--ignored trained_models_fused` runs the same on the trained models `CANTRIP_FUSE_MODELS` names, also on the CPU.
//!
//! With one (`--ignored`; onnxruntime-gpu named by `ORT_DYLIB_PATH`, CUDA's libraries by `CANTRIP_CUDA_LIBS`):
//! - the engine makes the tiny model's references, within export's tolerance at fp32 and the reduced bar at tf32 and
//!   at fp16, at every level of the fp16 graph (`Fp16Graph`) and with either GELU kernel;
//! - a row's outputs are bitwise the same whatever it is batched with and wherever in the batch: on the tiny model, and
//!   on the trained models `CANTRIP_SHAPE_MODELS` (or `CANTRIP_FUSE_MODELS`) names (`a_trained_model_s_row`);
//! - with the opt-in smaller shapes (`Options::shapes`), every shape the engine keeps computes the rows the test has
//!   bitwise as the full shape does, and under traffic each row is the row alone (`a_row_is_the_same_at_every_shape`,
//!   on the same trained models);
//! - `evaluate_positions` is bitwise `evaluate`.

#![cfg(feature = "gpu")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cantrip_ai::CardIndex;
use cantrip_ai::eval::reference::Reference;
use cantrip_ai::eval::{Evaluator, FeatureBatch};
use cantrip_ai::gpu::{self, Engine, Fp16Graph, GeluKernel, GpuEvaluator, Options, Precision, Rows};
use cantrip_obs::encode::{CATS, Encoded, NUMS, PTRS};
use prost::Message;
use tract_onnx::pb;
use tract_onnx::prelude::*;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny")
}

fn own(dir: &Path) -> CardIndex {
    CardIndex::read(&dir.join("cards.json")).unwrap()
}

fn references() -> Reference {
    Reference::load(&tiny().join("reference")).unwrap()
}

#[test]
fn rows_are_the_same_from_positions_a_batch_and_the_wire() {
    let r = references();
    let positions: Vec<&Encoded> = r.positions.iter().take(24).collect();
    let batch = FeatureBatch::new(&positions);
    let from_positions = Rows::from_positions(&positions, &[]).unwrap();
    let from_batch = Rows::from_batch(&batch, &[]).unwrap();
    assert_eq!(format!("{from_positions:?}"), format!("{from_batch:?}"));
    // The wire sends the rows by bucket (`order`): each row the same as the batch's.
    let (payload, order) = cantrip_ai::remote::request(&batch, &[]).unwrap();
    let wire = Rows::from_wire(&payload).unwrap();
    let starts = |r: &Rows| {
        let mut at = 0;
        r.lengths
            .iter()
            .map(|&l| {
                at += l;
                at - l
            })
            .collect::<Vec<_>>()
    };
    let (sw, sb) = (starts(&wire), starts(&from_batch));
    for (i, &row) in order.iter().enumerate() {
        let len = wire.lengths[i];
        assert_eq!(len, from_batch.lengths[row]);
        let (a, b) = (sw[i], sb[row]);
        assert_eq!(wire.kind[a..a + len], from_batch.kind[b..b + len]);
        assert_eq!(wire.card[a..a + len], from_batch.card[b..b + len]);
        assert_eq!(wire.cats[a * CATS..(a + len) * CATS], from_batch.cats[b * CATS..(b + len) * CATS]);
        assert_eq!(wire.ptrs[a * PTRS..(a + len) * PTRS], from_batch.ptrs[b * PTRS..(b + len) * PTRS]);
        assert_eq!(wire.legal[a..a + len], from_batch.legal[b..b + len]);
        for k in 0..len {
            let nums = |r: &Rows, t: usize| {
                let (n0, n1) = (r.num_start[t] as usize, r.num_start[t + 1] as usize);
                (r.num_feature[n0..n1].to_vec(), r.num_value[n0..n1].iter().map(|x| x.to_bits()).collect::<Vec<_>>())
            };
            assert_eq!(nums(&wire, a + k), nums(&from_batch, b + k));
        }
    }
}

/// A graph through tract at `tokens` tokens and `rows` rows, its integer inputs as `narrow` says.
fn tract_run(bytes: &[u8], b: &FeatureBatch, narrow: bool) -> TVec<TValue> {
    tract_run_as(bytes, b, narrow, true)
}

/// `tract_run`, `optimized` or each node as written (no cast folded, no node fused: what the graph says, exactly).
fn tract_run_as(bytes: &[u8], b: &FeatureBatch, narrow: bool, optimized: bool) -> TVec<TValue> {
    let (rows, t) = (b.rows, b.tokens);
    let mut model = tract_onnx::onnx().model_for_read(&mut &bytes[..]).unwrap();
    let ints = |v: &[i64]| v.to_vec();
    let inputs: TVec<TValue> = if narrow {
        model = model
            .with_input_fact(0, u8::fact([rows, t]).into())
            .unwrap()
            .with_input_fact(1, i32::fact([rows, t]).into())
            .unwrap()
            .with_input_fact(2, u8::fact([rows, t, CATS]).into())
            .unwrap()
            .with_input_fact(3, u8::fact([rows, t, PTRS]).into())
            .unwrap();
        tvec!(
            Tensor::from_shape(&[rows, t], &b.kind.iter().map(|&x| x as u8).collect::<Vec<_>>()).unwrap().into(),
            Tensor::from_shape(&[rows, t], &b.card.iter().map(|&x| x as i32).collect::<Vec<_>>()).unwrap().into(),
            Tensor::from_shape(&[rows, t, CATS], &b.cats.iter().map(|&x| x as u8).collect::<Vec<_>>()).unwrap().into(),
            Tensor::from_shape(&[rows, t, PTRS], &b.ptrs.iter().map(|&x| x as u8).collect::<Vec<_>>()).unwrap().into(),
            Tensor::from_shape(&[rows, t, NUMS], &b.nums).unwrap().into(),
            Tensor::from_shape(&[rows, t], &b.legal).unwrap().into(),
        )
    } else {
        model = model
            .with_input_fact(0, i64::fact([rows, t]).into())
            .unwrap()
            .with_input_fact(1, i64::fact([rows, t]).into())
            .unwrap()
            .with_input_fact(2, i64::fact([rows, t, CATS]).into())
            .unwrap()
            .with_input_fact(3, i64::fact([rows, t, PTRS]).into())
            .unwrap();
        tvec!(
            Tensor::from_shape(&[rows, t], &ints(&b.kind)).unwrap().into(),
            Tensor::from_shape(&[rows, t], &ints(&b.card)).unwrap().into(),
            Tensor::from_shape(&[rows, t, CATS], &ints(&b.cats)).unwrap().into(),
            Tensor::from_shape(&[rows, t, PTRS], &ints(&b.ptrs)).unwrap().into(),
            Tensor::from_shape(&[rows, t, NUMS], &b.nums).unwrap().into(),
            Tensor::from_shape(&[rows, t], &b.legal).unwrap().into(),
        )
    };
    let model = model
        .with_input_fact(4, f32::fact([rows, t, NUMS]).into())
        .unwrap()
        .with_input_fact(5, bool::fact([rows, t]).into())
        .unwrap();
    let typed = if optimized { model.into_optimized().unwrap() } else { model.into_typed().unwrap() };
    typed.into_runnable().unwrap().run(inputs).unwrap()
}

fn floats(v: &TValue) -> Vec<f32> {
    v.try_as_plain_ram().unwrap().as_slice::<f32>().unwrap().to_vec()
}

/// The positions of the references in `bucket`, as a batch.
fn bucket_batch(r: &Reference, bucket: usize, rows: usize) -> FeatureBatch {
    let es: Vec<&Encoded> = r.positions.iter().filter(|e| e.bucket == bucket).take(rows).collect();
    assert!(!es.is_empty(), "no reference of {bucket} tokens");
    FeatureBatch::new(&es)
}

#[test]
fn the_narrowed_graph_is_the_graph() {
    let r = references();
    for t in cantrip_obs::encode::BUCKETS {
        let path = tiny().join(format!("model-t{t}.onnx"));
        let bytes = std::fs::read(&path).unwrap();
        let mut model = pb::ModelProto::decode(&bytes[..]).unwrap();
        gpu::narrow_inputs(&mut model).unwrap();
        let narrowed = model.encode_to_vec();
        let Some(b) = r.positions.iter().any(|e| e.bucket == t).then(|| bucket_batch(&r, t, 8)) else {
            continue;
        };
        let (wide, narrow) = (tract_run(&bytes, &b, false), tract_run(&narrowed, &b, true));
        for k in 0..3 {
            let (x, y) = (floats(&wide[k]), floats(&narrow[k]));
            assert!(x.iter().zip(&y).all(|(p, q)| p.to_bits() == q.to_bits()), "output {k} at {t} tokens");
        }
    }
}

#[test]
fn the_half_graph_matches_its_matrix_products_only() {
    let bytes = std::fs::read(tiny().join("model-t32.onnx")).unwrap();
    let mut model = pb::ModelProto::decode(&bytes[..]).unwrap();
    let before = model.clone();
    let stage = gpu::input_stage(&model).unwrap();
    let staged: std::collections::HashSet<String> =
        stage.iter().map(|&i| model.graph.as_ref().unwrap().node[i].output[0].clone()).collect();
    let kept = gpu::half_matmuls(&mut model).unwrap();
    let g = model.graph.as_ref().unwrap();
    let ops = |g: &pb::GraphProto, op: &str| g.node.iter().filter(|n| n.op_type == op).count();
    let old = before.graph.as_ref().unwrap();
    let products = ops(old, "MatMul") + ops(old, "Gemm");
    // The input stage's products (the card table's, the numbers', the pointers' gather and projection) stay as they
    // were; every other product reads half (a cast, a half initializer, or a half output moved by a Reshape) and writes
    // half; every other node is as it was.
    assert_eq!(ops(g, "MatMul") + ops(g, "Gemm"), products);
    assert_eq!(kept, 3, "the tiny model's numbers, pointer gather and pointer projection");
    for n in &g.node {
        if n.op_type == "MatMul" || n.op_type == "Gemm" {
            if staged.contains(&n.output[0]) {
                assert!(n.input.iter().all(|i| !i.contains("__fp16")), "{n:?}");
                continue;
            }
            assert!(n.input.iter().all(|i| i.ends_with("__fp16") || i.ends_with("__fp16out")), "{n:?}");
            assert!(n.output[0].ends_with("__fp16out"));
        }
    }
    // A node that only moves values may move a product's half output instead (the same values): up to that name.
    let others = |g: &pb::GraphProto| {
        let plain = |v: &[String]| v.iter().map(|s| s.trim_end_matches("__fp16out").to_string()).collect::<Vec<_>>();
        g.node
            .iter()
            .filter(|n| !matches!(n.op_type.as_str(), "MatMul" | "Gemm" | "Cast"))
            .map(|n| format!("{}:{:?}->{:?}", n.op_type, plain(&n.input), plain(&n.output)))
            .collect::<Vec<_>>()
    };
    assert_eq!(others(g), others(old));
}

fn model(path: &Path) -> pb::ModelProto {
    pb::ModelProto::decode(&std::fs::read(path).unwrap()[..]).unwrap()
}

fn manifest(dir: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap()
}

fn count(g: &pb::GraphProto, op: &str) -> usize {
    g.node.iter().filter(|n| n.op_type == op).count()
}

#[test]
fn every_bucket_s_attention_is_fused() {
    let manifest = manifest(&tiny());
    let (layers, heads, width) = (
        manifest["config"]["layers"].as_u64().unwrap() as usize,
        manifest["config"]["heads"].as_i64().unwrap(),
        manifest["config"]["width"].as_i64().unwrap(),
    );
    for t in cantrip_obs::encode::BUCKETS {
        let mut m = model(&tiny().join(format!("model-t{t}.onnx")));
        let old = m.graph.clone().unwrap();
        assert_eq!(gpu::fuse_attention(&mut m).unwrap(), layers, "{t} tokens");
        let g = m.graph.as_ref().unwrap();
        assert!(m.opset_import.iter().any(|o| o.domain == gpu::MICROSOFT && o.version == 1));
        // A layer's softmax, its qkv split and the split's three squeezes are gone (the heads' outputs split too).
        for (op, per_layer) in [("Softmax", 1), ("Split", 1), ("Squeeze", 3)] {
            assert_eq!(count(g, op), count(&old, op) - per_layer * layers, "{op}s at {t} tokens");
        }
        assert_eq!(count(g, "Softmax"), 0);
        // A layer's qkv projection and its attention's two products are now the q, k and v projections.
        assert_eq!(count(g, "MatMul"), count(&old, "MatMul"));
        let fused: Vec<_> = g.node.iter().filter(|n| n.op_type == "MultiHeadAttention").collect();
        assert_eq!(fused.len(), layers);
        let head = (width / heads) as f32;
        for n in fused {
            assert_eq!(n.domain, gpu::MICROSOFT);
            assert_eq!(n.input.len(), 6);
            assert!(n.input[4].is_empty(), "no key padding mask: the mask is the attention bias");
            let attr = |name: &str| n.attribute.iter().find(|a| a.name == name).unwrap();
            assert_eq!(attr("num_heads").i, heads);
            assert!((attr("scale").f - head.powf(-0.5)).abs() < 1e-7);
            // The attention bias: the graph's mask floored and broadcast over the queries.
            let expand = g.node.iter().find(|m| m.output[0] == n.input[5]).unwrap();
            assert_eq!(expand.op_type, "Expand");
            let floor = g.node.iter().find(|m| m.output[0] == expand.input[0]).unwrap();
            assert_eq!(floor.op_type, "Max");
            let square = g.initializer.iter().find(|i| i.name == expand.input[1]).unwrap();
            assert_eq!(square.int64_data, vec![1, 1, t as i64, t as i64]);
        }
        // Every tensor read is made by a node, or is an input or an initializer.
        let mut known: std::collections::HashSet<&str> = g.input.iter().map(|i| i.name.as_str()).collect();
        known.extend(g.initializer.iter().map(|i| i.name.as_str()));
        for n in &g.node {
            for i in n.input.iter().filter(|i| !i.is_empty()) {
                assert!(known.contains(i.as_str()), "{i} read before it is made, at {t} tokens");
            }
            known.extend(n.output.iter().map(String::as_str));
        }
    }
}

/// A graph through onnxruntime's CPU provider (`ORT_DYLIB_PATH`; no GPU) on a batch of `t` tokens: value, policy, aux
/// and belief. `optimized`: with onnxruntime's own fusions, as a session normally runs; otherwise node by node.
fn cpu_run(graph: &[u8], b: &FeatureBatch, t: usize, optimized: bool) -> [Vec<f32>; 4] {
    use ort::session::Session;
    use ort::session::builder::GraphOptimizationLevel;
    use ort::value::TensorRef;
    let env = cantrip_ai::eval::ort_environment().unwrap();
    let level = if optimized { GraphOptimizationLevel::Level3 } else { GraphOptimizationLevel::Disable };
    let builder = Session::builder(env).unwrap().with_intra_threads(4).unwrap();
    let mut builder = builder.with_optimization_level(level).unwrap();
    let mut s = builder.commit_from_memory(graph).unwrap();
    let n = b.rows;
    let out = s
        .run(ort::inputs![
            "kind" => TensorRef::from_array_view(([n, t], &b.kind[..])).unwrap(),
            "card" => TensorRef::from_array_view(([n, t], &b.card[..])).unwrap(),
            "cats" => TensorRef::from_array_view(([n, t, CATS], &b.cats[..])).unwrap(),
            "ptrs" => TensorRef::from_array_view(([n, t, PTRS], &b.ptrs[..])).unwrap(),
            "nums" => TensorRef::from_array_view(([n, t, NUMS], &b.nums[..])).unwrap(),
            "legal" => TensorRef::from_array_view(([n, t], &b.legal[..])).unwrap(),
        ])
        .unwrap();
    ["value", "policy", "aux", "belief"].map(|name| out[name].try_extract_tensor::<f32>().unwrap().1.to_vec())
}

/// The rewrites of `level` (and the levels below it) on a graph at fp32, as the engine makes them before halving.
fn rewrite(m: &mut pb::ModelProto, level: Fp16Graph, gelu: GeluKernel) {
    let shapes = gpu::tensor_shapes(m).unwrap();
    if level >= Fp16Graph::Attention {
        gpu::fuse_attention(m).unwrap();
    }
    if level >= Fp16Graph::Ffn {
        gpu::fuse_gelu(m, gelu).unwrap();
    }
    if level >= Fp16Graph::Residual {
        gpu::fuse_norms(m, &shapes).unwrap();
    }
}

/// The rewrites a test runs at fp32: each level, and the FFN's with either GELU kernel.
const REWRITES: [(Fp16Graph, GeluKernel); 5] = [
    (Fp16Graph::Attention, GeluKernel::Gelu),
    (Fp16Graph::Ffn, GeluKernel::Gelu),
    (Fp16Graph::Ffn, GeluKernel::BiasGelu),
    (Fp16Graph::Residual, GeluKernel::Gelu),
    (Fp16Graph::Residual, GeluKernel::BiasGelu),
];

/// The original graph (node by node) and its rewritten one through onnxruntime's CPU provider at fp32, on a directory's
/// references of each bucket: the most any output differs, relative above 1.
fn fused_drift(dir: &Path, level: Fp16Graph, gelu: GeluKernel) -> f32 {
    let r = Reference::load(&dir.join("reference")).unwrap();
    let manifest = manifest(dir);
    let mut worst = 0.0f32;
    for t in cantrip_obs::encode::BUCKETS {
        if !r.positions.iter().any(|e| e.bucket == t) {
            continue;
        }
        let b = bucket_batch(&r, t, 64);
        let bytes = std::fs::read(dir.join(manifest["buckets"][t.to_string()].as_str().unwrap())).unwrap();
        let mut m = pb::ModelProto::decode(&bytes[..]).unwrap();
        rewrite(&mut m, level, gelu);
        let (a, f) = (cpu_run(&bytes, &b, t, false), cpu_run(&m.encode_to_vec(), &b, t, true));
        for k in 0..4 {
            assert_eq!(a[k].len(), f[k].len());
            for (x, y) in a[k].iter().zip(&f[k]) {
                assert!(x.is_finite() == y.is_finite(), "output {k} at {t} tokens: {x} and {y}");
                worst = worst.max((x - y).abs() / x.abs().max(1.0));
            }
        }
    }
    worst
}

#[test]
fn a_padding_key_weighs_nothing_at_the_floor_as_at_masked() {
    let r = references();
    for t in cantrip_obs::encode::BUCKETS {
        if !r.positions.iter().any(|e| e.bucket == t) {
            continue;
        }
        let b = bucket_batch(&r, t, 64);
        let padded = (0..b.rows).filter(|&i| b.kind[i * t..(i + 1) * t].contains(&0)).count();
        assert!(padded > 0, "rows with padding keys at {t} tokens");
        let mut m = model(&tiny().join(format!("model-t{t}.onnx")));
        gpu::fuse_attention(&mut m).unwrap();
        let floored = m.encode_to_vec();
        let floors = m.graph.as_mut().unwrap().initializer.iter_mut().filter(|i| i.name.ends_with("__floor"));
        for f in floors {
            f.raw_data = cantrip_ai::eval::MASKED.to_le_bytes().to_vec();
        }
        let (a, f) = (cpu_run(&floored, &b, t, true), cpu_run(&m.encode_to_vec(), &b, t, true));
        for k in 0..4 {
            let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&a[k]), bits(&f[k]), "output {k} at {t} tokens ({padded} rows padded)");
        }
    }
}

#[test]
fn the_fused_graph_is_the_graph_on_the_cpu() {
    for (level, gelu) in REWRITES {
        let drift = fused_drift(&tiny(), level, gelu);
        println!("tiny: {} ({}) drifts {drift:e} at fp32", level.name(), gelu.name());
        assert!(drift < 1e-5, "{}: {drift}", level.name());
    }
}

/// `fused_drift` on the model directories `CANTRIP_FUSE_MODELS` names (`;`-separated): trained models, which the tests
/// do not hold.
#[test]
#[ignore]
fn trained_models_fused_are_the_models_on_the_cpu() {
    let dirs = std::env::var("CANTRIP_FUSE_MODELS").expect("CANTRIP_FUSE_MODELS names the model directories");
    for dir in dirs.split(';').filter(|d| !d.is_empty()) {
        for (level, gelu) in REWRITES {
            let drift = fused_drift(Path::new(dir), level, gelu);
            println!("{dir}: {} ({}) drifts {drift:e} at fp32", level.name(), gelu.name());
            assert!(drift < 1e-4, "{dir} {}: {drift}", level.name());
        }
    }
}

/// The first node of the graph that `is` picks.
fn find(g: &pb::GraphProto, is: impl Fn(&pb::NodeProto) -> bool) -> usize {
    g.node.iter().position(is).unwrap()
}

fn perm(n: &pb::NodeProto) -> Vec<i64> {
    n.attribute.iter().find(|a| a.name == "perm").map(|a| a.ints.clone()).unwrap_or_default()
}

/// A node `Identity(tensor)`, a second reader of it.
fn reader(tensor: &str) -> pb::NodeProto {
    pb::NodeProto {
        input: vec![tensor.to_string()],
        output: vec![format!("{tensor}__too")],
        op_type: "Identity".into(),
        ..Default::default()
    }
}

/// An int64 initializer, its name.
fn ints(g: &mut pb::GraphProto, name: &str, values: &[i64]) -> String {
    g.initializer.push(pb::TensorProto {
        name: name.into(),
        dims: vec![values.len() as i64],
        data_type: 7,
        int64_data: values.to_vec(),
        ..Default::default()
    });
    name.into()
}

#[test]
fn an_attention_out_of_pattern_is_left_alone() {
    let original = model(&tiny().join("model-t32.onnx"));
    let softmax = |g: &pb::GraphProto| find(g, |n| n.op_type == "Softmax");
    let split = |g: &pb::GraphProto| find(g, |n| n.op_type == "Split" && n.name.contains("attention"));
    // The heads' outputs' Reshape: the one reading the Transpose back to [B, T, heads, head].
    let back = |g: &pb::GraphProto| {
        let t = find(g, |n| n.op_type == "Transpose" && perm(n) == [0, 2, 1, 3]);
        let out = g.node[t].output[0].clone();
        find(g, |n| n.op_type == "Reshape" && n.input[0] == out)
    };
    type Mutation = Box<dyn Fn(&mut pb::GraphProto)>;
    let cases: Vec<(&str, Mutation)> = vec![
        ("the softmax read twice", Box::new(move |g| {
            let probs = g.node[softmax(g)].output[0].clone();
            g.node.push(reader(&probs));
        })),
        ("a softmax over another axis", Box::new(move |g| {
            let s = softmax(g);
            g.node[s].attribute.retain(|a| a.name != "axis");
            g.node[s].attribute.push(pb::AttributeProto {
                name: "axis".into(),
                r#type: pb::attribute_proto::AttributeType::Int as i32,
                i: 2,
                ..Default::default()
            });
        })),
        ("the scores scaled by a tensor", Box::new(|g| {
            let mul = find(g, |n| n.op_type == "Mul" && n.name.contains("attention"));
            let scale = g.node[mul].input[1].clone();
            let at = find(g, |n| n.output.contains(&scale));
            g.node[at].op_type = "Identity".into();
            g.node[at].attribute.clear();
            g.node[at].input = vec!["nums".into()];
        })),
        ("q and k swapped in the split", Box::new(move |g| {
            let s = split(g);
            g.node[s].output.swap(0, 1);
        })),
        ("q read twice", Box::new(move |g| {
            let part = g.node[split(g)].output[0].clone();
            let q = g.node[find(g, |n| n.op_type == "Squeeze" && n.input[0] == part)].output[0].clone();
            g.node.push(reader(&q));
        })),
        ("a split part read twice", Box::new(move |g| {
            let part = g.node[split(g)].output[2].clone();
            g.node.push(reader(&part));
        })),
        ("v squeezed on another axis", Box::new(move |g| {
            let part = g.node[split(g)].output[2].clone();
            let sq = find(g, |n| n.op_type == "Squeeze" && n.input[0] == part);
            g.node[sq].input[1] = ints(g, "axis_one", &[1]);
        })),
        ("qkv transposed otherwise", Box::new(|g| {
            let t = find(g, |n| n.op_type == "Transpose" && perm(n).len() == 5);
            for a in g.node[t].attribute.iter_mut().filter(|a| a.name == "perm") {
                a.ints = vec![2, 0, 1, 3, 4];
            }
        })),
        ("the heads' outputs reshaped to another width", Box::new(move |g| {
            let r = back(g);
            let shape = ints(g, "wider", &[-1, 16, 32]);
            g.node[r].input[1] = shape;
        })),
        ("the heads' outputs reshaped to no constant", Box::new(move |g| {
            let r = back(g);
            g.node[r].input.truncate(1);
        })),
        ("a mask not at MASKED", Box::new(|g| {
            let c = find(g, |n| {
                n.op_type == "Constant"
                    && n.attribute.iter().any(|a| {
                        a.t.as_ref().is_some_and(|t| t.raw_data == (-1e9f32).to_le_bytes() || t.float_data == [-1e9f32])
                    })
            });
            for a in g.node[c].attribute.iter_mut() {
                if let Some(t) = a.t.as_mut() {
                    t.raw_data = (-5e4f32).to_le_bytes().to_vec();
                    t.float_data.clear();
                }
            }
        })),
        ("an Add of one input", Box::new(move |g| {
            let input = g.node[softmax(g)].input[0].clone();
            let add = find(g, |n| n.output.contains(&input));
            g.node[add].input.truncate(1);
        })),
    ];
    for (what, mutate) in cases {
        let mut m = original.clone();
        mutate(m.graph.as_mut().unwrap());
        let before = m.encode_to_vec();
        let e = gpu::fuse_attention(&mut m).expect_err(what);
        println!("{what}: {e}");
        assert_eq!(m.encode_to_vec(), before, "{what}: the graph is untouched");
    }
}

/// Each tensor's element type, as far as these graphs' operators go: what a Cast casts to, a Where's branches', and
/// otherwise its first input's.
fn element_types(g: &pb::GraphProto) -> std::collections::HashMap<String, i32> {
    let mut types = std::collections::HashMap::new();
    for i in &g.input {
        if let Some(pb::type_proto::Value::TensorType(t)) = i.r#type.as_ref().and_then(|t| t.value.as_ref()) {
            types.insert(i.name.clone(), t.elem_type);
        }
    }
    for i in &g.initializer {
        types.insert(i.name.clone(), i.data_type);
    }
    for n in &g.node {
        let ty = match n.op_type.as_str() {
            "Constant" => n.attribute.iter().find(|a| a.name == "value").and_then(|a| a.t.as_ref()).unwrap().data_type,
            "Cast" => n.attribute.iter().find(|a| a.name == "to").unwrap().i as i32,
            "Where" => types[&n.input[1]],
            _ => types[&n.input[0]],
        };
        for o in &n.output {
            types.insert(o.clone(), ty);
        }
    }
    types
}

#[test]
fn every_level_of_the_half_graph_passes_half_tensors_directly() {
    const FLOAT: i32 = 1;
    const FLOAT16: i32 = 10;
    let path = tiny().join("model-t64.onnx");
    let lns = count(model(&path).graph.as_ref().unwrap(), "LayerNormalization");
    for level in Fp16Graph::ALL {
        for gelu in [GeluKernel::Gelu, GeluKernel::BiasGelu] {
            let options = Options { precision: Precision::Fp16, fp16: level, gelu, ..Options::default() };
            let (bytes, done) = gpu::graph_bytes(&path, &options).unwrap();
            assert!(!done.refused(), "{}: {}", level.name(), done.summary());
            let Some(Ok(kept)) = done.inputs else { panic!("{}: {}", level.name(), done.summary()) };
            let m = pb::ModelProto::decode(&bytes[..]).unwrap();
            let g = m.graph.as_ref().unwrap();
            let what = format!("{} ({})", level.name(), gelu.name());
            let types = element_types(g);
            let made: std::collections::HashMap<&str, &pb::NodeProto> =
                g.node.iter().flat_map(|n| n.output.iter().map(move |o| (o.as_str(), n))).collect();
            // The input stage's products and its norm (a fused residual's, at `residual`) stay fp32.
            let stage = gpu::input_stage(&m).unwrap();
            let mut float_products = 0;
            for (index, n) in g.node.iter().enumerate() {
                let inputs: Vec<i32> = n.input.iter().filter(|i| !i.is_empty()).map(|i| types[i]).collect();
                match n.op_type.as_str() {
                    "MatMul" | "Gemm" | "MultiHeadAttention" | "BiasGelu" | "Gelu" | "SkipLayerNormalization"
                        if stage.contains(&index) =>
                    {
                        assert!(inputs.iter().all(|&t| t == FLOAT), "{what}: the input stage's {n:?}: {inputs:?}");
                        float_products += matches!(n.op_type.as_str(), "MatMul" | "Gemm") as usize;
                    }
                    "MatMul" | "Gemm" | "MultiHeadAttention" | "BiasGelu" | "Gelu" | "SkipLayerNormalization" => {
                        assert!(inputs.iter().all(|&t| t == FLOAT16), "{what}: {n:?}: {inputs:?}");
                    }
                    "Add" | "Mul" | "Sub" | "Div" | "Max" => {
                        assert!(inputs.iter().all(|&t| t == inputs[0]), "{what}: {n:?}: {inputs:?}")
                    }
                    // No half tensor goes to float and back.
                    "Cast" if types[&n.output[0]] == FLOAT16 => {
                        let from = made.get(n.input[0].as_str());
                        assert!(!from.is_some_and(|f| f.op_type == "Cast"), "{what}: a round trip at {}", n.input[0]);
                    }
                    _ => {}
                }
            }
            for o in &g.output {
                assert_eq!(types[&o.name], FLOAT, "{what}: {}", o.name);
            }
            assert_eq!(float_products, kept, "{what}");
            let ops = |op: &str| count(g, op);
            assert_eq!(ops("MultiHeadAttention") > 0, level >= Fp16Graph::Attention, "{what}");
            let gelus = ops("BiasGelu") + ops("Gelu");
            assert_eq!(gelus > 0, level >= Fp16Graph::Ffn, "{what}");
            if level >= Fp16Graph::Residual {
                assert_eq!((ops("LayerNormalization"), ops("SkipLayerNormalization")), (0, lns), "{what}");
            } else {
                assert_eq!((ops("LayerNormalization"), ops("SkipLayerNormalization")), (lns, 0), "{what}");
            }
            // The attention reads the projections' half outputs and the out projection reads its own.
            for n in g.node.iter().filter(|n| n.op_type == "MultiHeadAttention") {
                assert!(n.input[..3].iter().all(|i| made[i.as_str()].op_type == "MatMul"), "{what}");
                let readers: Vec<_> = g.node.iter().filter(|r| r.input.contains(&n.output[0])).collect();
                assert_eq!(readers.len(), 1, "{what}");
                assert_eq!(readers[0].op_type, "MatMul", "{what}");
            }
        }
    }
}

/// onnxruntime's own rewrites must not undo the GELU kernel chosen: its `BiasGeluFusion` makes a bias Add and a `Gelu`
/// one `BiasGelu`, so a session at `GeluKernel::Gelu` turns it off (`gpu::disabled_optimizers`). onnxruntime's CPU
/// provider (`ORT_DYLIB_PATH`) optimises the fp32 rewrite as the CUDA one does, and writes what it would run.
#[test]
fn the_gelu_kernel_chosen_is_the_one_onnxruntime_runs() {
    use ort::session::Session;
    use ort::session::builder::GraphOptimizationLevel;
    let env = cantrip_ai::eval::ort_environment().unwrap();
    let mut m = model(&tiny().join("model-t32.onnx"));
    rewrite(&mut m, Fp16Graph::Ffn, GeluKernel::Gelu);
    let bytes = m.encode_to_vec();
    let at = |gelu| Options { precision: Precision::Fp16, fp16: Fp16Graph::Ffn, gelu, ..Options::default() };
    assert_eq!(gpu::disabled_optimizers(&at(GeluKernel::BiasGelu)), None);
    let optimized = |off: Option<&str>| {
        let path = std::env::temp_dir().join(format!("cantrip-gelu-{}-{}.onnx", std::process::id(), off.is_some()));
        let mut b = Session::builder(env).unwrap().with_optimization_level(GraphOptimizationLevel::Level3).unwrap();
        if let Some(off) = off {
            b = b.with_disabled_optimizers(off).unwrap();
        }
        let mut b = b.with_optimized_model_path(&path).unwrap();
        b.commit_from_memory(&bytes).unwrap();
        let m = model(&path);
        let _ = std::fs::remove_file(&path);
        let g = m.graph.unwrap();
        (count(&g, "Gelu"), count(&g, "BiasGelu"))
    };
    // The rewrite's Gelus (the heads' GELUs after a Gemm, which onnxruntime fuses into Gelus itself, are the others).
    let ours = count(m.graph.as_ref().unwrap(), "Gelu");
    let (gelus, biased) = optimized(None);
    assert_eq!(biased, ours, "onnxruntime fuses the bias and Gelu by itself: {gelus} Gelu, {biased} BiasGelu");
    let (gelus, biased) = optimized(gpu::disabled_optimizers(&at(GeluKernel::Gelu)));
    assert!(gelus >= ours && biased == 0, "with the fusion off the Gelus stay: {gelus} Gelu, {biased} BiasGelu");
}

#[test]
fn a_gelu_or_a_norm_out_of_pattern_is_left_alone() {
    let original = model(&tiny().join("model-t32.onnx"));
    type Mutation = Box<dyn Fn(&mut pb::GraphProto)>;
    let gelus: Vec<(&str, Mutation)> = vec![
        ("divided by another constant", Box::new(|g| {
            let div = find(g, |n| n.op_type == "Div" && n.name.contains("ffn"));
            let c = g.node[div].input[1].clone();
            let at = find(g, |n| n.output.contains(&c));
            for a in g.node[at].attribute.iter_mut() {
                if let Some(t) = a.t.as_mut() {
                    t.raw_data = 1.5f32.to_le_bytes().to_vec();
                    t.float_data.clear();
                }
            }
        })),
        ("divided by √2 off by an ulp", Box::new(|g| {
            let div = find(g, |n| n.op_type == "Div" && n.name.contains("ffn"));
            let c = g.node[div].input[1].clone();
            let at = find(g, |n| n.output.contains(&c));
            let off = f32::from_bits(std::f32::consts::SQRT_2.to_bits() + 1);
            for a in g.node[at].attribute.iter_mut() {
                if let Some(t) = a.t.as_mut() {
                    t.raw_data = off.to_le_bytes().to_vec();
                    t.float_data.clear();
                }
            }
        })),
        ("erf read twice", Box::new(|g| {
            let e = g.node[find(g, |n| n.op_type == "Erf")].output[0].clone();
            g.node.push(reader(&e));
        })),
        ("x read a third time", Box::new(|g| {
            let div = find(g, |n| n.op_type == "Div" && n.name.contains("ffn"));
            let x = g.node[div].input[0].clone();
            g.node.push(reader(&x));
        })),
    ];
    for (what, mutate) in gelus {
        let mut m = original.clone();
        mutate(m.graph.as_mut().unwrap());
        let before = m.encode_to_vec();
        let e = gpu::fuse_gelu(&mut m, GeluKernel::BiasGelu).expect_err(what);
        println!("{what}: {e}");
        assert_eq!(m.encode_to_vec(), before, "{what}: the graph is untouched");
    }
    // A bias that is no vector is no bias the fused node takes: that GELU stays as exported, the others fuse.
    let mut m = original.clone();
    let g = m.graph.as_mut().unwrap();
    let div = find(g, |n| n.op_type == "Div" && n.name.contains("ffn"));
    let x = g.node[div].input[0].clone();
    let add = find(g, |n| n.output.contains(&x));
    let bias = g.node[add].input.iter().find(|i| g.initializer.iter().any(|t| &t.name == *i)).unwrap().clone();
    let t = g.initializer.iter_mut().find(|t| t.name == bias).unwrap();
    t.dims = vec![1, t.dims[0]];
    let fused = gpu::fuse_gelu(&mut m, GeluKernel::BiasGelu).unwrap().fused;
    let g = m.graph.as_ref().unwrap();
    assert!(g.node.iter().any(|n| n.op_type == "Erf" && n.name.contains("ffn")), "the ffn's GELU is as exported");
    assert!(fused >= 1, "the policy head's GELU still fuses");
    let norms: Vec<(&str, Mutation)> = vec![
        ("a norm of another axis", Box::new(|g| {
            let l = find(g, |n| n.op_type == "LayerNormalization");
            g.node[l].attribute.retain(|a| a.name != "axis");
            g.node[l].attribute.push(pb::AttributeProto {
                name: "axis".into(),
                r#type: pb::attribute_proto::AttributeType::Int as i32,
                i: 1,
                ..Default::default()
            });
        })),
        ("a norm of a residual read before it", Box::new(|g| {
            let l = find(g, |n| n.op_type == "LayerNormalization");
            let x = g.node[l].input[0].clone();
            g.node.insert(l, reader(&x));
        })),
        ("a norm whose statistics are read", Box::new(|g| {
            let l = find(g, |n| n.op_type == "LayerNormalization");
            g.node[l].output.push("mean".into());
        })),
        ("a residual Add that broadcasts", Box::new(|g| {
            // The second norm's residual Add with one addend summed over the tokens: [B, 1, width] + [B, T, width].
            let l = find(g, |n| n.op_type == "LayerNormalization" && n.name.contains("norm2"));
            let x = g.node[l].input[0].clone();
            let r = find(g, |n| n.output.contains(&x));
            let addend = g.node[r].input[0].clone();
            let axis = ints(g, "token_axis", &[1]);
            let summed = format!("{addend}__summed");
            g.node.insert(r, pb::NodeProto {
                input: vec![addend, axis],
                output: vec![summed.clone()],
                op_type: "ReduceSum".into(),
                attribute: vec![pb::AttributeProto {
                    name: "keepdims".into(),
                    r#type: pb::attribute_proto::AttributeType::Int as i32,
                    i: 1,
                    ..Default::default()
                }],
                ..Default::default()
            });
            g.node[r + 1].input[0] = summed;
        })),
    ];
    for (what, mutate) in norms {
        let mut m = original.clone();
        mutate(m.graph.as_mut().unwrap());
        let before = m.encode_to_vec();
        let shapes = gpu::tensor_shapes(&m).unwrap();
        let e = gpu::fuse_norms(&mut m, &shapes).expect_err(what);
        println!("{what}: {e}");
        assert_eq!(m.encode_to_vec(), before, "{what}: the graph is untouched");
    }
}

#[test]
fn a_norm_without_an_epsilon_fuses_with_onnx_s() {
    let mut m = model(&tiny().join("model-t32.onnx"));
    let g = m.graph.as_mut().unwrap();
    let l = find(g, |n| n.op_type == "LayerNormalization");
    g.node[l].attribute.retain(|a| a.name != "epsilon");
    let out = g.node[l].output[0].clone();
    let shapes = gpu::tensor_shapes(&m).unwrap();
    gpu::fuse_norms(&mut m, &shapes).unwrap();
    let g = m.graph.as_ref().unwrap();
    for n in g.node.iter().filter(|n| n.op_type == "SkipLayerNormalization") {
        let epsilon = n.attribute.iter().find(|a| a.name == "epsilon").expect("an epsilon on every fused norm").f;
        if n.output[0] == out {
            assert_eq!(epsilon, 1e-5, "ONNX's default, not the fused node's 1e-12");
        }
    }
}

/// `half_matmuls` as it was before the fused rewrites (8f0ca456f's parent): every MatMul and Gemm cast in and out, no
/// cast left out and nothing else touched.
fn half_matmuls_before(model: &mut pb::ModelProto, skip: &std::collections::HashSet<usize>) {
    let g = model.graph.as_mut().unwrap();
    let cast = |input: &str, output: &str, to: i64| pb::NodeProto {
        input: vec![input.into()],
        output: vec![output.into()],
        op_type: "Cast".into(),
        attribute: vec![pb::AttributeProto {
            name: "to".into(),
            r#type: pb::attribute_proto::AttributeType::Int as i32,
            i: to,
            ..Default::default()
        }],
        ..Default::default()
    };
    let initializers: std::collections::HashMap<String, usize> =
        g.initializer.iter().enumerate().map(|(i, t)| (t.name.clone(), i)).collect();
    let (mut halves, mut casts) = (std::collections::HashMap::new(), std::collections::HashMap::new());
    let (mut added, mut nodes) = (Vec::new(), Vec::new());
    for (index, mut n) in std::mem::take(&mut g.node).into_iter().enumerate() {
        if skip.contains(&index) || !(n.domain.is_empty() && (n.op_type == "MatMul" || n.op_type == "Gemm")) {
            nodes.push(n);
            continue;
        }
        for input in n.input.iter_mut().filter(|i| !i.is_empty()) {
            let half = format!("{input}__fp16");
            if let Some(&i) = initializers.get(input.as_str()) {
                halves.entry(input.clone()).or_insert_with(|| {
                    let t = &g.initializer[i];
                    let values: Vec<f32> = if t.raw_data.is_empty() {
                        t.float_data.clone()
                    } else {
                        t.raw_data.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()
                    };
                    added.push(pb::TensorProto {
                        name: half.clone(),
                        dims: t.dims.clone(),
                        data_type: 10,
                        raw_data: values.iter().flat_map(|&x| cantrip_obs::encode::f16_bits(x).to_le_bytes()).collect(),
                        ..Default::default()
                    });
                });
            } else {
                casts.entry(input.clone()).or_insert_with(|| nodes.push(cast(input, &half, 10)));
            }
            *input = half;
        }
        let out = n.output[0].clone();
        n.output[0] = format!("{out}__fp16out");
        let back = cast(&n.output[0], &out, 1);
        nodes.push(n);
        nodes.push(back);
    }
    g.node = nodes;
    g.initializer.extend(added);
}

/// Each half MatMul and Gemm of a half graph computed in float: its inputs cast to float and its output back to half,
/// as tract can run it (it has no half products).
fn products_in_float(model: &mut pb::ModelProto) {
    let g = model.graph.as_mut().unwrap();
    let cast = |input: &str, output: &str, to: i64| pb::NodeProto {
        input: vec![input.into()],
        output: vec![output.into()],
        op_type: "Cast".into(),
        attribute: vec![pb::AttributeProto {
            name: "to".into(),
            r#type: pb::attribute_proto::AttributeType::Int as i32,
            i: to,
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut nodes = Vec::new();
    for (k, mut n) in std::mem::take(&mut g.node).into_iter().enumerate() {
        // A half product reads half tensors (a cast's, a half initializer, a half output); a float one (the input
        // stage's) stays as it is.
        let half = n.input.first().is_some_and(|i| i.ends_with("__fp16") || i.ends_with("__fp16out"));
        if !((n.op_type == "MatMul" || n.op_type == "Gemm") && half) {
            nodes.push(n);
            continue;
        }
        for (j, input) in n.input.iter_mut().enumerate().filter(|(_, i)| !i.is_empty()) {
            let wide = format!("{input}__float{k}_{j}");
            nodes.push(cast(input, &wide, 1));
            *input = wide;
        }
        let out = n.output[0].clone();
        n.output[0] = format!("{out}__float");
        let back = cast(&n.output[0], &out, 10);
        nodes.push(n);
        nodes.push(back);
    }
    g.node = nodes;
}

#[test]
fn the_products_level_computes_bitwise_what_it_did() {
    let r = references();
    for t in cantrip_obs::encode::BUCKETS {
        if !r.positions.iter().any(|e| e.bucket == t) {
            continue;
        }
        let b = bucket_batch(&r, t, 64);
        let path = tiny().join(format!("model-t{t}.onnx"));
        let (mut now, mut before) = (model(&path), model(&path));
        gpu::half_matmuls(&mut now).unwrap();
        // The old pass, the input stage left as the new one leaves it: what differs is the casts left out and the
        // moves on half tensors, which must not change a bit.
        let stage = gpu::input_stage(&before).unwrap();
        half_matmuls_before(&mut before, &stage);
        // tract runs each graph node by node as written, every cast a rounding, each half product computed in float
        // between casts the same way in both. (onnxruntime's CPU provider cannot say: it takes out the casts it finds
        // redundant, roundings included.)
        products_in_float(&mut now);
        products_in_float(&mut before);
        let a = tract_run_as(&now.encode_to_vec(), &b, false, false);
        let f = tract_run_as(&before.encode_to_vec(), &b, false, false);
        for k in 0..4 {
            let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&floats(&a[k])), bits(&floats(&f[k])), "output {k} at {t} tokens");
        }
        // And the casts do round: the half graph is not the float one.
        let float = tract_run_as(&std::fs::read(tiny().join(format!("model-t{t}.onnx"))).unwrap(), &b, false, false);
        assert_ne!(floats(&a[0]), floats(&float[0]), "the half graph's value at {t} tokens is the float graph's");
    }
}

fn engine(precision: Precision) -> Arc<Engine> {
    engine_with(precision, Fp16Graph::Attention, GeluKernel::Gelu)
}

fn engine_with(precision: Precision, fp16: Fp16Graph, gelu: GeluKernel) -> Arc<Engine> {
    let options = Options {
        precision,
        batch: [16, 8, 6, 4],
        // Smaller shapes too, each kept where the probe finds it exact: the tests below hold every row to itself
        // whatever shape its batch ran at.
        shapes: std::array::from_fn(|_| vec![2, 3, 4, 8]),
        fp16,
        gelu,
        ..Options::default()
    };
    Arc::new(Engine::start(&[&tiny()], options).unwrap())
}

/// The engines the GPU tests run: fp32, tf32, and fp16 at each level of its graph (and either GELU kernel).
fn engines() -> Vec<(String, Precision, Arc<Engine>)> {
    let mut out = vec![
        ("fp32".to_string(), Precision::Fp32, engine(Precision::Fp32)),
        ("tf32".to_string(), Precision::Tf32, engine(Precision::Tf32)),
    ];
    for level in Fp16Graph::ALL {
        for gelu in [GeluKernel::Gelu, GeluKernel::BiasGelu] {
            if level < Fp16Graph::Ffn && gelu == GeluKernel::BiasGelu {
                continue;
            }
            let name = format!("fp16 {} ({})", level.name(), gelu.name());
            out.push((name, Precision::Fp16, engine_with(Precision::Fp16, level, gelu)));
        }
    }
    out
}

#[test]
#[ignore]
fn the_engine_makes_the_references() {
    for (name, precision, e) in engines() {
        let c = gpu::check(&e, 0).unwrap();
        println!("{name}: {c:?}");
        assert_eq!((c.non_finite, c.illegal_argmax), (0, 0), "{name}: every output finite, every best option legal");
        if precision == Precision::Fp32 {
            assert!(c.pass, "{c:?}");
            assert!(c.value < 1e-3 && c.logits < 1e-3 && c.swing_reduced < 1e-3 && c.dead < 1e-3);
            assert_eq!(c.same_argmax, 1.0);
        } else {
            // The values are held to the reduced bar. The best option is held only where the reference decides it:
            // the tiny model is untrained, and 6 of its 19 references with two legal options or more have their top
            // two logits within 1.4e-4 of each other, ties any rounding settles either way (a real model is held to
            // `SAME_ARGMAX` by `check` at start). Every reference whose top two are `DECIDED` apart (13 of them) keeps
            // its best option.
            assert!(c.value < gpu::REDUCED_VALUE && c.dead < gpu::REDUCED_VALUE, "{c:?}");
            assert!(
                c.tv_mean < gpu::REDUCED_TV_MEAN
                    && c.tv_p99 < gpu::REDUCED_TV_P99
                    && c.tv_max < gpu::REDUCED_TV_MAX
                    && c.decided_flips == 0
                    && c.swing_reduced < gpu::REDUCED_SWING,
                "{c:?}"
            );
            let (decided, ties, flips) = argmax_away_from_ties(&e);
            println!("{name}: the best option the same at {decided} decided references; {ties} ties");
            assert!(decided >= 13, "{name}: {decided} decided references, where the fixture has 13");
            assert!(flips.is_empty(), "{name}: a decided reference's best option changed at {flips:?}");
        }
    }
}

/// How far apart a reference's top two legal logits are for its best option to count as decided: from the reference
/// alone (its near ties are within 1.4e-4).
const DECIDED: f32 = 1e-3;

/// The tiny model's references through the engine: how many have a decided best option (their top two legal logits
/// `DECIDED` apart or more), how many are ties, and the decided ones whose best option changed.
fn argmax_away_from_ties(e: &Arc<Engine>) -> (usize, usize, Vec<usize>) {
    let r = references();
    let ev = GpuEvaluator::new(e.clone(), 0, &own(&tiny())).unwrap();
    let (mut decided, mut ties, mut flips) = (0, 0, Vec::new());
    for chunk in (0..r.positions.len()).collect::<Vec<_>>().chunks(64) {
        let es: Vec<&Encoded> = chunk.iter().map(|&i| &r.positions[i]).collect();
        let out = ev.evaluate(&FeatureBatch::new(&es)).unwrap();
        for (j, &i) in chunk.iter().enumerate() {
            let p = &r.positions[i];
            let legal: Vec<usize> = (0..p.options).filter(|&k| p.legal[p.first_option + k]).collect();
            if legal.len() < 2 {
                continue;
            }
            let best =
                |x: &[f32]| legal.iter().copied().max_by(|&a, &b| x[a].total_cmp(&x[b]).then(b.cmp(&a))).unwrap();
            let want = &r.policy[i];
            let top = best(want);
            let second = legal.iter().filter(|&&k| k != top).map(|&k| want[k]).fold(f32::MIN, f32::max);
            if want[top] - second >= DECIDED {
                decided += 1;
                if best(&out.policy[j]) != top {
                    flips.push(i);
                }
            } else {
                ties += 1;
            }
        }
    }
    (decided, ties, flips)
}

#[test]
#[ignore]
fn a_row_is_the_same_whatever_it_is_batched_with() {
    let r = references();
    for (name, _, e) in engines() {
        let ev = GpuEvaluator::new(e.clone(), 0, &own(&tiny())).unwrap();
        let all: Vec<&Encoded> = r.positions.iter().collect();
        let alone: Vec<_> = all.iter().map(|p| ev.evaluate_positions(&[p]).unwrap()).collect();
        // Every position again, in batches of 7 (several per GPU batch, at changing slots) and in reverse.
        for (pass, order) in [(0, (0..all.len()).collect::<Vec<_>>()), (1, (0..all.len()).rev().collect())] {
            for chunk in order.chunks(7) {
                let es: Vec<&Encoded> = chunk.iter().map(|&i| all[i]).collect();
                let out = ev.evaluate_positions(&es).unwrap();
                let padded = ev.evaluate(&FeatureBatch::new(&es)).unwrap();
                assert_eq!(out, padded, "evaluate_positions is evaluate");
                for (j, &i) in chunk.iter().enumerate() {
                    let a = &alone[i];
                    assert_eq!(out.value[j].to_bits(), a.value[0].to_bits(), "{name} pass {pass} row {i}");
                    assert_eq!(
                        out.policy[j].iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                        a.policy[0].iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                    );
                    assert_eq!(out.aux[j][0].to_bits(), a.aux[0][0].to_bits());
                    assert_eq!(out.aux[j][1].to_bits(), a.aux[0][1].to_bits());
                }
            }
        }
    }
}

#[test]
#[ignore]
fn many_callers_share_the_engine() {
    let r = references();
    let e = engine(Precision::Fp32);
    let table = own(&tiny());
    let want: Vec<_> = {
        let ev = GpuEvaluator::new(e.clone(), 0, &table).unwrap();
        r.positions.iter().map(|p| ev.evaluate_positions(&[p]).unwrap()).collect()
    };
    std::thread::scope(|s| {
        for c in 0..16 {
            let (e, table, r, want) = (e.clone(), &table, &r, &want);
            s.spawn(move || {
                let ev = GpuEvaluator::new(e, 0, table).unwrap();
                for round in 0..20 {
                    let at = (c * 13 + round * 5) % r.positions.len();
                    let n = 1 + (c + round) % 6;
                    let idx: Vec<usize> = (0..n).map(|k| (at + k) % r.positions.len()).collect();
                    let es: Vec<&Encoded> = idx.iter().map(|&i| &r.positions[i]).collect();
                    let out = ev.evaluate_positions(&es).unwrap();
                    for (j, &i) in idx.iter().enumerate() {
                        assert_eq!(out.value[j].to_bits(), want[i].value[0].to_bits());
                        assert_eq!(out.policy[j], want[i].policy[0]);
                    }
                }
            });
        }
    });
    let s = e.stats();
    assert_eq!(s.requests as usize, r.positions.len() + 16 * 20);
}

/// The references' own outputs as an evaluator's (`Outputs`).
fn as_outputs(r: &Reference) -> cantrip_ai::Outputs {
    cantrip_ai::Outputs {
        value: r.value.clone(),
        policy: r.policy.clone(),
        aux: r.aux.clone(),
    }
}

#[test]
fn the_check_fails_a_nan_and_a_best_option_lost() {
    let r = references();
    let exact = gpu::score(&r, &as_outputs(&r), Precision::Fp16);
    assert!(exact.pass && exact.non_finite == 0 && exact.illegal_argmax == 0, "{exact:?}");
    let with_legal = (0..r.positions.len())
        .find(|&i| {
            let p = &r.positions[i];
            (0..p.options).filter(|&k| p.legal[p.first_option + k]).count() >= 2
        })
        .unwrap();
    // One reference's legal logits NaN: every drift skips it (max ignores NaN), yet the check fails.
    let mut nan = as_outputs(&r);
    for x in nan.policy[with_legal].iter_mut().filter(|x| **x > cantrip_ai::eval::MASKED / 2.0) {
        *x = f32::NAN;
    }
    let c = gpu::score(&r, &nan, Precision::Fp16);
    assert!(!c.pass && c.non_finite == 1 && c.illegal_argmax == 1, "{c:?}");
    assert!(c.logits == 0.0 && c.same_argmax > 0.99, "the drifts did not see it: {c:?}");
    // A NaN value alone.
    let mut value = as_outputs(&r);
    value.value[0] = f32::NAN;
    let c = gpu::score(&r, &value, Precision::Fp32);
    assert!(!c.pass && c.non_finite == 1 && c.illegal_argmax == 0, "{c:?}");
    // Every legal option masked: finite, but the best option is lost.
    let mut masked = as_outputs(&r);
    for x in masked.policy[with_legal].iter_mut() {
        *x = cantrip_ai::eval::MASKED;
    }
    let c = gpu::score(&r, &masked, Precision::Fp16);
    assert!(!c.pass && c.non_finite == 0 && c.illegal_argmax == 1, "{c:?}");
}

/// A graph's outputs through tract on `b`, as an evaluator gives them (each position's option logits).
fn tract_outputs(bytes: &[u8], b: &FeatureBatch, es: &[Encoded], optimized: bool) -> cantrip_ai::Outputs {
    let out = tract_run_as(bytes, b, false, optimized);
    let (value, policy, aux) = (floats(&out[0]), floats(&out[1]), floats(&out[2]));
    cantrip_ai::Outputs {
        value,
        policy: es
            .iter()
            .enumerate()
            .map(|(j, e)| policy[j * b.tokens + e.first_option..j * b.tokens + e.first_option + e.options].to_vec())
            .collect(),
        aux: aux.chunks_exact(2).map(|a| [a[0], a[1]]).collect(),
    }
}

/// gen2 met a card with ~131,700 +1/+1 counters: its numbers, scaled linearly, were 16,464, the numbers' projection
/// reached 47,700, and the pointers' projection that sums such tokens left fp16's range: NaN on every legal logit. Here
/// the tiny model (smaller weights) meets numbers of 1e5 on one card token. Its fp16 graph runs through tract
/// (`products_in_float`: each product in float between the graph's casts, every cast rounding as on the GPU), and the
/// server's check (`gpu::score`) holds it to the fp32 graph: with every product halved it fails (outputs not finite, the
/// best option lost); with the input stage in fp32 (`half_matmuls`) it passes.
#[test]
fn a_huge_number_overflows_a_halved_input_stage_and_not_an_fp32_one() {
    let r = references();
    let t = 32;
    let mut es: Vec<Encoded> = r.positions.iter().filter(|e| e.bucket == t).take(8).cloned().collect();
    // A token with numbers of its own (a card), else token 1.
    let e = &es[0];
    let k = (0..e.len).find(|&k| e.kind[k] != 0 && e.nums[k * NUMS..(k + 1) * NUMS].iter().any(|&x| x != 0.0));
    let k = k.unwrap_or(1);
    for x in es[0].nums[k * NUMS..(k + 1) * NUMS].iter_mut() {
        *x = 1e5;
    }
    let refs: Vec<&Encoded> = es.iter().collect();
    let b = FeatureBatch::new(&refs);
    let path = tiny().join(format!("model-t{t}.onnx"));
    let exact = tract_outputs(&std::fs::read(&path).unwrap(), &b, &es, true);
    let want = Reference {
        positions: es.clone(),
        value: exact.value.clone(),
        aux: exact.aux.clone(),
        policy: exact.policy.clone(),
        decoded: vec![0; es.len()],
    };
    assert!(want.value.iter().all(|x| x.is_finite()), "fp32 holds it");
    let half = |stage_fp32: bool| {
        let mut m = model(&path);
        if stage_fp32 {
            gpu::half_matmuls(&mut m).unwrap();
        } else {
            half_matmuls_before(&mut m, &std::collections::HashSet::new());
        }
        products_in_float(&mut m);
        gpu::score(&want, &tract_outputs(&m.encode_to_vec(), &b, &es, false), Precision::Fp16)
    };
    let every = half(false);
    println!("every product halved: {every:?}");
    assert!(!every.pass && every.non_finite > 0, "{every:?}");
    let stage = half(true);
    println!("the input stage fp32: {stage:?}");
    assert!(stage.pass && stage.non_finite == 0 && stage.illegal_argmax == 0, "{stage:?}");
}

// --- Batch shapes ----------------------------------------------------------------------------------------------------

/// `e` cut to its first `t` tokens: the rest padding, a pointer past them none. A long reference stands in for a row of
/// a bucket few references run at.
fn cut(e: &Encoded, t: usize) -> Encoded {
    let mut c = e.clone();
    for k in t.min(c.bucket)..c.bucket {
        c.kind[k] = 0;
        c.card[k] = cantrip_obs::encode::UNKNOWN_CARD;
        c.cats[k * CATS..(k + 1) * CATS].fill(0);
        c.ptrs[k * PTRS..(k + 1) * PTRS].fill(cantrip_obs::encode::NO_POINTER);
        c.nums[k * NUMS..(k + 1) * NUMS].fill(0.0);
        c.legal[k] = false;
    }
    for p in &mut c.ptrs {
        if (*p as usize) >= t {
            *p = cantrip_obs::encode::NO_POINTER;
        }
    }
    c.len = c.len.min(t);
    c.origin.truncate(c.len);
    c
}

/// The tokens a position runs with (its last token that is not padding; no reference points past its tokens).
fn real_len(e: &Encoded) -> usize {
    Rows::from_positions(&[e], &[]).unwrap().lengths[0]
}

/// The bucket a position runs at in a model with every bucket's graph.
fn bucket_index(e: &Encoded) -> usize {
    gpu::GRAPH_BUCKETS.iter().position(|&t| t >= real_len(e).max(1)).unwrap_or(3)
}

/// Rows for each bucket: the references that run at it, then longer ones cut to it, `per` in all.
fn rows_by_bucket(r: &Reference, per: usize) -> [Vec<Encoded>; 4] {
    std::array::from_fn(|k| {
        let t = gpu::GRAPH_BUCKETS[k];
        let mut out: Vec<Encoded> = r.positions.iter().filter(|e| bucket_index(e) == k).take(per).cloned().collect();
        let more: Vec<Encoded> = r
            .positions
            .iter()
            .filter(|e| bucket_index(e) > k)
            .map(|e| cut(e, t))
            .filter(|c| bucket_index(c) == k)
            .take(per - out.len())
            .collect();
        out.extend(more);
        out
    })
}

/// Every output a row is answered with, as bits.
fn bits(o: &cantrip_ai::Outputs, j: usize) -> Vec<u32> {
    let mut b = vec![o.value[j].to_bits(), o.aux[j][0].to_bits(), o.aux[j][1].to_bits()];
    b.extend(o.policy[j].iter().map(|x| x.to_bits()));
    b
}

/// Each row alone through an engine of `dir` at `options`.
fn alone(dir: &Path, options: Options, rows: &[&Encoded]) -> Vec<Vec<u32>> {
    let e = Arc::new(Engine::start(&[dir], options).unwrap());
    let ev = GpuEvaluator::new(e, 0, &own(dir)).unwrap();
    rows.iter().map(|p| bits(&ev.evaluate_positions(&[p]).unwrap(), 0)).collect()
}

/// Twelve callers sending requests of 1 to 24 of `all` (rows of every bucket, so batches fill and run partial) to
/// `engine`: each row's outputs bitwise `want`'s.
fn traffic(engine: &Arc<Engine>, dir: &Path, all: &[&Encoded], want: &[Vec<u32>]) {
    let table = own(dir);
    std::thread::scope(|s| {
        for c in 0..12 {
            let (engine, table) = (engine.clone(), &table);
            s.spawn(move || {
                let ev = GpuEvaluator::new(engine, 0, table).unwrap();
                let mut at = c * 37 % all.len();
                for round in 0..40 {
                    let n = 1 + (c * 7 + round * 5) % 24;
                    let idx: Vec<usize> = (0..n).map(|k| (at + k * 11) % all.len()).collect();
                    at = (at + n * 13) % all.len();
                    let es: Vec<&Encoded> = idx.iter().map(|&i| all[i]).collect();
                    let out = ev.evaluate_positions(&es).unwrap();
                    for (j, &i) in idx.iter().enumerate() {
                        assert_eq!(bits(&out, j), want[i], "row {i} among {n} at round {round}");
                    }
                }
            });
        }
    });
}

/// The default engine (one shape a bucket) on trained models (`CANTRIP_SHAPE_MODELS` or `CANTRIP_FUSE_MODELS`), at
/// fp16 and fp32: each row among others, in batches full and partial, is bitwise the row alone.
#[test]
#[ignore]
fn a_trained_model_s_row_is_the_same_whatever_it_is_batched_with() {
    let dirs = std::env::var("CANTRIP_SHAPE_MODELS")
        .or_else(|_| std::env::var("CANTRIP_FUSE_MODELS"))
        .expect("CANTRIP_SHAPE_MODELS or CANTRIP_FUSE_MODELS names trained model directories");
    for dir in dirs.split(';').filter(|d| !d.is_empty()).map(Path::new) {
        let r = Reference::load(&dir.join("reference")).unwrap();
        let by_bucket = rows_by_bucket(&r, 256);
        let all: Vec<&Encoded> = by_bucket.iter().flatten().collect();
        for precision in [Precision::Fp16, Precision::Fp32] {
            let options = Options {
                precision,
                ..Options::default()
            };
            let e = Arc::new(Engine::start(&[dir], options).unwrap());
            let ev = GpuEvaluator::new(e.clone(), 0, &own(dir)).unwrap();
            let want: Vec<Vec<u32>> = all.iter().map(|p| bits(&ev.evaluate_positions(&[p]).unwrap(), 0)).collect();
            drop(ev);
            traffic(&e, dir, &all, &want);
            let st = e.stats();
            println!("{} {}: {} batches, {} rows", dir.display(), precision.name(), st.batches, st.rows);
        }
    }
}

/// The shapes a bucket may run at besides its full one (`Options::shapes`, opt-in), on trained models
/// (`CANTRIP_SHAPE_MODELS`, `;`-separated, or else `CANTRIP_FUSE_MODELS`), at fp16 and fp32:
/// - every shape the engine keeps computes each row alone (an engine whose only shape it is) bitwise as the full shape
///   does, for every row of the bucket the test has (the references that run at it, and longer ones cut to it). A
///   shape it drops may agree on all of them: kernels that add in another order can differ on few rows (the probe
///   saw it; the evidence is the probe's and this test's rows, not a proof, which is why the shapes are opt-in);
/// - under traffic (many callers, requests of 1 to 24 rows of every bucket, so batches run at every shape) each row is
///   bitwise the row alone at the full shape, and batches did run at a smaller shape;
/// - at fp16, the precision self-play serves at, some bucket keeps a smaller shape (on the RTX 4080 the templates
///   line's gen2 and gen4 keep 48, 64 and 80 at T96 and 48 and 56 at T128 with batches 96 and 64).
#[test]
#[ignore]
fn a_row_is_the_same_at_every_shape_the_engine_keeps() {
    let dirs = std::env::var("CANTRIP_SHAPE_MODELS")
        .or_else(|_| std::env::var("CANTRIP_FUSE_MODELS"))
        .expect("CANTRIP_SHAPE_MODELS or CANTRIP_FUSE_MODELS names trained model directories");
    let asked = vec![16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128, 160, 192];
    for dir in dirs.split(';').filter(|d| !d.is_empty()).map(Path::new) {
        let r = Reference::load(&dir.join("reference")).unwrap();
        let by_bucket = rows_by_bucket(&r, 256);
        for precision in [Precision::Fp16, Precision::Fp32] {
            let full = Options {
                precision,
                pipelines: 1,
                ..Options::default()
            };
            let multi_options = Options {
                shapes: std::array::from_fn(|_| asked.clone()),
                ..full.clone()
            };
            let start_multi = || {
                let e = Arc::new(Engine::start(&[dir], multi_options.clone()).unwrap());
                let kept: Vec<Vec<usize>> =
                    (0..4).map(|k| e.models()[0].shapes[k].get().cloned().unwrap_or_default()).collect();
                (e, kept)
            };
            // Started twice (GPU memory is short with the other engines beside it): it keeps the same shapes each time.
            let (multi, kept) = start_multi();
            println!("{} {}: shapes {}", dir.display(), precision.name(), multi.models()[0].shapes_summary());
            drop(multi);
            let all: Vec<&Encoded> = by_bucket.iter().flatten().collect();
            let want = alone(dir, full.clone(), &all);
            // Each shape asked for, alone, on every bucket's rows.
            let mut smaller_kept = 0;
            for &n in &asked {
                let at_n = alone(dir, Options { batch: [n; 4], ..full.clone() }, &all);
                let mut at = 0;
                for (k, rows) in by_bucket.iter().enumerate() {
                    let range = at..at + rows.len();
                    at += rows.len();
                    if n >= full.batch[k] || rows.is_empty() {
                        continue;
                    }
                    let differ = range.clone().filter(|&i| at_n[i] != want[i]).count();
                    let keeps = kept[k].contains(&n);
                    let t = gpu::GRAPH_BUCKETS[k];
                    let name = format!("{} {} T{t} shape {n}", dir.display(), precision.name());
                    assert!(!keeps || differ == 0, "{name}: kept, but {differ} of {} rows differ", rows.len());
                    if !keeps {
                        println!("{name}: dropped; {differ} of {} rows differ", rows.len());
                    }
                    smaller_kept += keeps as usize;
                }
            }
            if precision == Precision::Fp16 {
                assert!(smaller_kept > 0, "{}: no smaller shape kept at fp16", dir.display());
            }
            // Traffic: every row among others, through the engine that runs batches at every shape it kept.
            let (multi, again) = start_multi();
            assert_eq!(again, kept, "the probe kept other shapes the second time");
            traffic(&multi, dir, &all, &want);
            let st = multi.stats();
            let smaller: u64 = (0..4)
                .map(|k| st.batches_by_shape[k].iter().filter(|(n, _)| *n < full.batch[k]).map(|(_, c)| c).sum::<u64>())
                .sum();
            println!(
                "{} {}: {} batches, {smaller} at a smaller shape: {:?}",
                dir.display(),
                precision.name(),
                st.batches,
                st.batches_by_shape
            );
            if kept.iter().enumerate().any(|(k, s)| s.len() > 1 && !by_bucket[k].is_empty()) {
                assert!(smaller > 0, "no batch ran at a smaller shape");
            }
        }
    }
}
