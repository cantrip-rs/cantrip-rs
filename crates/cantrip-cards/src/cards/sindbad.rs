use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Draw a card and reveal it. If it isn't a land card, discard it." (CR 121.6c, 701.20a).
fn draw_and_reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let drawn = g.draw_with_receipt(item.controller, 1);
    g.reveal(item.controller, &drawn);
    g.end_sentence();
    if let Some(&card) = drawn.first() {
        if !g.chars(card).is(CardType::Land) {
            g.discard_revealed(&[card]);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, draw_and_reveal, "{T}: Draw a card and reveal it. If it isn't a land card, discard it.")],
    ..oracle::SINDBAD
};
