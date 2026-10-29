//! Learn (CR 701.48a): "'Learn' means 'You may discard a card. If you do, draw a card. If you didn't discard a card, you may
//! reveal a Lesson card you own from outside the game and put it into your hand.'" Academic Dispute prints it.

use cantrip_api::select::part;
use cantrip_api::*;

use crate::choose;
use crate::effects::may_discard_a_card;
use crate::select::{up_to, Part, Selection};

fn lesson(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| q.chars(card).has_subtype(Subtype::Lesson))
}

/// "A Lesson card" (the Lesson subtype, CR 205.3k): the quality learn reveals.
pub const LESSON: Part = part(&(lesson as SelectFn), 0, "");

/// "The number of Lesson cards in your graveyard" (CR 205.3k): the Lesson cards in `player`'s graveyard, as the caller asks.
/// Accumulate Wisdom, Combustion Technique and Gran-Gran print it.
pub fn lesson_cards_in_graveyard(q: &dyn Query, player: PlayerId) -> usize {
    crate::subtype_cards_in_graveyard(q, player, Subtype::Lesson)
}

/// One Lesson card or none.
pub const LESSON_CARD: Selection = Selection::of(up_to(1), &[LESSON]).called("Lesson");

/// Learn (CR 701.48a), as the resolving spell or ability of `item`: the controller may discard a card, and if they do, draws a
/// card; if they didn't, they may reveal a Lesson card they own from outside the game (CR 400.11b) and put it into their hand.
pub fn learn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if may_discard_a_card(g, item)? {
        g.draw(you, 1);
        return Ok(());
    }
    let outside = g.outside_the_game(you);
    if let Some(&card) = choose::select(g, you, item.object, Purpose::Reveal, outside, LESSON_CARD)?.first() {
        g.reveal(you, &[card]);
        g.bring_into_hand_revealed(&[card]);
    }
    Ok(())
}
