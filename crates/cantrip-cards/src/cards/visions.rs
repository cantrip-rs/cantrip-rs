use cantrip_api::*;
use cantrip_mechanics::{effects, targets, top_of_library};
use cantrip_oracle as oracle;

const SHUFFLE: Question = Question::new(
    "ShuffleLibrary",
    "have that player shuffle their library?",
    "After looking at the top five cards",
    "Shuffle",
    "Don't shuffle",
).no_default();

/// "Look at the top five cards of target player's library. You may then have that player shuffle that library." (CR 608.2h, 701.24a).
fn look_and_may_shuffle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let cards = top_of_library(g, player, 5);
    g.look_at(item.controller, &cards);
    g.end_sentence();
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&SHUFFLE))? {
        g.shuffle(player);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], look_and_may_shuffle),
    ..oracle::VISIONS
};
