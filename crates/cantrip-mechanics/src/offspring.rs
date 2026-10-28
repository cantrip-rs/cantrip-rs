//! Offspring [cost] (CR 702.175a): "You may pay an additional [cost] as you cast this spell. If you do, when this
//! creature enters, create a 1/1 token that's a copy of it." Thundertrap Trainer and Pawpatch Recruit print it.
//!
//! A card with offspring gives `kicker(cost)` as its `kicker` and lists `trigger(text)` among its `abilities`.

use crate::triggers::THIS_ENTERS;
use cantrip_api::*;

/// The additional cost, paid once.
pub const fn kicker(cost: ManaCost) -> Kicker {
    Kicker { cost, extra: None, multi: false, waterbend: false }
}

/// The intervening "if" (CR 603.4): its additional cost was paid.
fn offspring_paid(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).kicked > 0
}

/// A copy of the creature that entered, except that it is 1/1 (CR 707.2, 707.9b).
fn create_offspring(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let values = g.copiable_values(item.object).sized(1, 1);
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

pub const fn trigger(text: &'static str) -> AbilityDef {
    triggered(THIS_ENTERS, create_offspring, text)
        .condition(offspring_paid)
}
