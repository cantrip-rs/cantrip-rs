//! The AI player's side of the engine (docs/ai-player-plan.md): what each seat knows of the hidden game
//! (`knowledge`), and its observation of a game (`observe`), the plain data of `cantrip-obs` a policy reads; and the worlds a search plays in (`worlds`), dealt from what a seat knows. A seat is
//! told no more than the host would show it: both go through `cantrip_engine::visibility`. An answer the policy makes
//! step by step (`cantrip_obs::factor`) is turned back into the engine's and checked (`answer`); payment is no policy
//! step (`pay`). The card index a position's cards are encoded with is `index`, the engine's own cards. A finished game is
//! made into training samples by `record`, written to and read from zstd shards by `shard`. A trained network is
//! loaded and run by `eval` (tract by default, onnxruntime behind the `ort` feature), or on the inference server by
//! `remote`, or on the GPU in this process by `gpu` (behind the `gpu` feature, which the native inference server is
//! built on), and `net` answers a request with it alone.

pub mod alloc;
pub mod atomic;
mod answer;
mod ask;
pub mod cancel;
pub mod distill;
pub mod eval;
#[cfg(feature = "gpu")]
pub mod gpu;
mod knowledge;
pub mod net;
mod observe;
pub mod pay;
pub mod record;
pub mod remote;
pub mod search;
pub mod shard;
pub mod trace;
mod index;
pub mod issearch;
mod worlds;

pub use eval::{EvalError, Evaluator, FeatureBatch, Manifest, Model, Outputs, TractEvaluator};
pub use answer::{Answering, Rejection, factored_name, final_x};
pub use knowledge::Knowledge;
pub use observe::{EVENTS, kind_name, observe, observe_indexed};
pub use index::{CardIndex, CardRows};
pub use remote::RemoteEvaluator;
pub use worlds::{Prior, determinize, determinize_owned};
