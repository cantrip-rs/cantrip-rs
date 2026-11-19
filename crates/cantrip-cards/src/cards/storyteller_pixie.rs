use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ADVENTURE_SPELL_YOU_CAST,
        |g, item| effects::draw(g, item, 1),
        "Whenever you cast an Adventure spell, draw a card.",
    )],
    ..oracle::STORYTELLER_PIXIE
};
