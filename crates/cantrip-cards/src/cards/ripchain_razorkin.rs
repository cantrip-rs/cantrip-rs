use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{R}").and(sacrifice(1, targets::filters::land)), |g, item| effects::draw(g, item, 1), "{2}{R}, Sacrifice a land: Draw a card.")],
    ..oracle::RIPCHAIN_RAZORKIN
};
