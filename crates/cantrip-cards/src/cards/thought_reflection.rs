use cantrip_api::*;
use cantrip_mechanics::replacements::DRAW_TWO_INSTEAD;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "If you would draw a card, draw two cards instead." (CR 121.6, 614.1a)
    replacements: &[DRAW_TWO_INSTEAD],
    ..oracle::THOUGHT_REFLECTION
};
