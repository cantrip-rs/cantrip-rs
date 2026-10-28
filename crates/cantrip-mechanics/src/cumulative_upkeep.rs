//! Cumulative upkeep [cost] (CR 702.24a): "At the beginning of your upkeep, if this permanent is on the battlefield, put an
//! age counter on this permanent. Then you may pay [cost] for each age counter on it. If you don't, sacrifice it." Aboroth,
//! Firestorm Hellkite, Gallowbraid, Illusionary Forces, Illusionary Wall, Jötun Grunt, Morinfen, Phyrexian Soulgorger,
//! Sheltering Ancient, Uktabi Efreet, Volunteer Reserves, Wall of Shards and Yavimaya Ants print it.
//!
//! A card with cumulative upkeep lists `cumulative_upkeep!(payment, text)` among its `abilities`, `payment` being one
//! [`Payment`]: the cost paid once for each age counter. The whole is paid or none of it (CR 702.24a: "partial payments
//! aren't allowed"): the payer is asked once, before anything is paid, and only if every one of the payments can be made;
//! each choice the cost has (which creature to sacrifice, which opponent gains the life, which creature gets the counter,
//! which graveyard) is made separately for each age counter. Each instance triggers separately, and counts the permanent's
//! age counters as it resolves, not only the ones it put (CR 702.24b).
//!
//! The payments the engine pays as costs (mana, life, sacrifices, counters put on this permanent) are paid by `may_pay`,
//! the cost multiplied by the age counters; the others are asked and made here, in the order the cost says.

use crate::{choose, library};
use cantrip_api::*;

#[doc(hidden)]
pub use cantrip_api as api;

/// The cost of a cumulative upkeep ability, paid once for each age counter (CR 702.24a).
#[derive(Clone, Copy)]
pub enum Payment {
    /// "Cumulative upkeep {1}": mana, `times` the age counters over (a cost whose multiple doesn't fit a mana cost can't be
    /// paid by anyone).
    Mana(ManaCost),
    /// "Cumulative upkeep--Pay 1 life.": this much life for each age counter (CR 119.4).
    Life(u8),
    /// "Cumulative upkeep--Sacrifice a creature.": one permanent that passes the filter for each age counter, each a different
    /// one, the permanent itself included (Phyrexian Soulgorger's ruling).
    Sacrifice(ObjectFilter),
    /// "Cumulative upkeep--Put a -1/-1 counter on this creature.": one for each age counter, put on the permanent itself.
    PutMinusOneMinusOneCounterOnThis,
    /// "Cumulative upkeep--Put a +1/+1 counter on a creature an opponent controls.": one for each age counter, each on a
    /// creature chosen separately, not targeted (Sheltering Ancient's rulings).
    PutPlusOnePlusOneCounterOnAnOpposingCreature,
    /// "Cumulative upkeep--An opponent gains 1 life.": this much for each age counter, each to an opponent chosen separately
    /// (Wall of Shards's ruling); a cost that can't be paid while they can't gain life (CR 119.7).
    OpponentGainsLife(u8),
    /// "Cumulative upkeep--Put two cards from a single graveyard on the bottom of their owner's library.": two cards of one
    /// graveyard for each age counter, a graveyard chosen separately for each (Jötun Grunt's ruling).
    TwoCardsFromAGraveyardOnTheBottom,
}

const CUMULATIVE: &str = "Cumulative upkeep: it gets an age counter, and the cost is paid once for each";

/// "Pay [mana and life], or sacrifice it" for a cost the prompt shows whole, its `{cost}` already multiplied.
pub const PAY: Question = Question::new("CumulativeUpkeep", "pay {cost}, or sacrifice {subject}?", CUMULATIVE, "Pay {cost}", "Sacrifice it").no_default();

const SACRIFICE: Question = Question::new(
    "CumulativeUpkeepSacrifice",
    "sacrifice a creature for each age counter, or sacrifice {subject}?",
    CUMULATIVE,
    "Sacrifice the creatures",
    "Sacrifice it",
)
.no_default();

