use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS};
use cantrip_oracle as oracle;

/// "Raphael deals 1 damage to target opponent."
fn ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        ping,
        "Alliance \u{2014} Whenever another creature you control enters, Raphael deals 1 damage to target opponent.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::RAPHAEL_TOUGH_TURTLE
};
