use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "Draw a card for each card you've discarded this turn": counted as this resolves, the discard that paid the cost included, and
/// cards discarded by any means this turn - not only to this ability (CR 701.9, 121.1).
fn draw_for_each_card_discarded(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = history::cards_discarded_this_turn(g, item.controller);
    g.draw(item.controller, n);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}, {T}").and(discard_cards(1)),
        draw_for_each_card_discarded,
        "{2}, {T}, Discard a card: Draw a card for each card you've discarded this turn.",
    )],
    ..oracle::MISTY_KNIGHT_HERO_FOR_HIRE
};
