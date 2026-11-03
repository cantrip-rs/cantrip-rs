use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}").and(life(2)), |g, item| effects::draw(g, item, 1), "{2}, Pay 2 life: Draw a card.")],
    ..oracle::BOOK_OF_RASS
};
