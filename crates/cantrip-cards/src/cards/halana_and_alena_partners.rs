use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{haste_until_end_of_turn, targets};
use cantrip_oracle as oracle;

/// "Put X +1/+1 counters on another target creature you control, where X is Halana and Alena's power. That creature gains haste
/// until end of turn": X is worked out once as the ability resolves, from last known information if Halana and Alena has gone
/// (CR 608.2h), a power of zero or less putting no counters (CR 107.1b); the haste follows either way (CR 611.2a), and nothing
/// happens if the target is no longer legal (CR 608.2b).
fn counters_and_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = targets::power_or_last_known(g, item.object);
    // `add_counters` takes counters off for a negative amount, so a negative power is clamped to the no counters CR 107.1b gives.
    g.add_counters(creature, CounterKind::PlusOne, x.max(0));
    g.end_sentence();
    haste_until_end_of_turn(g, item.object, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        counters_and_haste,
        "At the beginning of combat on your turn, put X +1/+1 counters on another target creature you control, where X is Halana and Alena's power. That creature gains haste until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::HALANA_AND_ALENA_PARTNERS
};
