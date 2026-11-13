use cantrip_api::*;
use cantrip_mechanics::{flashback, library, select};
use cantrip_oracle as oracle;

/// "Mill three cards, then return an instant or sorcery card from your graveyard to your hand." (CR 701.17, 608.2c): the choice is from
/// the whole graveyard after the mill, a card just milled included, and the spell itself is on the stack, not in the graveyard.
fn mill_then_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_return_a_card(g, item, 3, select::instant_or_sorcery_cards(select::as_many_as_can(1)))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], mill_then_return),
    ways: &[flashback::way(cost("{5}{U}{U}"), "Flashback {5}{U}{U}")],
    ..oracle::INSPIRATION_FROM_BEYOND
};
