use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gains banding until end of turn." (CR 611.2a, 702.22).
fn grant_banding(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Banding);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}"), grant_banding, "{2}: Target creature gains banding until end of turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::BATON_OF_MORALE
};
