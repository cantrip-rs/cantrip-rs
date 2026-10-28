use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this artifact enters, draw a card."),
        ability(
            cost("{3}{U}").and(SACRIFICE_THIS),
            |g, item| effects::draw(g, item, 2),
            "{3}{U}, Sacrifice this artifact: Draw two cards.",
        ),
    ],
    ..oracle::FUTURIST_FORGE
};
