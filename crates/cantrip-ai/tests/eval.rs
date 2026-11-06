//! Evaluating with a trained network (`cantrip_ai::eval`) and answering with it alone (`cantrip_ai::net`), on the
//! tiny model checked in under `tests/data/tiny` (`ai/cantrip_ai/export.py` over a 10 k-parameter network, its
//! references 48 scenario positions):
//!
//! - tract makes of the references what PyTorch made (within 1e-3), and the best option it rates is a legal one;
//! - the Rust decoder (`from_compact`) reads each reference position as the Python one did;
//! - a model of another encoding schema or encoder hash is refused, and one of another card index is mapped by name;
//! - the network's answers to a sample of the corpus's requests are the engine validator's to accept.
//!
//! `cargo test -p cantrip-ai --test eval -- --ignored` also checks a real model's references (`CANTRIP_MODEL`, default
//! `../ai-data/models/gen0`), through tract, with `--features ort` onnxruntime, and with `CANTRIP_AI_SERVER=HOST:PORT`
//! the inference server serving it (`cantrip_ai::RemoteEvaluator`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cantrip_ai::eval::reference::{Reference, digest, parity};
use cantrip_ai::eval::{Evaluator, Manifest, Model, TractEvaluator, check};
use cantrip_ai::net::{Choice, answer};
use cantrip_ai::record::table;
use cantrip_ai::{CardIndex, EvalError, Knowledge};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::encode::{SCHEMA_VERSION, UNKNOWN_CARD};
use cantrip_script::GameScript;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny")
}

