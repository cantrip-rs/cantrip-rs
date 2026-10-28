//! Undying (CR 702.93a): "When this permanent is put into a graveyard from the battlefield, if it had no +1/+1 counters
//! on it, return it to the battlefield under its owner's control with a +1/+1 counter on it." Butcher Ghoul, Hound of
//! Griselbrand, Nearheath Stalker, Strangleroot Geist, Vengeful Vampire, Vorapede and Young Wolf print it. It is persist
//! (`persist`) with the other kind of counter, so both go through `returns_with_counter`.
//!
//! A card with undying lists `trigger(text)` among its `abilities`.

use crate::returns_with_counter::{had_none, returns, trigger as returning};
use cantrip_api::*;

fn had_no_plus_counters(q: &dyn Query, asker: &Asker) -> bool {
    had_none(q, asker, CounterKind::PlusOne)
}

fn returns_with_a_plus_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    returns(g, item, CounterKind::PlusOne)
}

pub const fn trigger(text: &'static str) -> AbilityDef {
    returning(returns_with_a_plus_counter, had_no_plus_counters, text)
}
