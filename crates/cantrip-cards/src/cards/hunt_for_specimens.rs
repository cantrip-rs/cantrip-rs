use cantrip_api::*;
use cantrip_mechanics::{learn, tokens};
use cantrip_oracle as oracle;

/// "Learn. (You may reveal a Lesson card you own from outside the game and put it into your hand, or discard a card to draw a card.)" (CR 608.2, 111.1).
fn learn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&tokens::PEST_1_1_BLACK_GREEN);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(1));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    g.end_sentence();
    learn::learn(g, item)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], learn),
    ..oracle::HUNT_FOR_SPECIMENS
};

pub static EXTRA: &[&CardDef] = &[&tokens::PEST_1_1_BLACK_GREEN];