/// A model's own card index (its `cards.json`): what its reference positions' rows are of, so nothing is remapped.
fn own(dir: &Path) -> CardIndex {
    CardIndex::read(&dir.join("cards.json")).unwrap()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn tract_makes_the_references() {
    let evaluator = TractEvaluator::load(&tiny(), &own(&tiny())).unwrap();
    assert!(
        evaluator.model().remap.is_empty(),
        "the references are in the tiny model's own rows"
    );
    let r = Reference::load(&tiny().join("reference")).unwrap();
    assert_eq!(r.positions.len(), 48);
    for b in cantrip_obs::encode::BUCKETS {
        assert!(
            r.positions.iter().any(|e| e.bucket == b),
            "no reference position of {b} tokens"
        );
    }
    for (i, e) in r.positions.iter().enumerate() {
        assert_eq!(
            digest(e),
            r.decoded[i],
            "position {i}: from_compact and ai/shards.py decode it differently"
        );
    }
    for batch in [1, 5, 16] {
        let p = parity(&evaluator, &r, batch).unwrap();
        println!("batch {batch}: {p:?}");
        assert!(
            p.value < 1e-3 && p.aux < 1e-3 && p.policy < 1e-3,
            "batch {batch}: {p:?}"
        );
        assert!(p.same_argmax >= 0.995, "batch {batch}: {p:?}");
        assert_eq!(p.illegal_argmax, 0);
    }
}

fn manifest() -> Manifest {
    serde_json::from_str(&std::fs::read_to_string(tiny().join("manifest.json")).unwrap()).unwrap()
}

#[test]
fn another_encoding_is_refused() {
    let index = own(&tiny());
    let names = || -> Result<Vec<String>, EvalError> { Ok(index.names.clone()) };
    assert_eq!(check(&manifest(), &index, names), Ok(Vec::new()));

    for schema in [1, SCHEMA_VERSION + 1] {
        let mut m = manifest();
        m.schema = schema;
        let e = check(&m, &index, names).unwrap_err();
        assert!(e.0.contains("schema"), "{e}");
    }

    let mut m = manifest();
    m.encoder_hash = "0123456789abcdef".into();
    let e = check(&m, &index, names).unwrap_err();
    assert!(e.0.contains("encoder hash"), "{e}");

    // On disk too: the directory is refused as a whole.
    let dir = std::env::temp_dir().join(format!("cantrip-eval-refused-{}", std::process::id()));
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
    let ours = manifest().encoder_hash;
    assert!(text.contains(&ours));
    std::fs::write(
        dir.join("manifest.json"),
        text.replace(&ours, "0123456789abcdef"),
    )
    .unwrap();
    let e = TractEvaluator::load(&dir, table()).err().expect("refused");
    assert!(e.0.contains("encoder hash"), "{e}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn another_card_set_is_mapped_by_name() {
    let mut m = manifest();
    m.text_hash = "a newer card set".into();
    // The model knew the runtime's cards in another order, and not the first of them.
    let index = own(&tiny());
    let runtime = &index.names;
    let mut known: Vec<String> = runtime[1..].to_vec();
    known.reverse();
    let remap = check(&m, &index, || Ok(known.clone())).unwrap();
    assert_eq!(remap.len(), runtime.len() + 1);
    assert_eq!(remap[0], UNKNOWN_CARD);
    assert_eq!(
        remap[1], UNKNOWN_CARD,
        "a card the model never saw is the unknown card"
    );
    for (i, name) in runtime.iter().enumerate().skip(1) {
        let row = remap[i + 1] as usize;
        assert_eq!(&known[row - 1], name);
    }

    // And the evaluator maps a batch's rows through it: a model whose table is the runtime's reversed rates a
    // position as the plain model rates the position with its rows reversed.
    let model = Model::load(&tiny(), &index).unwrap();
    let plain = TractEvaluator::new(model.clone());
    let reversed: Vec<u32> = std::iter::once(0)
        .chain((1..=runtime.len() as u32).rev())
        .collect();
    let mapped = TractEvaluator::new(Model {
        remap: reversed.clone(),
        ..model
    });
    let r = Reference::load(&tiny().join("reference")).unwrap();
    let e = r
        .positions
        .iter()
        .find(|e| e.card.iter().filter(|&&c| c != 0).count() > 3)
        .unwrap();
    let mut flipped = e.clone();
    for c in &mut flipped.card {
        *c = reversed[*c as usize];
    }
    let a = mapped
        .evaluate(&cantrip_ai::FeatureBatch::new(&[e]))
        .unwrap();
    let b = plain
        .evaluate(&cantrip_ai::FeatureBatch::new(&[&flipped]))
        .unwrap();
    assert!((a.value[0] - b.value[0]).abs() < 1e-5);
}

#[derive(Default, Debug)]
struct Tally {
    requests: usize,
    accepted: usize,
    first_walk: usize,
    fallback: usize,
    refused: Vec<String>,
}

fn play(path: &Path, evaluator: &dyn Evaluator, tally: &Mutex<Tally>) {
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let Ok(script) = GameScript::load(path) else {
        return;
    };
    let Ok(result) = cantrip_script::run(&script) else {
        return;
    };
    let mut game = result.game;
    let nodes = game.tree.path(game.at);
    let answers: Vec<_> = nodes
        .iter()
        .filter_map(|&n| game.tree.answer(n).map(|(_, p, _)| (n, *p)))
        .collect();
    for (k, (node, player)) in answers.iter().enumerate() {
        let Some(parent) = game.tree.parent(*node) else {
            continue;
        };
        game.goto(parent);
        let Some(request) = game.pending().cloned() else {
            continue;
        };
        if request.player != *player {
            continue;
        }
        let knowledge = Knowledge::of(&game, *player);
        let mode = [InfoMode::Hidden, InfoMode::Decklist, InfoMode::Full][k % 3];
        let choice = if k % 2 == 0 {
            Choice::Best
        } else {
            Choice::Sample {
                temperature: 1.0,
                seed: k as u64 + 1,
            }
        };
        let made = answer(
            &game,
            &request,
            &knowledge,
            mode,
            table(),
            evaluator,
            choice,
        );
        let mut t = tally.lock().unwrap();
        t.requests += 1;
        match made {
            Ok(d) => {
                // Checked again here, as the host would.
                if let Err(r) = check_answer(&game, &request, &d.answer) {
                    t.refused.push(format!(
                        "{name} answer {k}: {:?} refused: {}",
                        d.answer, r.0
                    ));
                    continue;
                }
                assert!((-1.0..=1.0).contains(&d.value));
                t.accepted += 1;
                t.first_walk += d.rejected.is_empty() as usize;
                t.fallback += d.fallback as usize;
            }
            Err(e) if e.0.contains("deferred") => t.requests -= 1,
            Err(e) => t
                .refused
                .push(format!("{name} answer {k} ({:?}): {e}", request.kind)),
        }
    }
}

fn scenarios(every: usize) -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(root().join("scenarios"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "cantrip"))
        .collect();
    all.sort();
    all.into_iter().step_by(every).collect()
}

fn corpus(every: usize) -> Tally {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let sample = scenarios(every);
    let tally = Mutex::new(Tally::default());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            let (sample, tally, next, evaluator) = (&sample, &tally, &next, &evaluator);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(s, move || {
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(path) = sample.get(i) else { break };
                        play(path, evaluator, tally);
                    }
                })
                .unwrap();
        }
    });
    let t = tally.into_inner().unwrap();
    println!(
        "{} scenarios, {} requests: {} answered ({} on the first walk, {} by the first-legal fallback), {} refused",
        sample.len(),
        t.requests,
        t.accepted,
        t.first_walk,
        t.fallback,
        t.refused.len()
    );
    for r in t.refused.iter().take(20) {
        println!("  {r}");
    }
    t
}

