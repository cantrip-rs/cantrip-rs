use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::NONCREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        NONCREATURE_SPELL_YOU_CAST,
        effects::damage_each_opponent_n::<1>,
        "Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::FIREBRAND_ARCHER
};
