use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_DIES};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, effects::target_creature_explores, "When this creature dies, target creature you control explores.")
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::MINERS_GUIDEWING
};
