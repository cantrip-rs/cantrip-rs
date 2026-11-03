use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(sacrifice(1, targets::filters::artifact)),
            |g, item| effects::draw(g, item, 1),
            "{T}, Sacrifice an artifact: Draw a card.",
        ),
    ],
    ..oracle::SAGE_OF_LAT_NAM
};
