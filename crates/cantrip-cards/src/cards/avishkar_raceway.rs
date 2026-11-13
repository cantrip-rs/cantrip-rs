use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    // "Max speed — {3}, {T}, Discard a card: Draw a card" (CR 702.178a): granted by the static below while its controller's
    // speed is 4.
    delayed: &[ability(cost("{3}").and(TAP).and(discard_cards(1)), |g, item| effects::draw(g, item, 1), "{3}, {T}, Discard a card: Draw a card.")],
    statics: &[max_speed("Max speed \u{2014} {3}, {T}, Discard a card: Draw a card.")],
    ..oracle::AVISHKAR_RACEWAY
};
