use cantrip_api::*;
use cantrip_mechanics::amass::amass_goblins;
use cantrip_mechanics::{tokens, triggers::NONCREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(NONCREATURE_SPELL_YOU_CAST, amass_goblins::<1>, "Whenever you cast a noncreature spell, amass Goblins 1.")],
    ..oracle::BOTHERSOME_NOISEMAKER
};
