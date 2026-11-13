use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

// Flying and vigilance are the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{U}").and(TAP), effects::loot, "{1}{U}, {T}: Draw a card, then discard a card.")],
    ..oracle::STRIX_LOOKOUT
};
