use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{T}, Discard a card: Draw a card." The discard is a cost (CR 602.2); the draw is the effect (CR 608.2). Haste (CR 702.10) lets
/// it pay the {T} the turn it comes under its controller's control.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(discard_cards(1)), |g, item| effects::draw(g, item, 1), "{T}, Discard a card: Draw a card.")],
    ..oracle::CHARGING_STRIFEKNIGHT
};
