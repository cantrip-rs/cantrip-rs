use cantrip_api::*;
use cantrip_mechanics::card_types_in_graveyard;
use cantrip_oracle as oracle;

/// "Discard your hand. Then draw X cards, where X is the number of card types among cards in your graveyard." (CR 701.9, 121.1,
/// 205.2a): the whole hand goes first, so X counts the discarded cards too; the spell itself is still on the stack and is not
/// among the cards in the graveyard.
fn discard_your_hand_then_draw_a_card_per_card_type_in_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.discard_hand(item.controller);
    let x = card_types_in_graveyard(g, item.controller) as u32;
    g.draw(item.controller, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], discard_your_hand_then_draw_a_card_per_card_type_in_your_graveyard),
    ..oracle::PEER_PAST_THE_VEIL
};
