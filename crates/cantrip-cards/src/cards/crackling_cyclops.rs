use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::NONCREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(NONCREATURE_SPELL_YOU_CAST, effects::this_gets::<3, 0>, "Whenever you cast a noncreature spell, this creature gets +3/+0 until end of turn.")],
    ..oracle::CRACKLING_CYCLOPS
};
