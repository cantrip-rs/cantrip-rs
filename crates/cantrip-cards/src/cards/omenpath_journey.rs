use cantrip_api::*;
use cantrip_mechanics::{library, select, triggers};
use cantrip_oracle as oracle;

/// "Search your library for up to five land cards that have different names, exile them, then shuffle" (CR 701.23, 701.24, 201.2b):
/// the lands found are exiled together, face up, and linked to this ability's source (CR 607.2a) for the end step ability to find.
/// A search that finds none still shuffles.
fn search_for_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let rule = select::land_cards(select::up_to(5)).with(select::DIFFERENT_NAMES);
    let found = library::search(g, player, player, item.object, SearchDestination::Exile, rule)?;
    g.exile_with(&found, g.linked(item), false);
    g.shuffle(player);
    Ok(())
}

/// "Choose a card at random exiled with this enchantment and put it onto the battlefield tapped" (CR 607.2a, 608.2d): one of the
/// cards its first ability exiled that is still in exile as this resolves; nothing when there are none.
fn put_a_random_one_onto_the_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    if let Some(card) = library::random_of(g, &exiled) {
        g.put_onto_battlefield(card, item.controller, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            search_for_lands,
            "When this enchantment enters, search your library for up to five land cards that have different names, exile them, then shuffle.",
        ),
        triggered(
            triggers::YOUR_END_STEP,
            put_a_random_one_onto_the_battlefield,
            "At the beginning of your end step, choose a card at random exiled with this enchantment and put it onto the battlefield tapped.",
        ),
    ],
    ..oracle::OMENPATH_JOURNEY
};
