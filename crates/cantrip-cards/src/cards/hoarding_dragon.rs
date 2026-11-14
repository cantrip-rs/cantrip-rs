use cantrip_api::*;
use cantrip_mechanics::library::{self, ask_may_search};
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

const SEARCH: Question = Question::new("SearchLibrary", "search your library for an artifact card?", "It is exiled, then you shuffle", "Search", "Don't search");

const RETURN: Question = Question::new("ReturnExiledCard", "put the exiled card into its owner's hand?", "", "Put it into hand", "Leave it");

/// "You may search your library for an artifact card, exile it, then shuffle" (CR 701.23, 701.24): the card is exiled face up
/// and linked to this ability's source (CR 607.2a) for the dies ability to find. A declined search shuffles nothing.
fn hoard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if !ask_may_search(g, player, item.object, &SEARCH)? {
        return Ok(());
    }
    let found = library::search(g, player, player, item.object, SearchDestination::Exile, select::ARTIFACT_CARD)?;
    g.exile_with(&found, g.linked(item), false);
    g.shuffle(player);
    Ok(())
}

/// "You may put the exiled card into its owner's hand" (CR 607.2a): the card this Dragon's own first ability exiled, if it is
/// still in exile. Nothing is asked when there is none.
fn take_hoard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.exiled_with(g.linked(item)).is_empty() || !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&RETURN))? {
        return Ok(());
    }
    effects::return_exiled_to_hand(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, hoard, "When this creature enters, you may search your library for an artifact card, exile it, then shuffle."),
        triggered(triggers::THIS_DIES, take_hoard, "When this creature dies, you may put the exiled card into its owner's hand."),
    ],
    ..oracle::HOARDING_DRAGON
};
