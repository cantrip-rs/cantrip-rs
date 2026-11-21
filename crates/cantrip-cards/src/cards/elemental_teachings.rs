use cantrip_api::*;
use cantrip_mechanics::select::{land_cards, up_to, DIFFERENT_NAMES};
use cantrip_mechanics::{choose, library};
use cantrip_oracle as oracle;

/// "Search your library for up to four land cards with different names and reveal them. An opponent chooses two of those cards. Put the
/// chosen cards into your graveyard and the rest onto the battlefield tapped, then shuffle." (CR 701.23, 701.20, 608.2d, 701.24a).
///
/// With one or two cards found the opponent has no real choice: all of them are chosen and go to the graveyard (the ruling of 2025-10-02).
/// The shuffle happens even when nothing was found, as the search was made.
fn search_reveal_and_split(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let found = library::search(g, you, you, item.object, SearchDestination::Found, land_cards(up_to(4)).with(DIFFERENT_NAMES))?;
    g.reveal(you, &found);
    let chosen = choose::opponent_chooses(g, you, item.object, Purpose::PutIntoGraveyard, found.clone(), 2)?;
    g.put_into_graveyard(&chosen);
    let rest: Vec<(ObjectId, PlayerId, bool)> = found.into_iter().filter(|card| !chosen.contains(card)).map(|card| (card, you, true)).collect();
    g.put_all_onto_battlefield_as(&rest)?;
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], search_reveal_and_split), ..oracle::ELEMENTAL_TEACHINGS };
