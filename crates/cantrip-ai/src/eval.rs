//! Evaluating encoded positions with a trained network (docs/ai-player-plan.md, "Evaluation targets"): the
//! `Evaluator` trait, its CPU backends - `TractEvaluator` (pure Rust, the default) and `OrtEvaluator` (onnxruntime,
//! behind the `ort` feature, off in the workspace's build since `ort` downloads binaries) - and a model directory as
//! `ai/cantrip_ai/export.py` writes it: one ONNX graph per token bucket, `manifest.json` and the card index the model
//! was trained with (`cards.json`).
//!
//! Loading checks the manifest against this build: native graphs use the current schema/hash; exact documented
//! schema-2 graphs use LegacySnowInputV2, and the exact previous schema-3 layout uses LegacyCostInputV3, without
//! retagging their provenance. Other encodings are refused. A
//! model trained with another card index is accepted: the runtime's rows are mapped to the model's by name, and a
//! card the model never saw is the unknown card (row 0).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cantrip_obs::encode::{
    BUCKETS, CATS, Encoded, NO_POINTER, NUMS, PTRS, SCHEMA_VERSION, UNKNOWN_CARD, encoder_hash,
};
use serde::Deserialize;
use tract_onnx::prelude::*;

use crate::CardIndex;

/// A policy logit no legal option has: the graphs write it wherever a token is not a legal option.
pub const MASKED: f32 = -1e9;

#[derive(Clone, Debug, PartialEq)]
pub struct EvalError(pub String);

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EvalError {}

fn err(e: impl fmt::Display) -> EvalError {
    EvalError(e.to_string())
}

/// Positions padded to one token count, as the graphs take them. Card rows are the runtime table's; an evaluator
/// maps them to its model's.
#[derive(Clone, Debug, Default)]
pub struct FeatureBatch {
    pub rows: usize,
    pub tokens: usize,
    /// [rows, tokens]
    pub kind: Vec<i64>,
    pub card: Vec<i64>,
    /// [rows, tokens, CATS]
    pub cats: Vec<i64>,
    /// [rows, tokens, PTRS]
    pub ptrs: Vec<i64>,
    /// [rows, tokens, NUMS]
    pub nums: Vec<f32>,
    /// [rows, tokens]
    pub legal: Vec<bool>,
    /// Each position's options: its first option's token and how many.
    pub options: Vec<(usize, usize)>,
}

impl FeatureBatch {
    /// The positions padded to the largest of their buckets.
    pub fn new(positions: &[&Encoded]) -> FeatureBatch {
        let tokens = positions
            .iter()
            .map(|e| e.bucket)
            .max()
            .unwrap_or(BUCKETS[0]);
        let mut b = FeatureBatch::empty(positions.len(), tokens);
        for (i, e) in positions.iter().enumerate() {
            b.set(i, e);
        }
        b
    }

    fn empty(rows: usize, tokens: usize) -> FeatureBatch {
        FeatureBatch {
            rows,
            tokens,
            kind: vec![0; rows * tokens],
            card: vec![UNKNOWN_CARD as i64; rows * tokens],
            cats: vec![0; rows * tokens * CATS],
            ptrs: vec![NO_POINTER as i64; rows * tokens * PTRS],
            nums: vec![0.0; rows * tokens * NUMS],
            legal: vec![false; rows * tokens],
            options: vec![(0, 0); rows],
        }
    }

    fn set(&mut self, i: usize, e: &Encoded) {
        let t = self.tokens;
        for k in 0..e.len.min(t) {
            let at = i * t + k;
            self.kind[at] = e.kind[k] as i64;
            self.card[at] = e.card[k] as i64;
            for c in 0..CATS {
                self.cats[at * CATS + c] = e.cats[k * CATS + c] as i64;
            }
            for p in 0..PTRS {
                self.ptrs[at * PTRS + p] = e.ptrs[k * PTRS + p] as i64;
            }
            self.nums[at * NUMS..(at + 1) * NUMS]
                .copy_from_slice(&e.nums[k * NUMS..(k + 1) * NUMS]);
        }
        for k in 0..e.bucket.min(t) {
            self.legal[i * t + k] = e.legal[k];
        }
        self.options[i] = (e.first_option, e.options);
    }

    /// The same positions with `rows` rows (padding rows are all padding), the card rows mapped through `remap`.
    fn padded(&self, rows: usize, remap: &[u32]) -> FeatureBatch {
        let t = self.tokens;
        let mut b = FeatureBatch::empty(rows, t);
        let n = self.rows * t;
        b.kind[..n].copy_from_slice(&self.kind);
        b.card[..n].copy_from_slice(&self.card);
        if !remap.is_empty() {
            for c in &mut b.card[..n] {
                *c = remap.get(*c as usize).copied().unwrap_or(UNKNOWN_CARD) as i64;
            }
        }
        b.cats[..n * CATS].copy_from_slice(&self.cats);
        b.ptrs[..n * PTRS].copy_from_slice(&self.ptrs);
        b.nums[..n * NUMS].copy_from_slice(&self.nums);
        b.legal[..n].copy_from_slice(&self.legal);
        b
    }
}

