use cantrip_api::*;
use cantrip_mechanics::select::{self, up_to};
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "Mill a card. You may put a land card milled this way into your hand. You gain 2 life if a Lesson card is milled this way."
/// (CR 701.17, 608.2c): the land is taken from the card the mill moved, as it is where it went (a graveyard, or exile by a
/// replacement effect), and nobody is asked when it is no land. The life is gained whether or not a land was taken, and only
/// when the card milled is a Lesson card (CR 205.3k, 701.17a), never for a Lesson that was in the graveyard already.
fn mill_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let milled = g.mill(you, 1);
    if milled.iter().any(|&card| g.chars(card).is(CardType::Land)) {
        let chosen = choose::select(g, you, item.object, Purpose::PutIntoHand, milled.clone(), select::land_cards(up_to(1)))?;
        g.return_to_hand(&chosen);
    }
    if milled.iter().any(|&card| g.chars(card).has_subtype(Subtype::Lesson)) {
        g.gain_life(you, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        mill_a_card,
        "{T}: Mill a card. You may put a land card milled this way into your hand. You gain 2 life if a Lesson card is milled this way. (To mill a card, put the top card of your library into your graveyard.)",
    )],
    ..oracle::SPARRING_DUMMY
};
