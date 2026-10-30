//! `cantrip-infer`, the native inference server, through `cantrip_ai::RemoteEvaluator` (`--ignored`, with the `gpu`
//! feature and a GPU: onnxruntime-gpu named by `ORT_DYLIB_PATH`, CUDA's libraries by `CANTRIP_CUDA_LIBS`):
//!
//! - it makes the tiny model's references as PyTorch made them (export's tolerance) and as the engine in this process
//!   makes them, bitwise;
//! - it refuses a client of a model it does not serve;
//! - threads sharing one evaluator each get a connection, and each gets its own rows' outputs.

#![cfg(feature = "gpu")]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cantrip_ai::eval::reference::{Reference, parity};
use cantrip_ai::eval::{Evaluator, FeatureBatch};
use cantrip_ai::gpu::{Engine, GpuEvaluator, Options};
use cantrip_ai::{CardIndex, RemoteEvaluator};
use cantrip_obs::encode::Encoded;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../cantrip-ai/tests/data/tiny")
}

fn own(dir: &Path) -> CardIndex {
    CardIndex::read(&dir.join("cards.json")).unwrap()
}

struct Server {
    child: Child,
    address: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn server(model: &Path) -> Server {
    let ready = std::env::temp_dir().join(format!("cantrip-infer-test-{}.ready", std::process::id()));
    let _ = std::fs::remove_file(&ready);
    let child = Command::new(env!("CARGO_BIN_EXE_cantrip-infer"))
        .args(["serve", "--model"])
        .arg(model)
        .args(["--port", "0", "--batch", "32:16,64:8,128:4", "--priority", "normal", "--log-every", "0", "--ready-file"])
        .arg(&ready)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "the server did not start");
        std::thread::sleep(Duration::from_millis(100));
    }
    let address = std::fs::read_to_string(&ready).unwrap();
    let _ = std::fs::remove_file(&ready);
    Server { child, address }
}

#[test]
#[ignore]
fn the_native_server_makes_the_references() {
    let s = server(&tiny());
    let remote = RemoteEvaluator::connect(&s.address, &tiny(), &own(&tiny())).unwrap();
    assert!(remote.server().contains("cantrip-infer"), "{}", remote.server());
    let r = Reference::load(&tiny().join("reference")).unwrap();
    for batch in [1, 64] {
        let p = parity(&remote, &r, batch).unwrap();
        println!("batch {batch}: {p:?}");
        assert!(p.value < 1e-3 && p.aux < 1e-3 && p.policy < 1e-3, "{p:?}");
        assert_eq!(p.same_argmax, 1.0);
        assert_eq!(p.illegal_argmax, 0);
    }
    // Bitwise the engine in this process (the same batch sizes).
    let engine = Arc::new(
        Engine::start(
            &[&tiny()],
            Options {
                batch: [16, 8, 4],
                ..Options::default()
            },
        )
        .unwrap(),
    );
    let local = GpuEvaluator::new(engine, 0, &own(&tiny())).unwrap();
    for chunk in r.positions.chunks(5) {
        let es: Vec<&Encoded> = chunk.iter().collect();
        let b = FeatureBatch::new(&es);
        assert_eq!(remote.evaluate(&b).unwrap(), local.evaluate(&b).unwrap());
    }
}

#[test]
#[ignore]
fn the_native_server_refuses_another_model() {
    let s = server(&tiny());
    let other = std::env::temp_dir().join(format!("cantrip-infer-other-{}", std::process::id()));
    std::fs::create_dir_all(&other).unwrap();
    for f in ["cards.json", "model-t32.onnx", "model-t64.onnx", "model-t128.onnx"] {
        std::fs::copy(tiny().join(f), other.join(f)).unwrap();
    }
    let mut manifest = std::fs::read_to_string(tiny().join("manifest.json")).unwrap();
    manifest.push('\n');
    std::fs::write(other.join("manifest.json"), manifest).unwrap();
    let e = RemoteEvaluator::connect(&s.address, &other, &own(&other)).err().expect("refused");
    assert!(e.0.contains("no model"), "{e}");
    let _ = std::fs::remove_dir_all(&other);
}

#[test]
#[ignore]
fn threads_share_the_native_server() {
    let s = server(&tiny());
    let remote = Arc::new(RemoteEvaluator::connect(&s.address, &tiny(), &own(&tiny())).unwrap());
    let r = Arc::new(Reference::load(&tiny().join("reference")).unwrap());
    let want: Vec<_> = r.positions.iter().map(|p| remote.evaluate(&FeatureBatch::new(&[p])).unwrap()).collect();
    std::thread::scope(|sc| {
        for c in 0..12 {
            let (remote, r, want) = (remote.clone(), r.clone(), &want);
            sc.spawn(move || {
                for round in 0..10 {
                    let at = (c * 11 + round * 3) % r.positions.len();
                    let idx: Vec<usize> = (0..1 + (c + round) % 5).map(|k| (at + k) % r.positions.len()).collect();
                    let es: Vec<&Encoded> = idx.iter().map(|&i| &r.positions[i]).collect();
                    let out = remote.evaluate(&FeatureBatch::new(&es)).unwrap();
                    for (j, &i) in idx.iter().enumerate() {
                        assert_eq!(out.value[j].to_bits(), want[i].value[0].to_bits());
                        assert_eq!(out.policy[j], want[i].policy[0]);
                    }
                }
            });
        }
    });
}
