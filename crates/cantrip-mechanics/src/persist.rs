//! Persist (CR 702.79a): "When this permanent is put into a graveyard from the battlefield, if it had no -1/-1 counters
//! on it, return it to the battlefield under its owner's control with a -1/-1 counter on it." Woodfall Primus prints it.
//!
//! A card with persist lists `trigger(text)` among its `abilities`.

use crate::returns_with_counter::{had_none, returns, trigger as returning};
use cantrip_api::*;

fn had_no_minus_counters(q: &dyn Query, asker: &Asker) -> bool {
    had_none(q, asker, CounterKind::MinusOne)
}

fn returns_with_a_minus_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    returns(g, item, CounterKind::MinusOne)
}

pub const fn trigger(text: &'static str) -> AbilityDef {
    returning(returns_with_a_minus_counter, had_no_minus_counters, text)
}
