use cantrip_api::*;
use cantrip_mechanics::{choose, library, select};
use cantrip_oracle as oracle;

/// "Search your library for a creature card, reveal it, put it into your hand or graveyard, then shuffle" (CR 701.23, 701.20):
/// the card is found and revealed, then its hand-or-graveyard destination is chosen (nothing is asked when none was found), and
/// the library is shuffled whether or not a card was found, or the search could be made at all.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let found = library::search(g, you, you, item.object, SearchDestination::Hand, select::CREATURE_CARD)?;
    g.reveal(you, &found);
    if !found.is_empty() {
        let card = g.obj(item.object).card;
        let to = if choose::one_of(g, you, item.object, vec![Label::Card(card, 0), Label::Card(card, 1)])? == 0 { Zone::hand(you) } else { Zone::graveyard(you) };
        let moves: Vec<(ObjectId, Zone)> = found.iter().map(|&c| (c, to)).collect();
        g.move_together_revealed(&moves, MoveReason::Search);
    }
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    labels: &["Put it into your hand", "Put it into your graveyard"],
    spell: spell(&[], search),
    ..oracle::DINAS_GUIDANCE
};
