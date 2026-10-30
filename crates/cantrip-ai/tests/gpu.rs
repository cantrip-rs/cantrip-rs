//! The GPU engine (`cantrip_ai::gpu`, the `gpu` feature): `cargo test -p cantrip-ai --features gpu --test gpu`.
//!
//! Without a GPU:
//! - the rows it takes are the same whether they come from positions, a padded batch or the wire;
//! - the graph it loads takes narrow integer inputs and makes bitwise what the exported graph makes (tract runs both);
//! - its half-precision graph (matrix products in fp16) stays within the reduced bar of the references.
//!
//! With one (`--ignored`; onnxruntime-gpu named by `ORT_DYLIB_PATH`, CUDA's libraries by `CANTRIP_CUDA_LIBS`):
//! - the engine makes the tiny model's references, within export's tolerance at fp32 and the reduced bar at fp16;
//! - a row's outputs are bitwise the same whatever it is batched with and wherever in the batch;
//! - `evaluate_positions` is bitwise `evaluate`.

#![cfg(feature = "gpu")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cantrip_ai::CardIndex;
use cantrip_ai::eval::reference::Reference;
use cantrip_ai::eval::{Evaluator, FeatureBatch};
use cantrip_ai::gpu::{self, Engine, GpuEvaluator, Options, Precision, Rows};
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
    let plan = model
        .with_input_fact(4, f32::fact([rows, t, NUMS]).into())
        .unwrap()
        .with_input_fact(5, bool::fact([rows, t]).into())
        .unwrap()
        .into_optimized()
        .unwrap()
        .into_runnable()
        .unwrap();
    plan.run(inputs).unwrap()
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
    gpu::half_matmuls(&mut model).unwrap();
    let g = model.graph.as_ref().unwrap();
    let ops = |g: &pb::GraphProto, op: &str| g.node.iter().filter(|n| n.op_type == op).count();
    let old = before.graph.as_ref().unwrap();
    let products = ops(old, "MatMul") + ops(old, "Gemm");
    // Every product's output is cast back to float; every other node is as it was.
    assert_eq!(ops(g, "MatMul") + ops(g, "Gemm"), products);
    assert!(ops(g, "Cast") >= ops(old, "Cast") + products);
    for n in &g.node {
        if n.op_type == "MatMul" || n.op_type == "Gemm" {
            assert!(n.input.iter().all(|i| i.ends_with("__fp16")), "{n:?}");
            assert!(n.output[0].ends_with("__fp16out"));
        }
    }
    let others = |g: &pb::GraphProto| {
        g.node
            .iter()
            .filter(|n| !matches!(n.op_type.as_str(), "MatMul" | "Gemm" | "Cast"))
            .map(|n| format!("{}:{:?}->{:?}", n.op_type, n.input, n.output))
            .collect::<Vec<_>>()
    };
    assert_eq!(others(g), others(old));
}

fn engine(precision: Precision) -> Arc<Engine> {
    let options = Options {
        precision,
        batch: [16, 8, 4],
        ..Options::default()
    };
    Arc::new(Engine::start(&[&tiny()], options).unwrap())
}

#[test]
#[ignore]
fn the_engine_makes_the_references() {
    for precision in [Precision::Fp32, Precision::Tf32, Precision::Fp16] {
        let e = engine(precision);
        let c = gpu::check(&e, 0).unwrap();
        println!("{}: {c:?}", precision.name());
        if precision == Precision::Fp32 {
            assert!(c.pass, "{c:?}");
            assert!(c.value < 1e-3 && c.logits < 1e-3 && c.swing_reduced < 1e-3 && c.dead < 1e-3);
            assert_eq!(c.same_argmax, 1.0);
        } else {
            // The tiny model is untrained: its 48 references hold near ties, which reduced precision may flip (one of
            // 48 at tf32 is 98%, under the 99.5% a real model is held to). The values are held to the reduced bar.
            assert!(c.value < gpu::REDUCED_VALUE && c.dead < gpu::REDUCED_VALUE, "{c:?}");
            assert!(c.probability < gpu::REDUCED_PROBABILITY && c.swing_reduced < gpu::REDUCED_SWING, "{c:?}");
            assert!(c.same_argmax >= 1.0 - 1.0 / c.positions as f64, "{c:?}");
        }
    }
}

#[test]
#[ignore]
fn a_row_is_the_same_whatever_it_is_batched_with() {
    let r = references();
    for precision in [Precision::Fp32, Precision::Fp16] {
        let e = engine(precision);
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
                    assert_eq!(out.value[j].to_bits(), a.value[0].to_bits(), "{} pass {pass} row {i}", precision.name());
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
