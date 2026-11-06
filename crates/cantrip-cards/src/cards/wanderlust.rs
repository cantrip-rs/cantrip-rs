use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[effects::enchanted_permanents_upkeep_damage::<1>(
        "At the beginning of the upkeep of enchanted creature's controller, this Aura deals 1 damage to that player.",
    )],
    ..oracle::WANDERLUST
};