/// What a network makes of each position of a batch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outputs {
    /// The expected result for the seat, in [-1, 1].
    pub value: Vec<f32>,
    /// Each position's option logits, in its step's order (`Encoded::option_token`); `MASKED` where not legal.
    pub policy: Vec<Vec<f32>>,
    /// The life swing over the turn (life points) and the probability of being dead by the next turn.
    pub aux: Vec<[f32; 2]>,
}

impl Outputs {
    /// The legal option a position's policy rates highest, `None` if none is legal.
    pub fn argmax(&self, i: usize) -> Option<usize> {
        let row = &self.policy[i];
        (0..row.len())
            .filter(|&k| row[k] > MASKED / 2.0)
            .max_by(|&a, &b| row[a].total_cmp(&row[b]).then(b.cmp(&a)))
    }
}

pub trait Evaluator: Send + Sync {
    fn evaluate(&self, batch: &FeatureBatch) -> Result<Outputs, EvalError>;

    /// The positions themselves: what `evaluate` makes of `FeatureBatch::new(positions)`. An evaluator that takes
    /// each position at its own length (the GPU engine, `gpu::GpuEvaluator`) skips padding them all to one size.
    fn evaluate_positions(&self, positions: &[&Encoded]) -> Result<Outputs, EvalError> {
        self.evaluate(&FeatureBatch::new(positions))
    }

    /// Its caller will send nothing for a while (a self-play lane waiting for admission, or switched off): an evaluator
    /// that counts as a client of a batching engine or server stops counting until it next evaluates, so the others'
    /// partial batches do not wait on it (`gpu::Engine::client`; the servers count connections). Batching is
    /// row-invariant, so no result changes. Nothing to let go of, by default.
    fn idle(&self) {}
}

/// `manifest.json`: what `export.py` says of a model.
#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub encoder_hash: String,
    pub text_hash: String,
    #[serde(default)]
    pub commit: Option<String>,
    /// Token count → the graph's file.
    pub buckets: BTreeMap<String, String>,
    #[serde(default)]
    pub parameters: u64,
    #[serde(default)]
    pub metrics: serde_json::Value,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Versioned input contract of a graph. Legacy graphs retain their manifest and weights;
/// only categories and cells assigned after their schema are hidden from their inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputCompatibility {
    Native,
    LegacySnowInputV2,
    LegacyCostInputV3,
}

impl InputCompatibility {
    pub fn category(self, slot: usize, value: u8) -> u8 {
        use cantrip_obs::encode::cat;
        if matches!(self, Self::LegacySnowInputV2 | Self::LegacyCostInputV3) && ((slot == cat::ASK && value == 27) || (slot == cat::TOK && value == 29)) { 0 } else { value }
    }
    /// Newly assigned numeric cells for the model's token kinds.
    pub fn masks(self, kind: u8, feature: usize) -> bool {
        use cantrip_obs::encode::{game, kind as k, option, player};
        let snow = self == Self::LegacySnowInputV2 && match kind {
            k::PLAYER => (player::SNOW_POOL..player::SNOW_POOL + 6).contains(&feature),
            k::GAME => feature == game::SNOW_COST,
            k::OPTION => feature == option::SNOW || feature == option::PAY_SNOW,
            _ => false,
        };
        let old_cost = matches!(self, Self::LegacySnowInputV2 | Self::LegacyCostInputV3) && ((kind == k::OPTION
            && ((option::COST_OPERATION_KINDS..option::COST_OPERATION_KINDS + 15).contains(&feature)
                || feature == option::BLOCK_CAPACITY))
            || (kind == k::GAME && feature == game::MANA_ABILITIES));
        snow || old_cost
    }

    pub fn adapt(self, batch: &mut FeatureBatch) {
        if self == Self::Native { return; }
        for (token, &kind) in batch.kind.iter().enumerate() {
            use cantrip_obs::encode::cat;
            // Cost-choice provenance is separate from the digit token's identity. These slots were unused
            // by older layouts and must remain absent for their graphs.
            if kind == i64::from(cantrip_obs::encode::kind::OPTION) {
                for slot in 2..PTRS {
                    if let Some(pointer) = batch.ptrs.get_mut(token * PTRS + slot) {
                        *pointer = i64::from(NO_POINTER);
                    }
                }
            }
            // Older embeddings have no rows for the new ask or its index digits. Unknown (zero) keeps every old
            // category untouched and avoids asking a legacy graph for an out-of-range embedding.
            let ask = token * CATS + cat::ASK;
            let tok = token * CATS + cat::TOK;
            if let Some(value) = batch.cats.get_mut(ask) { *value = i64::from(self.category(cat::ASK, *value as u8)); }
            if let Some(value) = batch.cats.get_mut(tok) { *value = i64::from(self.category(cat::TOK, *value as u8)); }
            for feature in 0..NUMS {
                if self.masks(kind as u8, feature) {
                    batch.nums[token * NUMS + feature] = 0.0;
                }
            }
        }
    }
}

