//! The inference server's client (`cantrip_ai::remote`), against a server written here that speaks the protocol and
//! evaluates with tract on the tiny model (`tests/data/tiny`), each row at its own bucket as `ai/cantrip_ai/serve.py`
//! does:
//!
//! - a batch reaches the server as the same tensors, card rows mapped as the local evaluators map them, wide card rows,
//!   numbers not exact in f16 and a cut-short frame included;
//! - the remote evaluator makes of the references what PyTorch made (export's tolerances), and bitwise what tract makes
//!   of each position alone;
//! - a server that serves another model, or a client of another encoder, is refused at the hello;
//! - threads sharing one evaluator each get a connection.
//!
//! The real server's own tests are `ai/tests/test_serve.py`; `model_parity` in `tests/eval.rs` runs a real model's
//! references through a running server when `CANTRIP_AI_SERVER` names it.

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cantrip_ai::eval::reference::{Reference, parity};
use cantrip_ai::eval::{Evaluator, FeatureBatch, TractEvaluator};
use cantrip_ai::remote::{
    self, RemoteEvaluator, model_identity, read_frame, read_hello, read_request, write_frame,
};
use cantrip_ai::{CardIndex, EvalError};
use cantrip_obs::encode::{BUCKETS, CATS, Encoded, NUMS, PTRS, SCHEMA_VERSION, encoder_hash};

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny")
}

fn own(dir: &Path) -> CardIndex {
    CardIndex::read(&dir.join("cards.json")).unwrap()
}

/// Row `i` of a batch alone, at `tokens` tokens (its real tokens fit), every real token an option so the whole policy
/// comes back.
fn row_at(b: &FeatureBatch, i: usize, tokens: usize, real: usize) -> FeatureBatch {
    let t = b.tokens;
    let mut r = FeatureBatch {
        rows: 1,
        tokens,
        kind: vec![0; tokens],
        card: vec![0; tokens],
        cats: vec![0; tokens * CATS],
        ptrs: vec![255; tokens * PTRS],
        nums: vec![0.0; tokens * NUMS],
        legal: vec![false; tokens],
        options: vec![(0, real)],
    };
    for k in 0..real {
        let (from, to) = (i * t + k, k);
        r.kind[to] = b.kind[from];
        r.card[to] = b.card[from];
        r.legal[to] = b.legal[from];
        r.cats[to * CATS..(to + 1) * CATS].copy_from_slice(&b.cats[from * CATS..(from + 1) * CATS]);
        r.ptrs[to * PTRS..(to + 1) * PTRS].copy_from_slice(&b.ptrs[from * PTRS..(from + 1) * PTRS]);
        r.nums[to * NUMS..(to + 1) * NUMS].copy_from_slice(&b.nums[from * NUMS..(from + 1) * NUMS]);
    }
    r
}

/// The real tokens of each row of a request read back: up to its last token that is not padding.
fn lengths(b: &FeatureBatch) -> Vec<usize> {
    let t = b.tokens;
    (0..b.rows)
        .map(|i| {
            (0..t)
                .rev()
                .find(|&k| {
                    let at = i * t + k;
                    b.kind[at] != 0
                        || b.card[at] != 0
                        || b.legal[at]
                        || b.cats[at * CATS..(at + 1) * CATS].iter().any(|&c| c != 0)
                        || b.ptrs[at * PTRS..(at + 1) * PTRS].iter().any(|&p| p != 255)
                        || b.nums[at * NUMS..(at + 1) * NUMS]
                            .iter()
                            .any(|x| x.to_bits() != 0)
                })
                .map_or(0, |k| k + 1)
        })
        .collect()
}

