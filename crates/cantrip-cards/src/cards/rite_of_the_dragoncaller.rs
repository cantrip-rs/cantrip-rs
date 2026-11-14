use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::INSTANT_OR_SORCERY_YOU_CAST};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        INSTANT_OR_SORCERY_YOU_CAST,
        tokens::create_a_dragon_5_5,
        "Whenever you cast an instant or sorcery spell, create a 5/5 red Dragon creature token with flying.",
    )],
    ..oracle::RITE_OF_THE_DRAGONCALLER
};

pub static EXTRA: &[&CardDef] = &[&tokens::DRAGON_5_5];