/// Exact schema-2 layout hashes whose dimensions, categorical vocabularies, and established
/// feature meanings are unchanged. Raw schema 3 and raw schema 4 respectively.
pub const LEGACY_SNOW_V2_HASHES: [&str; 2] = ["ac26dc68cba342fb", "6c8e810d37df2b8e"];

pub fn input_compatibility(manifest: &Manifest) -> Result<InputCompatibility, EvalError> {
    let ours = format!("{:016x}", encoder_hash());
    if manifest.schema == SCHEMA_VERSION && manifest.encoder_hash == ours {
        return Ok(InputCompatibility::Native);
    }
    if manifest.schema == 2 && LEGACY_SNOW_V2_HASHES.contains(&manifest.encoder_hash.as_str()) {
        return Ok(InputCompatibility::LegacySnowInputV2);
    }
    if manifest.schema == 3 && manifest.encoder_hash == "018b410d06b12950" {
        return Ok(InputCompatibility::LegacyCostInputV3);
    }
    Err(err(format!("unsupported encoding schema {} / encoder hash {}; this build {SCHEMA_VERSION} / {ours}",
        manifest.schema, manifest.encoder_hash)))
}

/// A model directory, checked against this build: its manifest, its graphs' paths by token count, and the map from
/// the runtime's card rows to the model's (empty when they are the same index).
#[derive(Clone, Debug)]
pub struct Model {
    pub dir: PathBuf,
    pub manifest: Manifest,
    /// The graph of each of the encoding's `BUCKETS` (a serving bucket's, such as T96, is left out).
    pub graphs: BTreeMap<usize, PathBuf>,
    pub remap: Vec<u32>,
}

impl Model {
    pub fn input_compatibility(&self) -> InputCompatibility {
        input_compatibility(&self.manifest).expect("validated model manifest")
    }

    /// The model in `dir`, its card rows mapped from `table`'s.
    pub fn load(dir: &Path, table: &CardIndex) -> Result<Model, EvalError> {
        let text = std::fs::read_to_string(dir.join("manifest.json"))
            .map_err(|e| err(format!("{}: {e}", dir.join("manifest.json").display())))?;
        let manifest: Manifest =
            serde_json::from_str(&text).map_err(|e| err(format!("manifest.json: {e}")))?;
        let model_cards = || -> Result<Vec<String>, EvalError> {
            let index = std::fs::read_to_string(dir.join("cards.json"))
                .map_err(|e| err(format!("cards.json: {e}")))?;
            let v: serde_json::Value = serde_json::from_str(&index).map_err(err)?;
            let names = v["names"]
                .as_array()
                .ok_or_else(|| err("cards.json has no names"))?;
            Ok(names
                .iter()
                .map(|n| n.as_str().unwrap_or_default().to_string())
                .collect())
        };
        let remap = check(&manifest, table, model_cards)?;
        // The encoding's buckets only: these evaluators run a batch at its encoded size. A graph for a serving bucket
        // between them (T96, `GRAPH_BUCKETS`) is the GPU engine's and serve.py's, and is neither opened nor needed.
        let mut graphs = BTreeMap::new();
        for (tokens, file) in &manifest.buckets {
            let t: usize = tokens
                .parse()
                .map_err(|_| err(format!("a bucket {tokens:?} that is no token count")))?;
            if BUCKETS.contains(&t) {
                graphs.insert(t, dir.join(file));
            }
        }
        for t in BUCKETS {
            if !graphs.contains_key(&t) {
                return Err(err(format!("the model has no graph for {t} tokens")));
            }
        }
        Ok(Model {
            dir: dir.to_path_buf(),
            manifest,
            graphs,
            remap,
        })
    }
}

/// The manifest against this build: a schema or encoder hash of another encoding is refused; another card index is
/// mapped by name (`model_cards`: the model's names, `names[i]` its row `i + 1`).
pub fn check(
    manifest: &Manifest,
    table: &CardIndex,
    model_cards: impl FnOnce() -> Result<Vec<String>, EvalError>,
) -> Result<Vec<u32>, EvalError> {
    check_encoding(manifest)?;
    if manifest.text_hash == table.hash {
        return Ok(Vec::new());
    }
    let rows: HashMap<String, u32> = model_cards()?
        .into_iter()
        .enumerate()
        .map(|(i, n)| (n, i as u32 + 1))
        .collect();
    Ok(std::iter::once(UNKNOWN_CARD)
        .chain(
            table
                .names
                .iter()
                .map(|n| rows.get(n).copied().unwrap_or(UNKNOWN_CARD)),
        )
        .collect())
}

/// Validate the native encoding or an explicitly supported legacy input adapter; unknown encodings are refused.
pub fn check_encoding(manifest: &Manifest) -> Result<(), EvalError> {
    input_compatibility(manifest).map(|_| ())
}

