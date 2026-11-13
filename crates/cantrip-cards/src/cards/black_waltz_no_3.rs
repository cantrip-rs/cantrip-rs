use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::NONCREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        NONCREATURE_SPELL_YOU_CAST,
        effects::damage_each_opponent_n::<2>,
        "Whenever you cast a noncreature spell, Black Waltz No. 3 deals 2 damage to each opponent.",
    )],
    ..oracle::BLACK_WALTZ_NO_3
};