const COUNTER_ON_THIS: Question = Question::new(
    "CumulativeUpkeepCounters",
    "put a -1/-1 counter on {subject} for each age counter, or sacrifice it?",
    CUMULATIVE,
    "Put the counters",
    "Sacrifice it",
)
.no_default();

const COUNTER_ON_OPPOSING: Question = Question::new(
    "CumulativeUpkeepCounters",
    "put a +1/+1 counter on a creature an opponent controls for each age counter on {source}, or sacrifice it?",
    CUMULATIVE,
    "Put the counters",
    "Sacrifice it",
)
.no_default();

const LIFE_FOR_OPPONENTS: Question = Question::new(
    "CumulativeUpkeepLife",
    "have an opponent gain life for each age counter on {source}, or sacrifice it?",
    CUMULATIVE,
    "Have them gain life",
    "Sacrifice it",
)
.no_default();

const BOTTOM_OF_LIBRARY: Question = Question::new(
    "CumulativeUpkeepLibrary",
    "put two cards from a single graveyard on the bottom of their owner's library for each age counter on {source}, or sacrifice it?",
    CUMULATIVE,
    "Put the cards",
    "Sacrifice it",
)
.no_default();

fn your_upkeep(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::Upkeep } if q.state().turn.active == q.controller(source) => Fired::PLAIN,
        _ => None,
    }
}

/// Whether the creatures of the opponents of `payer` that a counter could be put on are there: any creature on the battlefield
/// one of them controls.
fn opposing_creatures(g: &dyn Game, payer: PlayerId) -> Vec<ObjectId> {
    let opponents: Vec<PlayerId> = g.state().opponents(payer).collect();
    g.state().battlefield().iter().copied().filter(|&id| opponents.contains(&g.controller(id)) && g.chars(id).is(CardType::Creature)).collect()
}

/// The graveyards with two cards in them or more, who owns each.
fn graveyards_with_two(g: &dyn Game) -> Vec<PlayerId> {
    g.state().seats().filter(|&p| g.state().zone(Zone::graveyard(p)).len() >= 2).collect()
}

/// Whether `age` payments of two cards from a single graveyard can all be made: each takes two cards of one graveyard, so a
/// graveyard of k cards gives k / 2 of them.
fn graveyards_cover(g: &dyn Game, age: u32) -> bool {
    g.state().seats().map(|p| g.state().zone(Zone::graveyard(p)).len() as u32 / 2).sum::<u32>() >= age
}

