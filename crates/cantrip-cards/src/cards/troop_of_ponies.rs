use cantrip_api::*;
use cantrip_mechanics::{choose, library, select};
use cantrip_oracle as oracle;

/// "Search your library for up to two basic land cards, reveal them, put one onto the battlefield tapped and the other into your
/// hand, then shuffle." (CR 701.23, 701.20, 701.24): the player finds none, one or two. A lone card found goes onto the battlefield
/// tapped (the ruling); of two, the player says which one does, and the other goes into the hand, revealed. The library is shuffled
/// whether or not anything was found, unless the search could not be made (CR 101.2).
fn fetch_two_basic_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let found = library::search(g, you, you, item.object, SearchDestination::Found, select::basic_land_cards(select::up_to(2)))?;
    g.reveal(you, &found);
    let onto_battlefield = match found.len() {
        0 | 1 => found.clone(),
        _ => choose::select(g, you, item.object, Purpose::PutOntoBattlefield, found.clone(), select::cards(select::exactly(1)))?,
    };
    if let Some(&land) = onto_battlefield.first() {
        g.put_onto_battlefield(land, you, true)?;
    }
    let to_hand: Vec<(ObjectId, Zone)> = found.iter().filter(|c| !onto_battlefield.contains(c)).map(|&c| (c, Zone::hand(you))).collect();
    g.move_together_revealed(&to_hand, MoveReason::Search);
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}, {T}").and(SACRIFICE_THIS),
        fetch_two_basic_lands,
        "{2}, {T}, Sacrifice this creature: Search your library for up to two basic land cards, reveal them, put one onto the battlefield tapped and the other into your hand, then shuffle.",
    )],
    ..oracle::TROOP_OF_PONIES
};
