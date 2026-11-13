use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        discard_cards(1),
        |g, item| effects::draw(g, item, 1),
        "Discard a card: Draw a card. Activate only as a sorcery and only once each turn.",
    )
    .sorcery_speed()
    .once_each_turn()],
    ..oracle::THUNDERHEAD_GUNNER
};