/// Pays `payment` for `age` age counters, if the payer chooses to and can: whether it was paid. All of it or none (CR
/// 702.24a).
fn pay(g: &mut dyn Game, item: &StackItem, payment: Payment, age: u32) -> Result<bool, Suspend> {
    let (payer, this) = (item.controller, item.object);
    let count = u8::try_from(age);
    let may = |g: &mut dyn Game, question: &'static Question| Ok(matches!(g.ask(payer, RequestKind::May { source: this, purpose: Purpose::Ask(question) })?, Answer::Yes));
    match payment {
        Payment::Mana(mana) => match mana.times(u16::try_from(age).unwrap_or(u16::MAX)) {
            Some(total) => g.may_pay(payer, this, this, Cost::mana(total), &PAY),
            None => Ok(false),
        },
        Payment::Life(life) => g.may_pay(payer, this, this, Cost { life: u32::from(life) * age, ..Cost::NONE }, &PAY),
        Payment::Sacrifice(filter) => match count {
            Ok(count) => g.may_pay(payer, this, this, Cost::sacrifice(count, filter), &SACRIFICE),
            Err(_) => Ok(false),
        },
        Payment::PutMinusOneMinusOneCounterOnThis => match count {
            Ok(count) => g.may_pay(payer, this, this, put_counters(CounterKind::MinusOne, count), &COUNTER_ON_THIS),
            Err(_) => Ok(false),
        },
        Payment::PutPlusOnePlusOneCounterOnAnOpposingCreature => {
            if opposing_creatures(g, payer).is_empty() || !may(g, &COUNTER_ON_OPPOSING)? {
                return Ok(false);
            }
            for _ in 0..age {
                let candidates = opposing_creatures(g, payer);
                let purpose = Purpose::PutCounters(CounterKind::PlusOne, 1);
                if let Some(&creature) = choose::cards_from_kind(g, payer, this, purpose, "creature", candidates.clone(), candidates, 1, false)?.first() {
                    g.add_counters(creature, CounterKind::PlusOne, 1);
                }
            }
            Ok(true)
        }
        Payment::OpponentGainsLife(life) => {
            let can = |g: &dyn Game| g.state().opponents(payer).filter(|&p| g.can_gain_life(p)).collect::<Vec<_>>();
            if can(g).is_empty() || !may(g, &LIFE_FOR_OPPONENTS)? {
                return Ok(false);
            }
            for _ in 0..age {
                let candidates = can(g);
                let chosen = match candidates.as_slice() {
                    [only] => Some(*only),
                    _ => choose::player(g, payer, this, candidates, "GainLife", u16::from(life))?,
                };
                if let Some(opponent) = chosen {
                    g.gain_life(opponent, i32::from(life));
                }
            }
            Ok(true)
        }
        Payment::TwoCardsFromAGraveyardOnTheBottom => {
            if !graveyards_cover(g, age) || !may(g, &BOTTOM_OF_LIBRARY)? {
                return Ok(false);
            }
            for _ in 0..age {
                let owners = graveyards_with_two(g);
                let owner = match owners.as_slice() {
                    [only] => Some(*only),
                    _ => choose::player(g, payer, this, owners, "PutOnBottom", 2)?,
                };
                let Some(owner) = owner else { continue };
                let shown = g.state().zone(Zone::graveyard(owner)).to_vec();
                let chosen = choose::cards_from_kind(g, payer, this, Purpose::PutOnBottom, "", shown.clone(), shown, 2, false)?;
                let order = library::ask_library_order(g, owner, this, &chosen, DestinationKind::LibraryBottom)?;
                g.put_into_library(&order, LibraryPlace::Bottom);
            }
            Ok(true)
        }
    }
}

/// The ability's resolution (CR 702.24a): if the permanent is on the battlefield, an age counter is put on it, then its
/// controller pays `payment` for each age counter on it (counted now, CR 702.24b) or sacrifices it.
pub fn resolve(g: &mut dyn Game, item: &StackItem, payment: Payment) -> Result<(), Suspend> {
    let this = item.object;
    if !g.is_on_battlefield(this) || g.obj(this).became.is_some() {
        return Ok(());
    }
    g.add_counters(this, CounterKind::Age, 1);
    g.end_sentence();
    let age = g.obj(this).counters(CounterKind::Age).max(0) as u32;
    if !pay(g, item, payment, age)? {
        g.sacrifice(item.controller, &[this]);
    }
    Ok(())
}

/// The triggered ability of cumulative upkeep, its effect `pay` being the card's `resolve` with its payment. Use
/// `cumulative_upkeep!`.
pub const fn ability(pay: Effect, text: &'static str) -> AbilityDef {
    triggered(when(EventMask::STEP_BEGAN, your_upkeep), pay, text)
}

/// Cumulative upkeep [cost] (CR 702.24a): `cumulative_upkeep!(payment, text)`; see the module.
#[macro_export]
macro_rules! cumulative_upkeep {
    ($payment:expr, $text:expr) => {{
        fn pay(g: &mut dyn $crate::cumulative_upkeep::api::Game, item: &$crate::cumulative_upkeep::api::StackItem) -> Result<(), $crate::cumulative_upkeep::api::Suspend> {
            $crate::cumulative_upkeep::resolve(g, item, $payment)
        }
        $crate::cumulative_upkeep::ability(pay, $text)
    }};
}
