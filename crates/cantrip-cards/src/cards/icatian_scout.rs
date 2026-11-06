use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gains first strike until end of turn." (CR 611.2a).
fn grant_first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}").and(TAP), grant_first_strike,
        "{1}, {T}: Target creature gains first strike until end of turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::ICATIAN_SCOUT
};
