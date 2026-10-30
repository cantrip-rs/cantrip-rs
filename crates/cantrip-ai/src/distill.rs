//! A perfect-information value distilled into the seat's own view: at a position the seat is asked, worlds are dealt
//! from what it knows (`determinize`, Decklist) - so drawn as its belief would draw them - and a value network trained on full states (`InfoMode::Full` observations) is
//! asked of each world's full state; their mean is the target a Decklist value learns to predict (`--distill-mix`).
//! The policy is never trained on full observations: only this value is read from them.

use cantrip_engine::*;
use cantrip_obs::encode;
use cantrip_obs::factor::Machine;
use cantrip_obs::InfoMode;

use crate::eval::{Evaluator, FeatureBatch};
use crate::{Answering, CardIndex, Knowledge, Prior, determinize, observe_indexed};

/// The mean of `evaluator`'s value over `worlds` worlds dealt for `seat` from `knowledge` at `game`'s position, each read
/// as a full state (the seat's own request, its first step). `None` where the seat is not the one asked, no world could
/// be read, or the network failed. `seed` deals the worlds.
#[allow(clippy::too_many_arguments)]
pub fn world_mean(
    game: &Game,
    seat: PlayerId,
    knowledge: &Knowledge,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    prior: &Prior,
    worlds: usize,
    seed: u64,
) -> Option<f32> {
    if game.pending().is_none_or(|r| r.player != seat) {
        return None;
    }
    let mut encoded = Vec::with_capacity(worlds);
    for w in 0..worlds as u64 {
        let s = splitmix(seed ^ splitmix(w + 1));
        let mut g = game.fork_for_search();
        g.reseed(s);
        determinize(&mut g, knowledge, seat, InfoMode::Decklist, prior, &mut Rng::new(s));
        let Some(request) = g.pending().cloned().filter(|r| r.player == seat) else { continue };
        let Some((obs, index)) = observe_indexed(&g, knowledge, seat, InfoMode::Full) else { continue };
        let answering = Answering::new(&g, &request, &obs, &index);
        let m = Machine::new(&answering.ask, &[]);
        let Some(step) = m.step() else { continue };
        encoded.push(encode::encode(&obs, step, m.chosen(), &m.names(), table));
    }
    if encoded.is_empty() {
        return None;
    }
    let out = evaluator.evaluate(&FeatureBatch::new(&encoded.iter().collect::<Vec<_>>())).ok()?;
    let n = out.value.len().min(encoded.len());
    (n > 0).then(|| out.value[..n].iter().map(|v| v.clamp(-1.0, 1.0)).sum::<f32>() / n as f32)
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}
