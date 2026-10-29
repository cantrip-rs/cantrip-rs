use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::library::searched_cards;
use cantrip_api::select::{part, Part, SelectFn};
use cantrip_mechanics::select::{creature_cards, up_to, DIFFERENT_NAMES};
use cantrip_oracle as oracle;

fn legendary(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| q.chars(card).supertypes.legendary)
}

const LEGENDARY: Part = part(&(legendary as SelectFn), 0, "legendary");

/// "Search your library and/or outside the game for exactly four legendary creature cards you own with different names
/// (CR 701.23, 701.23j), then reveal those cards. An opponent chooses two of them. Put the chosen cards into your hand and
/// shuffle the rest into your library."
///
/// The library and the cards owned outside the game are searched together, one selection over both. The two the opponent
/// chooses come into the hand, and the rest go into the library, which is shuffled (also when nothing was found, as the
/// search was made). A library card moves to the hand; one from outside the game comes in from there (CR 400.11b).
fn search_reveal_and_split(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let library = if g.may_search(you, you) { searched_cards(g, you) } else { Vec::new() };
    let outside = g.outside_the_game(you);
    let rule = creature_cards(up_to(4)).with(LEGENDARY).with(DIFFERENT_NAMES);
    let shown: Vec<ObjectId> = outside.iter().copied().chain(library).collect();
    let request = RequestKind::SearchLibrary { source: item.object, library: shown, rule: Box::new(rule), destination: SearchDestination::Hand };
    let found = if let Answer::Cards(cards) = g.ask(you, request)? { cards } else { Vec::new() };
    g.searched(you, you);
    g.reveal(you, &found);
    let taken = choose::opponent_chooses(g, you, item.object, found.clone(), 2)?;
    let (from_outside, from_library): (Vec<ObjectId>, Vec<ObjectId>) = taken.iter().partition(|&&card| outside.contains(&card));
    g.bring_into_hand_revealed(&from_outside);
    let moves: Vec<(ObjectId, Zone)> = from_library.iter().map(|&card| (card, Zone::hand(you))).collect();
    g.move_together_revealed(&moves, MoveReason::Search);
    g.end_sentence();
    let rest: Vec<ObjectId> = found.into_iter().filter(|card| !taken.contains(card)).collect();
    // The rest were revealed, so they go back as cards all players saw (CR 603.10a), and the library is then shuffled.
    let moves: Vec<(ObjectId, Zone)> = rest.iter().map(|&card| (card, Zone::library(you))).collect();
    g.move_together_revealed(&moves, MoveReason::Search);
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_reveal_and_split),
    ..oracle::TURTLES_FOREVER
};
