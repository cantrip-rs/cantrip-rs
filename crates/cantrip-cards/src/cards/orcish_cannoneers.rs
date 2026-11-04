use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        effects::damage_target_and_you::<2, 3>,
        "{T}: This creature deals 2 damage to any target and 3 damage to you.",
    ).targets(&[targets::ANY])],
    ..oracle::ORCISH_CANNONEERS
};
