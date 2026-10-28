//! Mentor (CR 702.134a): "Whenever this creature attacks, put a +1/+1 counter on target attacking creature with power less
//! than this creature's power." Each instance triggers separately (CR 702.134b). Barging Sergeant, Blade Instructor, Hammer
//! Dropper, Parhelion Patrol and Sunhome Stalwart print it.
//!
//! A card with mentor lists `MENTOR` among its `abilities`. Its target is judged as the ability is put on the stack and again
//! as it resolves (CR 608.2b, the rulings), against this creature's power then - or as it last existed on the battlefield if
//! it has left (CR 113.7a) - so the ability does nothing once the target's power is no longer less.

use crate::targets::{attacking_creature_filter, power_or_last_known};
use cantrip_api::*;

/// "Target attacking creature with power less than this creature's power": strictly less, so a creature of equal power - this
/// one included - is no target.
fn attacking_creature_with_lesser_power(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    attacking_creature_filter(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).power.is_some_and(|p| p < power_or_last_known(q, asker.source)))
}

fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    Ok(())
}

/// Mentor, with the reminder text its cards print.
pub const MENTOR: AbilityDef = triggered(
    crate::triggers::THIS_ATTACKS,
    put_counter,
    "Mentor (Whenever this creature attacks, put a +1/+1 counter on target attacking creature with lesser power.)",
)
.targets(&[target(attacking_creature_with_lesser_power)]);
