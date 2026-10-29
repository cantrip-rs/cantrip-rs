use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Mode 1: "Discard your hand, then draw cards equal to the number of cards in target opponent's hand." The count is taken after the
/// discard, as the spell resolves (CR 608.2c). With the one target illegal the spell does nothing (CR 608.2b), so no hand is discarded.
fn discard_then_draw_for_their_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    g.discard_hand(item.controller);
    let cards = g.state().zone(Zone::hand(opponent)).len() as u32;
    g.draw(item.controller, cards);
    Ok(())
}

/// Mode 2: "Discard your hand, then draw cards equal to the number of cards discarded this way": the cards actually discarded (CR
/// 701.9, 121.1), so a card a replacement kept in the hand is not counted.
fn discard_then_draw_as_many(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let discarded = g.discard_hand(item.controller).len() as u32;
    g.draw(item.controller, discarded);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(discard_then_draw_for_their_hand, "Discard your hand, then draw cards equal to the number of cards in target opponent's hand.")
        .targets(&[targets::OPPONENT]),
    mode(discard_then_draw_as_many, "Discard your hand, then draw cards equal to the number of cards discarded this way."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::BORROWED_KNOWLEDGE };
