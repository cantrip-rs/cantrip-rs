use cantrip_api::*;
use cantrip_mechanics::{targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Target creature gains trample until end of turn." (CR 608.2b, 611.2a).
fn grant_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    trample_until_end_of_turn(g, item.object, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}").and(TAP), grant_trample,
        "{3}, {T}: Target creature gains trample until end of turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::WAR_CHARIOT
};
