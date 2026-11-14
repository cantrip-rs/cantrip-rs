use cantrip_api::*;
use cantrip_mechanics::{effects, raid, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[raid::enters(
        effects::damage_first_target::<2>,
        "Raid \u{2014} When this creature enters, if you attacked this turn, this creature deals 2 damage to any target.",
    )
    .targets(&[targets::ANY])],
    ..oracle::GOREHORN_RAIDER
};
