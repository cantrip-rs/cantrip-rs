use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "Reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your
/// hand and the other into your graveyard." The steps are `choose::reveal_top_cards_two_piles_one_to_hand_other_to_graveyard`: the cards
/// are revealed to every player (CR 701.20a), an opponent separates them without being targeted (CR 115.10a) and you choose the
/// pile to keep (CR 700.3, 608.2c-d), either pile possibly empty.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    choose::reveal_top_cards_two_piles_one_to_hand_other_to_graveyard::<5, false>(g, item)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::FACT_OR_FICTION };