/// A server of the protocol over tract: each row at the smallest bucket that holds it.
fn serve(model: PathBuf, listener: TcpListener) {
    let tract = Arc::new(TractEvaluator::load(&model, &own(&model)).unwrap());
    let identity = model_identity(&model).unwrap();
    for stream in listener.incoming() {
        let Ok(mut s) = stream else { return };
        let tract = Arc::clone(&tract);
        std::thread::spawn(move || {
            let Ok(hello) = read_frame(&mut s) else {
                return;
            };
            let (protocol, schema, hash, id, _) = read_hello(&hello).unwrap();
            if protocol != remote::PROTOCOL
                || schema != SCHEMA_VERSION
                || hash != encoder_hash()
                || id != identity
            {
                let _ = write_frame(
                    &mut s,
                    &remote::status(false, "not the model this server serves"),
                );
                return;
            }
            write_frame(&mut s, &remote::status(true, "tract (the test's server)")).unwrap();
            while let Ok(frame) = read_frame(&mut s) {
                let b = match read_request(&frame) {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = write_frame(&mut s, &remote::status(false, &e.0));
                        return;
                    }
                };
                let mut value = Vec::new();
                let mut aux = Vec::new();
                let mut policy = vec![0.0f32; b.rows * b.tokens];
                for (i, &real) in lengths(&b).iter().enumerate() {
                    let tokens = *BUCKETS.iter().find(|&&t| t >= real).unwrap();
                    let out = tract.evaluate(&row_at(&b, i, tokens, real)).unwrap();
                    value.push(out.value[0]);
                    aux.push(out.aux[0]);
                    policy[i * b.tokens..i * b.tokens + real].copy_from_slice(&out.policy[0]);
                }
                let outputs = cantrip_ai::Outputs {
                    value,
                    policy: Vec::new(),
                    aux,
                };
                write_frame(&mut s, &remote::reply(&b, &outputs, &policy)).unwrap();
            }
        });
    }
}

fn server(model: PathBuf) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    std::thread::spawn(move || serve(model, listener));
    address
}

fn references() -> Reference {
    Reference::load(&tiny().join("reference")).unwrap()
}

fn same(a: &FeatureBatch, b: &FeatureBatch, order: &[usize]) {
    // `b` is a request read back: MAX_TOKENS wide, no options, its rows in `order`.
    let (t, u) = (a.tokens, b.tokens);
    assert_eq!(a.rows, b.rows);
    let mut sorted = order.to_vec();
    sorted.sort();
    assert_eq!(sorted, (0..a.rows).collect::<Vec<_>>());
    for (i, &row) in order.iter().enumerate() {
        for k in 0..u {
            let at = i * u + k;
            if k >= t {
                assert_eq!(b.kind[at], 0);
                continue;
            }
            let from = row * t + k;
            assert_eq!(a.kind[from], b.kind[at]);
            assert_eq!(a.card[from], b.card[at]);
            assert_eq!(a.legal[from], b.legal[at]);
            assert_eq!(
                a.cats[from * CATS..(from + 1) * CATS],
                b.cats[at * CATS..(at + 1) * CATS]
            );
            assert_eq!(
                a.ptrs[from * PTRS..(from + 1) * PTRS],
                b.ptrs[at * PTRS..(at + 1) * PTRS]
            );
            let (x, y) = (
                &a.nums[from * NUMS..(from + 1) * NUMS],
                &b.nums[at * NUMS..(at + 1) * NUMS],
            );
            assert!(
                x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits()),
                "row {i} token {k}"
            );
        }
    }
}

#[test]
fn a_batch_reaches_the_server_as_the_same_tensors() {
    let r = references();
    let es: Vec<&Encoded> = r.positions.iter().collect();
    for chunk in es.chunks(7) {
        let b = FeatureBatch::new(chunk);
        let (bytes, order) = remote::request(&b, &[]).unwrap();
        same(&b, &read_request(&bytes).unwrap(), &order);
        // Every frame cut short is refused, never read as another batch.
        for cut in [1, bytes.len() / 2, bytes.len() - 1] {
            assert!(read_request(&bytes[..cut]).is_err());
        }
    }

    // Card rows mapped as a local evaluator maps them; past u16 they go four bytes wide.
    let b = FeatureBatch::new(&es[..5]);
    let biggest = *b.card.iter().max().unwrap() as usize;
    let remap: Vec<u32> = (0..=biggest as u32)
        .map(|c| if c == 0 { 0 } else { c + 70_000 })
        .collect();
    let mut mapped = b.clone();
    for c in &mut mapped.card {
        *c = remap[*c as usize] as i64;
    }
    let (bytes, order) = remote::request(&b, &remap).unwrap();
    assert_eq!(bytes[5], 4, "the card width");
    same(&mapped, &read_request(&bytes).unwrap(), &order);

    // A number f16 cannot hold exactly is sent as f32, so nothing is rounded on the way.
    let mut odd = b.clone();
    odd.nums[3] = 0.1;
    let (bytes, order) = remote::request(&odd, &[]).unwrap();
    assert_eq!(bytes[6], remote::NUMS_DENSE);
    same(&odd, &read_request(&bytes).unwrap(), &order);

    // A value no encoder makes is an error, never a different value.
    let mut bad = b.clone();
    bad.kind[0] = 300;
    assert!(remote::request(&bad, &[]).is_err());
}

