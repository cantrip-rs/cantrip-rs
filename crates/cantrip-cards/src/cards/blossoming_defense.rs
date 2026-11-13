use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effects::target_creature_gets_and_gains_hexproof::<2, 2>),
    ..oracle::BLOSSOMING_DEFENSE
};