/// The batch sizes a plan is made for: a batch is padded up to the next.
fn batch_size(rows: usize) -> usize {
    rows.max(1).next_power_of_two()
}

pub(crate) fn outputs(b: &FeatureBatch, value: &[f32], policy: &[f32], aux: &[f32]) -> Outputs {
    let t = b.tokens;
    Outputs {
        value: value[..b.rows].to_vec(),
        policy: (0..b.rows)
            .map(|i| {
                let (first, n) = b.options[i];
                (first..first + n)
                    .map(|k| if k < t { policy[i * t + k] } else { MASKED })
                    .collect()
            })
            .collect(),
        aux: (0..b.rows).map(|i| [aux[2 * i], aux[2 * i + 1]]).collect(),
    }
}

type Plan = Arc<TypedRunnableModel>;

/// The pure-Rust backend: tract, one optimised plan per (token count, batch size), made on first use.
pub struct TractEvaluator {
    model: Model,
    plans: Mutex<HashMap<(usize, usize), Plan>>,
}

impl TractEvaluator {
    pub fn load(dir: &Path, table: &CardIndex) -> Result<TractEvaluator, EvalError> {
        Ok(TractEvaluator::new(Model::load(dir, table)?))
    }

    pub fn new(model: Model) -> TractEvaluator {
        TractEvaluator {
            model,
            plans: Mutex::new(HashMap::new()),
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    fn plan(&self, tokens: usize, rows: usize) -> Result<Plan, EvalError> {
        if let Some(p) = self.plans.lock().unwrap().get(&(tokens, rows)) {
            return Ok(p.clone());
        }
        let path = self
            .model
            .graphs
            .get(&tokens)
            .ok_or_else(|| err(format!("no graph for {tokens} tokens")))?;
        let plan = (|| -> TractResult<Plan> {
            tract_onnx::onnx()
                .model_for_path(path)?
                .with_input_fact(0, i64::fact([rows, tokens]).into())?
                .with_input_fact(1, i64::fact([rows, tokens]).into())?
                .with_input_fact(2, i64::fact([rows, tokens, CATS]).into())?
                .with_input_fact(3, i64::fact([rows, tokens, PTRS]).into())?
                .with_input_fact(4, f32::fact([rows, tokens, NUMS]).into())?
                .with_input_fact(5, bool::fact([rows, tokens]).into())?
                .into_optimized()?
                .into_runnable()
        })()
        .map_err(|e| err(format!("{}: {e:?}", path.display())))?;
        self.plans
            .lock()
            .unwrap()
            .insert((tokens, rows), plan.clone());
        Ok(plan)
    }
}

impl Evaluator for TractEvaluator {
    fn evaluate(&self, batch: &FeatureBatch) -> Result<Outputs, EvalError> {
        if batch.rows == 0 {
            return Ok(Outputs::default());
        }
        let tokens = *BUCKETS
            .iter()
            .find(|&&t| t >= batch.tokens)
            .ok_or_else(|| err(format!("{} tokens", batch.tokens)))?;
        let b = if tokens == batch.tokens {
            batch.clone()
        } else {
            rebucket(batch, tokens)
        };
        let rows = batch_size(b.rows);
        let mut x = b.padded(rows, &self.model.remap);
        self.model.input_compatibility().adapt(&mut x);
        let plan = self.plan(tokens, rows)?;
        let t = tokens;
        let run = || -> TractResult<TVec<TValue>> {
            plan.run(tvec!(
                Tensor::from_shape(&[rows, t], &x.kind)?.into(),
                Tensor::from_shape(&[rows, t], &x.card)?.into(),
                Tensor::from_shape(&[rows, t, CATS], &x.cats)?.into(),
                Tensor::from_shape(&[rows, t, PTRS], &x.ptrs)?.into(),
                Tensor::from_shape(&[rows, t, NUMS], &x.nums)?.into(),
                Tensor::from_shape(&[rows, t], &x.legal)?.into(),
            ))
        };
        let out = run().map_err(|e| err(format!("{e:?}")))?;
        let slice = |i: usize| -> Result<Vec<f32>, EvalError> {
            Ok(out[i]
                .try_as_plain_ram()
                .map_err(|e| err(format!("{e:?}")))?
                .as_slice::<f32>()
                .map_err(|e| err(format!("{e:?}")))?
                .to_vec())
        };
        Ok(outputs(&b, &slice(0)?, &slice(1)?, &slice(2)?))
    }
}

impl TractEvaluator {
    /// The network's belief head for one position (`belief` [B,T], the fourth output of an exported graph): per token,
    /// the logit that the card is in an opponent's hand, trained on a pool card's tokens in a mode that hides the hands
    /// (`ai/cantrip_ai/model.py`). The first `e.len` entries, one per real token. For the advisor, which shows it; play
    /// never asks it.
    pub fn belief(&self, e: &Encoded) -> Result<Vec<f32>, EvalError> {
        let batch = FeatureBatch::new(&[e]);
        let tokens = *BUCKETS
            .iter()
            .find(|&&t| t >= batch.tokens)
            .ok_or_else(|| err(format!("{} tokens", batch.tokens)))?;
        let b = if tokens == batch.tokens { batch.clone() } else { rebucket(&batch, tokens) };
        let rows = batch_size(b.rows);
        let mut x = b.padded(rows, &self.model.remap);
        self.model.input_compatibility().adapt(&mut x);
        let plan = self.plan(tokens, rows)?;
        let t = tokens;
        let out = (|| -> TractResult<TVec<TValue>> {
            plan.run(tvec!(
                Tensor::from_shape(&[rows, t], &x.kind)?.into(),
                Tensor::from_shape(&[rows, t], &x.card)?.into(),
                Tensor::from_shape(&[rows, t, CATS], &x.cats)?.into(),
                Tensor::from_shape(&[rows, t, PTRS], &x.ptrs)?.into(),
                Tensor::from_shape(&[rows, t, NUMS], &x.nums)?.into(),
                Tensor::from_shape(&[rows, t], &x.legal)?.into(),
            ))
        })()
        .map_err(|e| err(format!("{e:?}")))?;
        let belief = out
            .get(3)
            .ok_or_else(|| err("the graph has no belief output"))?
            .try_as_plain_ram()
            .map_err(|e| err(format!("{e:?}")))?
            .as_slice::<f32>()
            .map_err(|e| err(format!("{e:?}")))?
            .to_vec();
        Ok(belief.into_iter().take(e.len.min(t)).collect())
    }
}

/// A batch padded out to `tokens` tokens.
fn rebucket(b: &FeatureBatch, tokens: usize) -> FeatureBatch {
    let mut out = FeatureBatch::empty(b.rows, tokens);
    let t = b.tokens;
    for i in 0..b.rows {
        for k in 0..t {
            let (from, to) = (i * t + k, i * tokens + k);
            out.kind[to] = b.kind[from];
            out.card[to] = b.card[from];
            out.legal[to] = b.legal[from];
            out.cats[to * CATS..(to + 1) * CATS]
                .copy_from_slice(&b.cats[from * CATS..(from + 1) * CATS]);
            out.ptrs[to * PTRS..(to + 1) * PTRS]
                .copy_from_slice(&b.ptrs[from * PTRS..(from + 1) * PTRS]);
            out.nums[to * NUMS..(to + 1) * NUMS]
                .copy_from_slice(&b.nums[from * NUMS..(from + 1) * NUMS]);
        }
    }
    out.options = b.options.clone();
    out
}

#[cfg(feature = "ort")]
pub use self::ort_backend::OrtEvaluator;
#[cfg(feature = "gpu")]
pub use self::ort_backend::environment as ort_environment;

#[cfg(feature = "ort")]
mod ort_backend {
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::Mutex;

    use ort::environment::Environment;
    use ort::session::Session;
    use ort::session::builder::GraphOptimizationLevel;
    use ort::value::TensorRef;

    use super::*;

    /// onnxruntime on the CPU: one session per token bucket (the batch is free in the graph).
    pub struct OrtEvaluator {
        model: Model,
        sessions: HashMap<usize, Mutex<Session>>,
    }

    /// onnxruntime allows one environment a process: every evaluator loaded in it shares this one (self-play loads an
    /// evaluator a lane), the GPU engine's sessions among them. With the `gpu` feature onnxruntime is loaded at run
    /// time (`ORT_DYLIB_PATH`), after `CANTRIP_CUDA_LIBS` is put on the library path.
    pub fn environment() -> Result<&'static Environment, EvalError> {
        static ENV: std::sync::OnceLock<Result<Environment, String>> = std::sync::OnceLock::new();
        ENV.get_or_init(|| {
            #[cfg(feature = "gpu")]
            crate::gpu::prepare_libraries();
            ort::init()
                .with_name("cantrip")
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| err(e.clone()))
    }

    impl OrtEvaluator {
        pub fn load(
            dir: &Path,
            table: &CardIndex,
            threads: usize,
        ) -> Result<OrtEvaluator, EvalError> {
            let model = Model::load(dir, table)?;
            let env = environment()?;
            let mut sessions = HashMap::new();
            for (&t, path) in &model.graphs {
                let session = Session::builder(env)
                    .map_err(err)?
                    .with_optimization_level(GraphOptimizationLevel::Level3)
                    .map_err(|e| err(e.to_string()))?
                    .with_intra_threads(threads)
                    .map_err(|e| err(e.to_string()))?
                    .with_inter_threads(1)
                    .map_err(|e| err(e.to_string()))?
                    .commit_from_file(path)
                    .map_err(|e| err(format!("{}: {e}", path.display())))?;
                sessions.insert(t, Mutex::new(session));
            }
            Ok(OrtEvaluator { model, sessions })
        }

        pub fn model(&self) -> &Model {
            &self.model
        }
    }

    impl Evaluator for OrtEvaluator {
        fn evaluate(&self, batch: &FeatureBatch) -> Result<Outputs, EvalError> {
            if batch.rows == 0 {
                return Ok(Outputs::default());
            }
            let tokens = *BUCKETS
                .iter()
                .find(|&&t| t >= batch.tokens)
                .ok_or_else(|| err(format!("{} tokens", batch.tokens)))?;
            let b = if tokens == batch.tokens {
                batch.clone()
            } else {
                rebucket(batch, tokens)
            };
            let (n, t) = (b.rows, tokens);
            let mut x = b.padded(n, &self.model.remap);
            self.model.input_compatibility().adapt(&mut x);
            let mut session = self.sessions[&t].lock().unwrap();
            let out = session
                .run(ort::inputs![
                    "kind" => TensorRef::from_array_view(([n, t], &x.kind[..])).map_err(err)?,
                    "card" => TensorRef::from_array_view(([n, t], &x.card[..])).map_err(err)?,
                    "cats" => TensorRef::from_array_view(([n, t, CATS], &x.cats[..])).map_err(err)?,
                    "ptrs" => TensorRef::from_array_view(([n, t, PTRS], &x.ptrs[..])).map_err(err)?,
                    "nums" => TensorRef::from_array_view(([n, t, NUMS], &x.nums[..])).map_err(err)?,
                    "legal" => TensorRef::from_array_view(([n, t], &x.legal[..])).map_err(err)?,
                ])
                .map_err(err)?;
            let get = |name: &str| -> Result<Vec<f32>, EvalError> {
                Ok(out[name]
                    .try_extract_tensor::<f32>()
                    .map_err(err)?
                    .1
                    .to_vec())
            };
            Ok(outputs(&b, &get("value")?, &get("policy")?, &get("aux")?))
        }
    }
}

/// The parity references `export.py` writes beside a model (`reference/`): positions, and what PyTorch made of each.
pub mod reference {
    use std::path::Path;

    use cantrip_obs::encode::{Encoded, fnv1a64, from_compact, from_compact_legacy_snow_v2};

    use super::{EvalError, Evaluator, FeatureBatch, err};

    /// What a precision below fp32 is held to (`serve.py`'s `REDUCED`): value and dead-next-turn within 1e-2, the policy
    /// within `PolicyDrift::within_reduced`, the life swing within half a point (or 1% of it above `SWING_RELATIVE`
    /// points: fp16's own spacing there is a quarter point and more); and the same best option on 99.5%.
    pub const REDUCED_VALUE: f32 = 1e-2;
    pub const REDUCED_SWING: f32 = 0.5;
    /// The policy's reduced bar, over the references with two legal options or more: the total-variation distance
    /// between the evaluator's legal-option probabilities and PyTorch's, its mean, its 99th percentile (nearest rank)
    /// and its largest; and no best option changed where PyTorch's top two are `TIE` apart or more. (The owner's
    /// decision, 2026-11-13. The bar before, one option's largest probability change, was set by the closest tie among
    /// the references and moved with batching: gen3 of the templates line drifted 0.019 or 0.030 by batch, at a mean
    /// distance of 1e-4.) Measured on the templates line's gen0-gen3 (the server's check, and its graphs under ORT):
    /// fp16 means 4e-5 to 1.3e-4, 99th percentiles 0.0007 to 0.0014, largest 0.002 to 0.030; tf32 means 3e-5 to 7e-5;
    /// no best option changed; bf16 means 5.4e-4 to 8.1e-4 and 99th percentiles 0.008 to 0.018, which fail.
    pub const REDUCED_TV_MEAN: f64 = 2e-4;
    pub const REDUCED_TV_P99: f64 = 5e-3;
    pub const REDUCED_TV_MAX: f64 = 5e-2;
    pub const TIE: f64 = 2e-2;
    pub const SWING_RELATIVE: f32 = 50.0;
    pub const TOLERANCE: f32 = 1e-3;
    pub const SAME_ARGMAX: f64 = 0.995;

    pub fn softmax(z: &[f32]) -> Vec<f64> {
        let m = z.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
        let e: Vec<f64> = z.iter().map(|&x| (x as f64 - m).exp()).collect();
        let s: f64 = e.iter().sum();
        e.into_iter().map(|x| x / s).collect()
    }

    /// The references' policies against PyTorch's, as the reduced bar measures them (`REDUCED_TV_MEAN`).
    #[derive(Clone, Debug, Default)]
    pub struct PolicyDrift {
        tv: Vec<f64>,
        /// References whose best option changed although PyTorch's top two are `TIE` apart or more.
        pub decided_flips: usize,
    }

    impl PolicyDrift {
        /// One reference's legal-option logits, the evaluator's `got` and PyTorch's `want`. A reference with fewer
        /// than two legal options has no choice to drift; one with an output that is not finite is counted apart
        /// (`non_finite`), as its distance has no meaning.
        pub fn add(&mut self, got: &[f32], want: &[f32]) {
            if got.len() < 2 || !got.iter().all(|x| x.is_finite()) {
                return;
            }
            let (a, b) = (softmax(got), softmax(want));
            self.tv
                .push(0.5 * a.iter().zip(&b).map(|(x, y)| (x - y).abs()).sum::<f64>());
            let best = |p: &[f64]| (0..p.len()).max_by(|&i, &j| p[i].total_cmp(&p[j]).then(j.cmp(&i)));
            if best(&a) != best(&b) {
                let mut s = b.clone();
                s.sort_by(|x, y| y.total_cmp(x));
                if s[0] - s[1] >= TIE {
                    self.decided_flips += 1;
                }
            }
        }

        pub fn mean(&self) -> f64 {
            self.tv.iter().sum::<f64>() / self.tv.len().max(1) as f64
        }

        /// The 99th percentile, nearest rank (numpy's `inverted_cdf`, as serve.py takes it).
        pub fn p99(&self) -> f64 {
            let mut s = self.tv.clone();
            s.sort_by(f64::total_cmp);
            let n = s.len();
            if n == 0 {
                0.0
            } else {
                s[((0.99 * n as f64).ceil() as usize).clamp(1, n) - 1]
            }
        }

        pub fn max(&self) -> f64 {
            self.tv.iter().copied().fold(0.0, f64::max)
        }

        pub fn within_reduced(&self) -> bool {
            self.mean() < REDUCED_TV_MEAN
                && self.p99() < REDUCED_TV_P99
                && self.max() < REDUCED_TV_MAX
                && self.decided_flips == 0
        }
    }

    pub struct Reference {
        pub positions: Vec<Encoded>,
        pub value: Vec<f32>,
        pub aux: Vec<[f32; 2]>,
        /// Each position's option logits.
        pub policy: Vec<Vec<f32>>,
        /// `digest` of each position as Python decoded it.
        pub decoded: Vec<u64>,
    }

    /// `positions.bin`: a u32 count, then each position's u32 length and compact bytes.
    pub fn read_positions(bytes: &[u8]) -> Result<Vec<Vec<u8>>, EvalError> {
        let u32_at = |at: usize| {
            bytes
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
                .ok_or_else(|| err("positions.bin is short"))
        };
        let n = u32_at(0)?;
        let mut at = 4;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let k = u32_at(at)?;
            out.push(
                bytes
                    .get(at + 4..at + 4 + k)
                    .ok_or_else(|| err("positions.bin is short"))?
                    .to_vec(),
            );
            at += 4 + k;
        }
        Ok(out)
    }

    pub fn write_positions(positions: &[Vec<u8>]) -> Vec<u8> {
        let mut out = (positions.len() as u32).to_le_bytes().to_vec();
        for p in positions {
            out.extend((p.len() as u32).to_le_bytes());
            out.extend(p);
        }
        out
    }

    fn floats(path: &Path) -> Result<Vec<f32>, EvalError> {
        let b = std::fs::read(path).map_err(|e| err(format!("{}: {e}", path.display())))?;
        Ok(b.chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// FNV-1a 64 of a position's arrays at its bucket's size, as `export.py` digests what Python decoded: kind u8,
    /// card u32, cats u8, ptrs u8, nums f32, legal u8, little-endian, end to end.
    pub fn digest(e: &Encoded) -> u64 {
        let mut b = Vec::new();
        b.extend(&e.kind);
        b.extend(e.card.iter().flat_map(|c| c.to_le_bytes()));
        b.extend(&e.cats);
        b.extend(&e.ptrs);
        b.extend(e.nums.iter().flat_map(|x| x.to_le_bytes()));
        b.extend(e.legal.iter().map(|&l| l as u8));
        fnv1a64(&b)
    }

    impl Reference {
        pub fn load(dir: &Path) -> Result<Reference, EvalError> {
            let raw = std::fs::read(dir.join("positions.bin"))
                .map_err(|e| err(format!("positions.bin: {e}")))?;
            let positions = read_positions(&raw)?
                .iter()
                .map(|p| (if p.first() == Some(&2) { from_compact_legacy_snow_v2(p) } else { from_compact(p) }).ok_or_else(|| err("a position from_compact cannot read")))
                .collect::<Result<Vec<_>, _>>()?;
            let value = floats(&dir.join("value.f32"))?;
            let aux = floats(&dir.join("auxiliary.f32"))?
                .chunks_exact(2)
                .map(|c| [c[0], c[1]])
                .collect();
            let flat = floats(&dir.join("policy.f32"))?;
            let mut at = 0;
            let mut policy = Vec::new();
            for e in &positions {
                policy.push(
                    flat.get(at..at + e.options)
                        .ok_or_else(|| err("policy.f32 is short"))?
                        .to_vec(),
                );
                at += e.options;
            }
            let decoded = std::fs::read(dir.join("decoded.fnv"))
                .map_err(|e| err(format!("decoded.fnv: {e}")))?
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            Ok(Reference {
                positions,
                value,
                aux,
                policy,
                decoded,
            })
        }
    }

    /// How far an evaluator is from the references.
    #[derive(Clone, Debug, Default)]
    pub struct Parity {
        pub positions: usize,
        pub value: f32,
        /// The auxiliary heads' drift: dead-next-turn's, and the life swing's relative to its size above
        /// `SWING_RELATIVE` points (as `serve.py`'s fp32 check holds it): the swing has no bound, and fp32 cannot hold a
        /// swing of a thousand points to 1e-3 (gen3 of the templates line: 0.002 at -1,175, 1.7e-6 of it).
        pub aux: f32,
        /// The life swing's drift, relative above `SWING_RELATIVE` points (`gpu::Check::swing_reduced`).
        pub swing_reduced: f32,
        /// Dead next turn.
        pub dead: f32,
        /// Over the legal options' logits.
        pub policy: f32,
        /// Over the legal options' probabilities: the largest change of one (shown; the reduced bar is `tv_*`).
        pub probability: f32,
        /// The policy's total-variation distance from PyTorch's (`PolicyDrift`): mean, 99th percentile, largest.
        pub tv_mean: f64,
        pub tv_p99: f64,
        pub tv_max: f64,
        /// Best options changed where PyTorch's top two are `TIE` apart or more.
        pub decided_flips: usize,
        /// The fraction of positions whose best legal option is the reference's.
        pub same_argmax: f64,
        /// Positions whose best option is not a legal one.
        pub illegal_argmax: usize,
        /// Positions whose value, aux or a legal option's logit is not finite: `f32::max` passes over NaN, so the drifts
        /// above cannot see one (as `gpu::Check::non_finite` and serve.py's check count it).
        pub non_finite: usize,
        pub seconds: f64,
    }

    /// The evaluator over the references, `batch` positions of one bucket at a time.
    pub fn parity(
        evaluator: &dyn Evaluator,
        r: &Reference,
        batch: usize,
    ) -> Result<Parity, EvalError> {
        let mut p = Parity {
            positions: r.positions.len(),
            ..Parity::default()
        };
        let start = std::time::Instant::now();
        let mut same = 0;
        let mut drift = PolicyDrift::default();
        for bucket in cantrip_obs::encode::BUCKETS {
            let rows: Vec<usize> = (0..r.positions.len())
                .filter(|&i| r.positions[i].bucket == bucket)
                .collect();
            for chunk in rows.chunks(batch.max(1)) {
                let es: Vec<&Encoded> = chunk.iter().map(|&i| &r.positions[i]).collect();
                let out = evaluator.evaluate(&FeatureBatch::new(&es))?;
                for (j, &i) in chunk.iter().enumerate() {
                    let e = &r.positions[i];
                    p.value = p.value.max((out.value[j] - r.value[i]).abs());
                    let swing = (out.aux[j][0] - r.aux[i][0]).abs();
                    let swing_reduced = swing / (r.aux[i][0].abs() / SWING_RELATIVE).max(1.0);
                    let dead = (out.aux[j][1] - r.aux[i][1]).abs();
                    p.aux = p.aux.max(swing_reduced).max(dead);
                    p.swing_reduced = p.swing_reduced.max(swing_reduced);
                    p.dead = p.dead.max(dead);
                    let legal: Vec<usize> = (0..e.options)
                        .filter(|&k| e.legal[e.first_option + k])
                        .collect();
                    if !(out.value[j].is_finite()
                        && out.aux[j].iter().all(|a| a.is_finite())
                        && legal.iter().all(|&k| out.policy[j][k].is_finite()))
                    {
                        p.non_finite += 1;
                    }
                    for &k in &legal {
                        p.policy = p.policy.max((out.policy[j][k] - r.policy[i][k]).abs());
                    }
                    if !legal.is_empty() {
                        let got: Vec<f32> = legal.iter().map(|&k| out.policy[j][k]).collect();
                        let want: Vec<f32> = legal.iter().map(|&k| r.policy[i][k]).collect();
                        for (a, b) in softmax(&got).iter().zip(softmax(&want)) {
                            p.probability = p.probability.max((a - b).abs() as f32);
                        }
                        drift.add(&got, &want);
                    }
                    let best = |row: &[f32]| {
                        legal
                            .iter()
                            .copied()
                            .max_by(|&a, &b| row[a].total_cmp(&row[b]).then(b.cmp(&a)))
                    };
                    match out.argmax(j) {
                        Some(k) if legal.contains(&k) => {}
                        _ => p.illegal_argmax += 1,
                    }
                    same += (best(&out.policy[j]) == best(&r.policy[i])) as usize;
                }
            }
        }
        p.seconds = start.elapsed().as_secs_f64();
        p.same_argmax = same as f64 / p.positions.max(1) as f64;
        (p.tv_mean, p.tv_p99, p.tv_max, p.decided_flips) =
            (drift.mean(), drift.p99(), drift.max(), drift.decided_flips);
        Ok(p)
    }
}