#[test]
fn network_answers_are_accepted() {
    let t = corpus(60);
    assert!(t.requests > 500, "{t:?}");
    assert!(t.refused.is_empty(), "{:#?}", t.refused);
}

#[test]
#[ignore]
fn network_answers_are_accepted_over_the_corpus() {
    let t = corpus(1);
    assert!(t.refused.is_empty(), "{} refused", t.refused.len());
}

/// A real model's references through tract (and onnxruntime, with `--features ort`; the GPU engine, with `--features
/// gpu`; and the inference server named by `CANTRIP_AI_SERVER`, Python's or the native one): PyTorch's values within
/// 1e-3 and the same best option on 99.5% of positions.
#[test]
#[ignore]
fn model_parity() {
    let dir = std::env::var("CANTRIP_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root().join("../ai-data/models/gen0"));
    let r = Reference::load(&dir.join("reference")).unwrap();
    for (i, e) in r.positions.iter().enumerate() {
        assert_eq!(digest(e), r.decoded[i], "position {i}");
    }
    #[allow(unused_mut)]
    let mut runs: Vec<(&str, Box<dyn Evaluator>)> = vec![(
        "tract",
        Box::new(TractEvaluator::load(&dir, &own(&dir)).unwrap()),
    )];
    #[cfg(feature = "ort")]
    runs.push((
        "ort",
        Box::new(cantrip_ai::eval::OrtEvaluator::load(&dir, &own(&dir), 4).unwrap()),
    ));
    // The GPU engine in this process, at fp32.
    #[cfg(feature = "gpu")]
    {
        let engine = std::sync::Arc::new(
            cantrip_ai::gpu::Engine::start(&[&dir], cantrip_ai::gpu::Options::default()).unwrap(),
        );
        runs.push((
            "gpu",
            Box::new(cantrip_ai::gpu::GpuEvaluator::new(engine, 0, &own(&dir)).unwrap()),
        ));
    }
    // The inference server serving this model (`python -m cantrip_ai.serve --model DIR`, or the native one,
    // `cantrip-infer serve --model DIR`), when one is named. `CANTRIP_AI_SERVER_REDUCED=1`: a server below fp32 (fp16,
    // tf32), held to the reduced bar on its value and dead-next-turn (1e-2) and its logits (5e-2) instead.
    let reduced = std::env::var("CANTRIP_AI_SERVER_REDUCED").is_ok_and(|v| v == "1");
    if let Ok(address) = std::env::var("CANTRIP_AI_SERVER") {
        runs.push((
            "remote",
            Box::new(cantrip_ai::RemoteEvaluator::connect(&address, &dir, &own(&dir)).unwrap()),
        ));
    }
    for (name, evaluator) in &runs {
        for batch in [1, 64] {
            let p = parity(evaluator.as_ref(), &r, batch).unwrap();
            println!(
                "{name} batch {batch}: {p:?} ({:.0} positions a second)",
                p.positions as f64 / p.seconds
            );
            // Reduced: value 1e-2, aux (the life swing, in life points, which no search reads) half a point, logits 5e-2.
            let (value, aux, logits) = if reduced && *name == "remote" { (1e-2, 0.5, 5e-2) } else { (1e-3, 1e-3, 1e-3) };
            assert!(
                p.value < value && p.aux < aux && p.policy < logits,
                "{name}: {p:?}"
            );
            assert!(p.same_argmax >= 0.995, "{name}: {p:?}");
            assert_eq!(p.illegal_argmax, 0);
        }
    }
}

#[test]
fn legacy_snow_input_masks_only_new_cells_and_preserves_all_reference_outputs() {
    use cantrip_ai::eval::{FeatureBatch, InputCompatibility, input_compatibility};
    use cantrip_obs::encode::{NUMS, kind};
    let evaluator = TractEvaluator::load(&tiny(), &own(&tiny())).unwrap();
    assert_eq!(evaluator.model().input_compatibility(), InputCompatibility::LegacySnowInputV2);
    assert_eq!(evaluator.model().manifest.schema, 2);
    let refs = Reference::load(&tiny().join("reference")).unwrap();
    #[cfg(feature = "ort")]
    let ort = cantrip_ai::eval::OrtEvaluator::load(&tiny(), &own(&tiny()), 1).unwrap();
    #[allow(unused_mut)]
    let mut evaluators: Vec<&dyn Evaluator> = vec![&evaluator];
    #[cfg(feature = "ort")]
    evaluators.push(&ort);
    for position in &refs.positions {
        let original = FeatureBatch::new(&[position]);
        let mut snow = original.clone();
        for (t, &k) in snow.kind.iter().enumerate() {
            for f in 0..NUMS {
                if InputCompatibility::LegacySnowInputV2.masks(k as u8, f) {
                    snow.nums[t * NUMS + f] = 19.0;
                }
            }
        }
        // Same graph, exact old logits, on every one of the 48 saved positions.
        for evaluator in &evaluators {
            assert_eq!(evaluator.evaluate(&original).unwrap(), evaluator.evaluate(&snow).unwrap());
        }
        InputCompatibility::LegacySnowInputV2.adapt(&mut snow);
        assert_eq!(original.nums, snow.nums);
    }
    // An old graph has no embeddings for the new ask/token. It receives unknown categories and zeroed new features,
    // while the existing parts of the same encoded position still produce exactly the old graph's output.
    let position = refs.positions.iter().find(|p| p.kind.iter().any(|&k| k == kind::OPTION)).unwrap();
    let mut neutral = FeatureBatch::new(&[position]);
    let game = neutral.kind.iter().position(|&k| k == kind::GAME as i64).unwrap();
    let option = neutral.kind.iter().position(|&k| k == kind::OPTION as i64).unwrap();
    neutral.cats[game * cantrip_obs::encode::CATS + cantrip_obs::encode::cat::ASK] = 0;
    neutral.cats[option * cantrip_obs::encode::CATS + cantrip_obs::encode::cat::TOK] = 0;
    let mut cost = neutral.clone();
    cost.cats[game * cantrip_obs::encode::CATS + cantrip_obs::encode::cat::ASK] = 27;
    cost.cats[option * cantrip_obs::encode::CATS + cantrip_obs::encode::cat::TOK] = 29;
    cost.nums[game * NUMS + cantrip_obs::encode::game::MANA_ABILITIES] = 1.0;
    cost.nums[option * NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS] = 1.0;
    cost.nums[option * NUMS + cantrip_obs::encode::option::BLOCK_CAPACITY] = 2.0 / 16.0;
    cost.ptrs[option * cantrip_obs::encode::PTRS + 2] = 0;
    cost.ptrs[option * cantrip_obs::encode::PTRS + 3] = 1;
    assert_eq!(evaluator.evaluate(&neutral).unwrap(), evaluator.evaluate(&cost).unwrap());
    let mut batch = FeatureBatch { rows: 1, tokens: 4, kind: vec![kind::GAME as i64, kind::PLAYER as i64, kind::OPTION as i64, kind::CARD as i64], nums: vec![7.0; 4 * NUMS], ..Default::default() };
    let native = batch.clone();
    InputCompatibility::Native.adapt(&mut batch);
    assert_eq!(native.nums, batch.nums);
    InputCompatibility::LegacySnowInputV2.adapt(&mut batch);
    assert_eq!(&batch.nums[3 * NUMS..], &[7.0; NUMS]);
    assert_eq!(batch.nums[31], 0.0);
    assert_eq!(&batch.nums[NUMS + 25..NUMS + 31], &[0.0; 6]);
    assert_eq!(&batch.nums[2 * NUMS + 29..2 * NUMS + 31], &[0.0; 2]);
    let mut current = manifest();
    current.schema = SCHEMA_VERSION;
    current.encoder_hash = format!("{:016x}", cantrip_obs::encode::encoder_hash());
    assert_eq!(input_compatibility(&current).unwrap(), InputCompatibility::Native);
    for hash in cantrip_ai::eval::LEGACY_SNOW_V2_HASHES {
        current.schema = 2;
        current.encoder_hash = hash.into();
        assert_eq!(input_compatibility(&current).unwrap(), InputCompatibility::LegacySnowInputV2);
    }
    current.schema = 3;
    current.encoder_hash = "018b410d06b12950".into();
    assert_eq!(input_compatibility(&current).unwrap(), InputCompatibility::LegacyCostInputV3);
    let mut new_cost = FeatureBatch { rows: 1, tokens: 2, kind: vec![kind::GAME as i64, kind::OPTION as i64],
        cats: vec![0; 2 * cantrip_obs::encode::CATS], nums: vec![0.0; 2 * NUMS], ..Default::default() };
    new_cost.cats[cantrip_obs::encode::cat::ASK] = 27;
    new_cost.cats[cantrip_obs::encode::CATS + cantrip_obs::encode::cat::TOK] = 29;
    new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS] = 1.0;
    new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 10] = 1.0;
    new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 11] = 1.0;
    new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 12] = 1.0;
    new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 13] = 1.0;
    new_cost.nums[NUMS + cantrip_obs::encode::option::BLOCK_CAPACITY] = 2.0 / 16.0;
    new_cost.nums[cantrip_obs::encode::game::MANA_ABILITIES] = 1.0;
    new_cost.ptrs = vec![0; 2 * cantrip_obs::encode::PTRS];
    let mut native_cost = new_cost.clone();
    InputCompatibility::Native.adapt(&mut native_cost);
    assert_eq!(native_cost.ptrs, new_cost.ptrs);
    InputCompatibility::LegacyCostInputV3.adapt(&mut new_cost);
    assert_eq!(&new_cost.ptrs[..cantrip_obs::encode::PTRS], &[0; cantrip_obs::encode::PTRS]);
    assert_eq!(&new_cost.ptrs[cantrip_obs::encode::PTRS..],
        &[0, 0, i64::from(cantrip_obs::encode::NO_POINTER), i64::from(cantrip_obs::encode::NO_POINTER)]);
    assert_eq!(new_cost.cats[cantrip_obs::encode::cat::ASK], 0);
    assert_eq!(new_cost.cats[cantrip_obs::encode::CATS + cantrip_obs::encode::cat::TOK], 0);
    assert_eq!(new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS], 0.0);
    assert_eq!(new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 10], 0.0);
    assert_eq!(new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 11], 0.0);
    assert_eq!(new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 12], 0.0);
    assert_eq!(new_cost.nums[NUMS + cantrip_obs::encode::option::COST_OPERATION_KINDS + 13], 0.0);
    assert_eq!(new_cost.nums[NUMS + cantrip_obs::encode::option::BLOCK_CAPACITY], 0.0);
    assert_eq!(new_cost.nums[cantrip_obs::encode::game::MANA_ABILITIES], 0.0);
    current.encoder_hash = "0123456789abcdef".into();
    assert!(input_compatibility(&current).is_err());
}

#[test]
fn cost_choose_slot_is_native_only_for_legacy_input_adapters() {
    use cantrip_ai::eval::{FeatureBatch, InputCompatibility};
    use cantrip_obs::encode::{kind, option, NUMS};
    let slot = option::COST_OPERATION_KINDS + 14;
    assert_eq!(slot, 45);
    let input = FeatureBatch { rows: 1, tokens: 1, kind: vec![kind::OPTION as i64], nums: {
        let mut nums = vec![0.0; NUMS]; nums[slot] = 1.0; nums
    }, ..Default::default() };
    for (adapter, expected) in [(InputCompatibility::Native, 1.0), (InputCompatibility::LegacySnowInputV2, 0.0), (InputCompatibility::LegacyCostInputV3, 0.0)] {
        let mut batch = input.clone();
        adapter.adapt(&mut batch);
        assert_eq!(batch.nums[slot], expected, "{adapter:?}");
    }
}
