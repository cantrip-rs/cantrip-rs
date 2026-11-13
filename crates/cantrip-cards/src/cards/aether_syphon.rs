use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::triggers::YOU_DRAW_A_CARD;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[ability(cost("{2}").and(TAP), |g, item| effects::draw(g, item, 1), "{2}, {T}: Draw a card.")],
    delayed: &[triggered(YOU_DRAW_A_CARD, effects::each_opponent_mills::<2>, "Max speed \u{2014} Whenever you draw a card, each opponent mills two cards.")],
    statics: &[max_speed("Max speed \u{2014} Whenever you draw a card, each opponent mills two cards.")],
    ..oracle::AETHER_SYPHON
};
