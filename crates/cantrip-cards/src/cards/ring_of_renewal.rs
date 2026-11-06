use cantrip_api::*;
use cantrip_mechanics::{discard_at_random, effects};
use cantrip_oracle as oracle;

/// "{5}, {T}: Discard a card at random, then draw two cards." (CR 608.2, 121.1).
fn discard_card_random_draw_two(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    discard_at_random(g, item.controller, 1);
    g.end_sentence();
    effects::draw(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{5}").and(TAP),
            discard_card_random_draw_two,
            "{5}, {T}: Discard a card at random, then draw two cards.",
        ),
    ],
    ..oracle::RING_OF_RENEWAL
};
