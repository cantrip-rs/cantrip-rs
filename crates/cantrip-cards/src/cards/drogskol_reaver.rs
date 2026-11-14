use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::YOU_GAIN_LIFE};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_GAIN_LIFE, |g, item| effects::draw(g, item, 1), "Whenever you gain life, draw a card.")],
    ..oracle::DROGSKOL_REAVER
};
