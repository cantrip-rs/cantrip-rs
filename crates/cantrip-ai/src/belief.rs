//! Worlds dealt by the belief head (docs/ai-player-plan.md, "Worlds"; the review of 2026-11-13, item 10). The worlds a
//! Decklist search deals take an opponent's hidden hand uniformly from what the seat has not seen of their list
//! (`worlds::determinize`). The network's belief head says, per card of that pool, how likely the opponent holds one:
//! late in a game it beats the uniform prior, which deals lands into a hand at the deck's rate where a real opponent
//! has played them.
//!
//! `hand_weights` turns the head's output at a request into a weight for each card of each opponent's pool: the odds
//! the head gives that the hand holds one, over the odds the uniform deal gives it (`at_least_one`). A card the head
//! rates as the uniform deal does weighs 1, so a head that agrees with the uniform prior deals with the uniform deal's
//! odds (not the same worlds for a seed: the weighted draw takes its own numbers from the generator). The deal then
//! draws the opponent's hidden hand by weighted sampling without replacement (`Prior::with_hand`); the rest of their
//! unseen cards go where they did, uniformly. Every world is still one the seat's knowledge allows: the weights only
//! change how likely each such world is dealt, never which can be.
//!
//! The weight of a card is a ratio of odds clamped to [`MIN_WEIGHT`, `MAX_WEIGHT`]: a head saying "certainly" or
//! "never" still leaves the deal a chance of the other, and a value no number cannot reach the dealer.

use std::collections::BTreeMap;

use cantrip_engine::*;
use cantrip_obs::encode::{self, Encoded, Origin};
use cantrip_obs::factor::Machine;
use cantrip_obs::{InfoMode, ObjectObs, ZoneObs};

use crate::eval::EvalError;
use crate::knowledge::front;
use crate::{Answering, CardIndex, Knowledge, TractEvaluator, observe_indexed};

/// The smallest and largest weight a card's belief gives it, relative to a card the head rates as the uniform deal does.
pub const MIN_WEIGHT: f64 = 1e-3;
pub const MAX_WEIGHT: f64 = 1e3;

/// A network's belief head: per token of an encoded position, the logit that the token's card is in an opponent's
/// hand (trained on a pool card's tokens, `ai/cantrip_ai/model.py`).
pub trait Belief: Send + Sync {
    fn belief(&self, e: &Encoded) -> Result<Vec<f32>, EvalError>;
}

impl Belief for TractEvaluator {
    fn belief(&self, e: &Encoded) -> Result<Vec<f32>, EvalError> {
        TractEvaluator::belief(self, e)
    }
}

/// How an opponent's hidden hand is dealt (`Prior::with_hand`): per opponent, each card (by its front) of their pool
/// and the weight a copy of it is drawn into the hand with; a card not listed weighs 1.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HandWeights {
    pub seats: BTreeMap<PlayerId, BTreeMap<CardId, f64>>,
}

impl HandWeights {
    pub fn is_empty(&self) -> bool {
        self.seats.values().all(BTreeMap::is_empty)
    }

    /// The weight of a copy of `card` in `seat`'s hand.
    pub fn weight(&self, seat: PlayerId, card: CardId) -> f64 {
        self.seats.get(&seat).and_then(|w| w.get(&card)).copied().unwrap_or(1.0)
    }
}

/// The chance that a draw of `hand` cards without replacement from `pool` cards, `copies` of them one card, holds at
/// least one copy: 1 - C(pool - copies, hand) / C(pool, hand).
pub fn at_least_one(pool: u32, copies: u32, hand: u32) -> f64 {
    if hand == 0 || copies == 0 {
        return 0.0;
    }
    if hand > pool.saturating_sub(copies) {
        return 1.0;
    }
    let mut none = 1.0f64;
    for i in 0..hand {
        none *= f64::from(pool - copies - i) / f64::from(pool - i);
    }
    1.0 - none
}

fn odds(p: f64) -> f64 {
    let p = p.clamp(1e-6, 1.0 - 1e-6);
    p / (1.0 - p)
}

/// The weights the belief head gives each opponent's pool at the request `game` waits on, for `seat` (see the
/// module): `None` in a mode with no pools (Full, Hidden), or where no opponent has a hidden card in hand.
pub fn hand_weights(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    belief: &dyn Belief,
    table: &CardIndex,
) -> Result<Option<HandWeights>, EvalError> {
    if mode != InfoMode::Decklist {
        return Ok(None);
    }
    let seat = request.player;
    let (obs, index) = observe_indexed(game, knowledge, seat, mode).ok_or_else(|| EvalError("no observation of the request (it is deferred)".into()))?;
    let answering = Answering::new(game, request, &obs, &index);
    let m = Machine::new(&answering.ask, &[]);
    let Some(step) = m.step() else {
        return Ok(None);
    };
    let e = encode::encode(&obs, step, m.chosen(), &m.names(), table);
    let mut out = HandWeights::default();
    let mut logits: Option<Vec<f32>> = None;
    for (p, pool) in obs.pools.iter().enumerate() {
        if p == seat.index() || pool.is_empty() {
            continue;
        }
        let unseen: u32 = pool.iter().map(|c| u32::from(c.count)).sum();
        let hand = obs.players.get(p).map_or(0, |pl| u32::from(pl.hand));
        let known = obs
            .objects
            .iter()
            .filter(|o| match o {
                ObjectObs::Hidden { place, owner, identity, .. } => usize::from(*owner) == p && place.zone == ZoneObs::Hand && identity.is_some(),
                ObjectObs::Visible(v) => usize::from(v.owner) == p && v.place.zone == ZoneObs::Hand,
                _ => false,
            })
            .count() as u32;
        let hidden = hand.saturating_sub(known);
        if hidden == 0 {
            continue;
        }
        if logits.is_none() {
            logits = Some(belief.belief(&e)?);
        }
        let l = logits.as_deref().unwrap_or_default();
        let mut weights = BTreeMap::new();
        for c in pool {
            let Some(token) = e.origin.iter().position(|o| matches!(o, Origin::PoolCard { player, name } if usize::from(*player) == p && *name == c.name)) else {
                continue;
            };
            let Some(&x) = l.get(token) else { continue };
            if !x.is_finite() {
                continue;
            }
            let Some(id) = card(&c.name) else { continue };
            let believed = 1.0 / (1.0 + (-f64::from(x)).exp());
            let uniform = at_least_one(unseen, u32::from(c.count), hidden);
            let w = (odds(believed) / odds(uniform)).clamp(MIN_WEIGHT, MAX_WEIGHT);
            weights.insert(front(game, id), w);
        }
        if !weights.is_empty() {
            out.seats.insert(PlayerId(p as u8), weights);
        }
    }
    Ok((!out.is_empty()).then_some(out))
}
