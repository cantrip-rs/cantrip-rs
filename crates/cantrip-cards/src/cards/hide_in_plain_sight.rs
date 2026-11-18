use cantrip_api::*;
use cantrip_mechanics::{choose, cloak, library, select, top_of_library};
use cantrip_oracle as oracle;

/// "Look at the top five cards of your library, cloak two of them, and put the rest on the bottom of your library in a random
/// order." The cards are looked at (CR 701.20a), the player picks two (all of them if the library has fewer than two, CR 609.3),
/// those are cloaked one at a time (CR 701.58e), and the rest go to the bottom in a random order (CR 401.4). A card that did not
/// enter face down (CR 701.58f) is simply not cloaked.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 5);
    if looked.is_empty() {
        return Ok(());
    }
    g.look_at(you, &looked);
    let chosen = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, looked.clone(), select::cards(select::as_many_as_can(2)))?;
    cloak::cloak(g, you, &chosen, false)?;
    library::rest_to_bottom_in_random_order(g, you, &looked);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ..oracle::HIDE_IN_PLAIN_SIGHT
};
