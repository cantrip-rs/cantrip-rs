use cantrip_api::*;
use cantrip_mechanics::learn;
use cantrip_oracle as oracle;

/// "Draw a card. Learn. (You may reveal a Lesson card you own from outside the game and put it into your hand, or discard a card to draw a card.)" (CR 608.2, 121.1).
fn draw_card_learn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.end_sentence();
    learn::learn(g, item)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], draw_card_learn),
    ..oracle::POP_QUIZ
};
