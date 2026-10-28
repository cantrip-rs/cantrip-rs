use cantrip_api::*;
use cantrip_mechanics::{effects, history, tokens, triggers};
use cantrip_oracle as oracle;

/// "Each opponent loses 3 life."
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, 3);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(|g, item| effects::draw(g, item, 1), "Draw a card."),
    mode(tokens::create_a_treasure, "Create a Treasure token."),
    mode(drain, "Each opponent loses 3 life."),
])
.allowed(history::mode_not_chosen_this_turn);

/// The Treasure token.
pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::YOU_DISCARD,
        &MODAL,
        "Whenever you discard a card, choose one that hasn't been chosen this turn \u{2014} draw a card; or create a Treasure token; or each opponent loses 3 life.",
    )],
    ..oracle::MONUMENT_TO_ENDURANCE
};