#[test]
fn the_remote_evaluator_makes_the_references() {
    let address = server(tiny());
    let evaluator = RemoteEvaluator::connect(&address, &tiny(), &own(&tiny())).unwrap();
    assert_eq!(evaluator.server(), "tract (the test's server)");
    let r = references();
    for batch in [1, 5, 16] {
        let p = parity(&evaluator, &r, batch).unwrap();
        assert!(
            p.value < 1e-3 && p.aux < 1e-3 && p.policy < 1e-3,
            "batch {batch}: {p:?}"
        );
        assert!(p.same_argmax >= 0.995, "batch {batch}: {p:?}");
        assert_eq!(p.illegal_argmax, 0);
    }
    // Each position, whatever it is batched with, is what tract makes of it alone: the wire loses nothing.
    let tract = TractEvaluator::load(&tiny(), &own(&tiny())).unwrap();
    let es: Vec<&Encoded> = r.positions.iter().collect();
    let all = evaluator.evaluate(&FeatureBatch::new(&es)).unwrap();
    for (i, e) in es.iter().enumerate() {
        let alone = tract.evaluate(&FeatureBatch::new(&[e])).unwrap();
        assert_eq!(
            alone.value[0].to_bits(),
            all.value[i].to_bits(),
            "position {i}"
        );
        assert_eq!(alone.aux[0].map(f32::to_bits), all.aux[i].map(f32::to_bits));
        assert_eq!(
            alone.policy[0]
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            all.policy[i]
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn threads_share_an_evaluator() {
    let address = server(tiny());
    let evaluator = Arc::new(RemoteEvaluator::connect(&address, &tiny(), &own(&tiny())).unwrap());
    let r = Arc::new(references());
    let want = Arc::new(
        evaluator
            .evaluate(&FeatureBatch::new(&r.positions.iter().collect::<Vec<_>>()))
            .unwrap()
            .value,
    );
    let threads: Vec<_> = (0..6)
        .map(|k| {
            let (evaluator, r, want) = (Arc::clone(&evaluator), Arc::clone(&r), Arc::clone(&want));
            std::thread::spawn(move || {
                for i in (k..r.positions.len()).step_by(6) {
                    let out = evaluator
                        .evaluate(&FeatureBatch::new(&[&r.positions[i]]))
                        .unwrap();
                    assert_eq!(out.value[0].to_bits(), want[i].to_bits());
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
}

#[test]
fn another_model_is_refused() {
    // A server of another model: the same graphs under another manifest.
    let dir = std::env::temp_dir().join(format!("cantrip-remote-other-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in [
        "manifest.json",
        "cards.json",
        "model-t32.onnx",
        "model-t64.onnx",
        "model-t128.onnx",
    ] {
        std::fs::copy(tiny().join(f), dir.join(f)).unwrap();
    }
    let text = std::fs::read_to_string(dir.join("manifest.json")).unwrap();
    std::fs::write(dir.join("manifest.json"), format!("{text}\n")).unwrap();
    let address = server(dir.clone());
    let e = RemoteEvaluator::connect(&address, &tiny(), &own(&tiny()))
        .err()
        .expect("refused");
    assert!(e.0.contains("refused"), "{e}");
    assert!(RemoteEvaluator::connect(&address, &dir, &own(&dir)).is_ok());

    // A client of another encoder is refused too: the hello carries this build's schema and encoder hash.
    let mut s = TcpStream::connect(&address).unwrap();
    let mut hello = remote::hello(model_identity(&dir).unwrap(), "x");
    hello[16] ^= 1;
    write_frame(&mut s, &hello).unwrap();
    let welcome = read_frame(&mut s).unwrap();
    assert_eq!(welcome[0], 1);
    std::fs::remove_dir_all(&dir).unwrap();

    // No server at all is an error at connect, not later.
    let gone = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = gone.local_addr().unwrap().to_string();
    drop(gone);
    let e: EvalError = RemoteEvaluator::connect(&address, &tiny(), &own(&tiny()))
        .err()
        .unwrap();
    assert!(e.0.contains("inference server"), "{e}");
}
