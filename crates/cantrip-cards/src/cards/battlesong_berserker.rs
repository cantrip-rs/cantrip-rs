use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::YOU_ATTACK};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        effects::target_creature_gets_and_gains_menace::<1, 0>,
        "Whenever you attack, target creature you control gets +1/+0 and gains menace until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::BATTLESONG_BERSERKER
};
