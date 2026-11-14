use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "Choose one" (CR 700.2): the mode is chosen as the ability goes on the stack.
static MODAL: ModalDef = modes(&[
    mode(tokens::create_a_3_3_beast, "Create a 3/3 green Beast creature token."),
    mode(|g, item| effects::gain_life(g, item, 3), "You gain 3 life."),
    mode(|g, item| effects::draw(g, item, 1), "Draw a card."),
]);

pub static EXTRA: &[&CardDef] = &[&tokens::BEAST_3_3];

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ATTACKS_OR_BLOCKS,
        &MODAL,
        "Whenever this creature attacks or blocks, choose one \u{2014} create a 3/3 green Beast creature token; you gain 3 life; or draw a card.",
    )],
    ..oracle::ELDER_GARGAROTH
};
