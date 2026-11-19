use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_BECOMES_TAPPED,
        |g, item| effects::may_discard_to_draw(g, item, 1),
        "Whenever this creature becomes tapped, you may discard a card. If you do, draw a card.",
    )],
    ..oracle::VOLATILE_WANDERGLYPH
};
