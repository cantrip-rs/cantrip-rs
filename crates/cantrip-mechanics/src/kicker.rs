//! Kicker and multikicker (CR 702.33): an optional additional cost, paid once or any number of times. The cost and the
//! count of payments (`Obj::kicked`) are the core's (`CardDef::kicker`); what a card does with them is its own, except
//! "enters with a counter for each time it was kicked" (CR 702.33d, 614.1c), which Batroc, the Leaper (+1/+1) and
//! Everflowing Chalice (charge) print.
//!
//! A card with it gives `enters_with_counters::<KIND>` as its `enters`, `KIND` being `CounterKind::X as usize`.

use cantrip_api::*;

/// Enters with a counter of kind `KIND` for each time it was kicked. An object put onto the battlefield without being
/// cast was never kicked and enters bare.
pub fn enters_with_counters<const KIND: usize>(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[KIND] += g.obj(entering.object).kicked as i16;
    Ok(())
}

/// A kicker whose cost is not mana (CR 702.33a: "Kicker - Sacrifice an artifact or creature"), a card's `optional_cost`.
/// It is announced as an optional cost is, by choosing between its two `labels` before any target, and paying it kicks the
/// spell once (CR 702.33b), so "if this spell was kicked" is the spell's `kicked`, as for any kicker. A kicker with mana is the
/// card's `kicker` instead.
pub const fn non_mana(cost: Cost) -> OptionalCost {
    OptionalCost { cost, kicker: true }
}
