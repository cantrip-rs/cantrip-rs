use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::seek_out::seek_out;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        seek_out(
            effects::damage_each_opponent_n::<2>,
            &[],
            "Seek out — Whenever you draw your second card each turn, Ro Laren deals 2 damage to each opponent.",
        ),
        ability(
            cost("{1}").and(TAP).and(discard_cards(1)),
            |g, item| effects::draw(g, item, 1),
            "{1}, {T}, Discard a card: Draw a card.",
        ),
    ],
    ..oracle::RO_LAREN_DOUBLE_AGENT
};
