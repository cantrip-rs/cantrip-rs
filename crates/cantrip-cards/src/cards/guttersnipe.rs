use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        effects::damage_each_opponent_n::<2>,
        "Whenever you cast an instant or sorcery spell, this creature deals 2 damage to each opponent.",
    )],
    ..oracle::GUTTERSNIPE
};
