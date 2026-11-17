use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// "Whenever one or more nontoken creatures you control die, investigate. This ability triggers only once each turn." (CR 700.4,
/// 603.2c, 603.2h, 701.16a): every nontoken creature of the controller's that dies, this one too, is a triggering, and only the first
/// of the turn happens, so creatures that die together are one trigger and none later that turn is another.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::NONTOKEN_CREATURE_YOU_CONTROL_DIES,
        investigate,
        "Whenever one or more nontoken creatures you control die, investigate. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::HOMICIDE_INVESTIGATOR
};
