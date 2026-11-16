use cantrip_api::*;
use cantrip_mechanics::expend::EXPEND_4;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EXPEND_4,
        effects::target_creature_gets_and_gains_trample::<1, 1>,
        "Whenever you expend 4, target creature you control gets +1/+1 and gains trample until end of turn. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::ROUGHSHOD_DUO
};
