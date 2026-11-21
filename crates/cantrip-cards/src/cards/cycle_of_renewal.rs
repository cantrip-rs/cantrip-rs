use cantrip_api::*;
use cantrip_mechanics::{library, sacrifice_chosen};
use cantrip_oracle as oracle;

/// "Sacrifice a land. Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle."
/// (CR 701.21a, 701.23, 701.24): the controller sacrifices a land of their choice, or the only one there is, and the search
/// happens whether or not they had one to sacrifice (the sentences are not linked by "if you do"); the land sacrificed is
/// already in the graveyard, not the library, when the search starts.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    sacrifice_chosen(g, item.controller, item.object, 1, |c| c.is(CardType::Land))?;
    library::search_for_two_basic_lands_onto_the_battlefield_tapped(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::CYCLE_OF_RENEWAL };
