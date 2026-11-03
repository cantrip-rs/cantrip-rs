use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Switch target creature's power and toughness until end of turn." (CR 608.2, 611.2a).
fn switch_target_creatures_power_toughness(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    effects::switch_power_toughness_this_turn(g, item.object, target);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], switch_target_creatures_power_toughness),
    ..oracle::TRANSMUTATION
};
