use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        |g, item| effects::may_discard_to_draw(g, item, 1),
        "When this creature enters, you may discard a card. If you do, draw a card.",
    )],
    ..oracle::DISCERNING_PEDDLER
};
