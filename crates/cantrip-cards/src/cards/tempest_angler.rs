use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::NONCREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(NONCREATURE_SPELL_YOU_CAST, effects::plus_one_counter_on_this, "Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.")],
    ..oracle::TEMPEST_ANGLER
};
