use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

// Convoke is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::draw_three_then_discard),
    ..oracle::UNEXPECTED_ASSISTANCE
};
