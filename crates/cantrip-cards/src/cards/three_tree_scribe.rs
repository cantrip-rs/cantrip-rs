use cantrip_api::*;
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_LEAVES_WITHOUT_DYING;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CREATURE_YOU_CONTROL_LEAVES_WITHOUT_DYING,
        effects::plus_one_counter_on_target,
        "Whenever this creature or another creature you control leaves the battlefield without dying, put a +1/+1 counter on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::THREE_TREE_SCRIBE
};
