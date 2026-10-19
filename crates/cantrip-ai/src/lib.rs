//! The AI player's side of the engine (docs/ai-player-plan.md): what each seat knows of the hidden game
//! (`knowledge`), and its observation of a game (`observe`), the plain data of `cantrip-obs` a policy reads; and the worlds a search plays in (`worlds`), dealt from what a seat knows. A seat is
//! told no more than the host would show it: both go through `cantrip_engine::visibility`.

mod knowledge;
mod observe;
mod worlds;

pub use knowledge::Knowledge;
pub use observe::{EVENTS, observe, observe_indexed};
pub use worlds::{Prior, determinize};
