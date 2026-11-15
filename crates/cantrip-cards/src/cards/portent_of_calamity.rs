use cantrip_api::*;
use cantrip_mechanics::{choose, top_of_library};
use cantrip_oracle as oracle;

/// "Reveal the top X cards of your library. For each card type, you may exile a card of that type from among them. Put the rest into
/// your graveyard. You may cast a spell from among the exiled cards without paying its mana cost if you exiled four or more cards
/// this way. Then put the rest of the exiled cards into your hand." (CR 701.20a, 406.2, 400.3, 118.9, 608.2g)
///
/// The cards not exiled go to the graveyard before anything is cast. "Four or more cards" counts the cards exiled, however many are
/// castable; a land among them, or a card that can't be cast, is not offered but goes to the hand with the rest. A card cast
/// is a new object (CR 400.7), so only what is still in exile goes to the hand.
fn portent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed = top_of_library(g, you, item.x as usize);
    if revealed.is_empty() {
        return Ok(());
    }
    g.reveal(you, &revealed);
    let chosen = choose::one_of_each_card_type(g, you, item.object, Purpose::Exile, &revealed)?;
    let exiled = g.exile_all(&chosen);
    g.put_into_graveyard(&revealed);
    g.end_sentence();
    if exiled.len() >= 4 {
        choose::cast_up_to_free(g, you, item.object, &exiled, 1)?;
    }
    g.end_sentence();
    g.return_to_hand_revealed(&exiled);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], portent), ..oracle::PORTENT_OF_CALAMITY };
