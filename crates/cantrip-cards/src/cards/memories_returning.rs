use cantrip_api::*;
use cantrip_mechanics::{choose, flashback, select, top_of_library};
use cantrip_oracle as oracle;

/// "You put one into your hand": one of the cards still revealed and in the library, chosen by their owner and returned to
/// the hand still revealed (CR 701.20a).
fn take_one(g: &mut dyn Game, you: PlayerId, source: ObjectId, left: &mut Vec<ObjectId>) -> Result<(), Suspend> {
    if let Some(card) = choose::select(g, you, source, Purpose::PutIntoHand, left.clone(), select::cards(select::exactly(1)))?.first().copied() {
        left.retain(|&c| c != card);
        g.return_to_hand_revealed(&[card]);
    }
    Ok(())
}

/// "They put one on the bottom of your library": one of the cards left, chosen by the opponent, who sees them all, and put on
/// the bottom of the controller's library, below everything there (CR 401.4). Nothing happens with no opponent chosen.
fn bury_one(g: &mut dyn Game, opponent: Option<PlayerId>, you: PlayerId, source: ObjectId, left: &mut Vec<ObjectId>) -> Result<(), Suspend> {
    let Some(opponent) = opponent else { return Ok(()) };
    if let Some(card) = choose::select(g, opponent, source, Purpose::PutOnBottom, left.clone(), select::cards(select::exactly(1)))?.first().copied() {
        left.retain(|&c| c != card);
        g.arrange_library(you, &[], &[card]);
    }
    Ok(())
}

/// "Reveal the top five cards of your library. Put one of them into your hand. Then choose an opponent. They put one on the
/// bottom of your library. Then you put one into your hand. Then they put one on the bottom of your library. Put the other
/// into your hand." Each instruction in the order written (CR 608.2c): the cards are revealed to every player (CR 701.20a),
/// the opponent is chosen without targeting (CR 608.2d, 115.10a) as the third instruction is followed, and each choice is
/// made among the cards still left. A library with fewer than five cards reveals what it has (CR 609.3), and an
/// instruction with no card left to follow does nothing.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let mut left = top_of_library(g, you, 5);
    if left.is_empty() {
        return Ok(());
    }
    g.reveal(you, &left);
    take_one(g, you, item.object, &mut left)?;
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let opponent = choose::player(g, you, item.object, opponents, "PutOnBottomOfYourLibrary", 2)?;
    bury_one(g, opponent, you, item.object, &mut left)?;
    take_one(g, you, item.object, &mut left)?;
    bury_one(g, opponent, you, item.object, &mut left)?;
    g.return_to_hand_revealed(&left);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{7}{U}{U}"), "Flashback {7}{U}{U}")],
    ..oracle::MEMORIES_RETURNING
};
