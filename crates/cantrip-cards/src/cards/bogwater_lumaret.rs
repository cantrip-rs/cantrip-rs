use cantrip_api::*;
use cantrip_mechanics::effects::gain_one_life;
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever this creature or another creature you control enters, you gain 1 life" (CR 603.6a): this creature itself and
/// tokens included.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CREATURE_YOU_CONTROL_ENTERS,
        gain_one_life,
        "Whenever this creature or another creature you control enters, you gain 1 life.",
    )],
    ..oracle::BOGWATER_LUMARET
};
