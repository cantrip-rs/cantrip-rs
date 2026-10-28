//! The return both persist (CR 702.79a) and undying (CR 702.93a) make: "When this permanent is put into a graveyard from the
//! battlefield, if it had no [kind] counters on it, return it to the battlefield under its owner's control with a [kind]
//! counter on it." The two differ only in the kind of counter, which the callers name.

use crate::triggers::THIS_DIES;
use cantrip_api::*;

/// "If it had no [kind] counters on it" (CR 603.4): the permanent as it last was on the battlefield.
pub(crate) fn had_none(q: &dyn Query, asker: &Asker, kind: CounterKind) -> bool {
    q.obj(asker.source).counters(kind) == 0
}

/// The return: the card the permanent became in the graveyard, if it is still there (CR 400.7), enters with a counter of
/// `kind` (CR 122.6).
pub(crate) fn returns(g: &mut dyn Game, item: &StackItem, kind: CounterKind) -> Result<(), Suspend> {
    let Some(card) = g.found_after_move(item.object) else { return Ok(()) };
    let owner = g.obj(card).owner;
    g.put_onto_battlefield_with_counters(card, owner, false, &[(kind, 1)])?;
    Ok(())
}

/// The triggered ability, `effect` its return and `had_none` its intervening "if", both of the caller's kind.
pub(crate) const fn trigger(effect: Effect, condition: Condition, text: &'static str) -> AbilityDef {
    triggered(THIS_DIES, effect, text).condition(condition)
}
