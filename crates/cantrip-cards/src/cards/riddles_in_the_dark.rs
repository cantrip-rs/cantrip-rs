use cantrip_api::*;
use cantrip_mechanics::{choose, top_of_library};
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library and separate them into a face-down pile and a face-up pile. An opponent chooses
/// one of the piles. Put that pile into your hand and the other into your graveyard." (CR 700.3, 608.2c-d): the caster looks at
/// the cards (a library with fewer shows what it has, CR 609.3) and chooses the face-down pile, the rest being the face-up
/// pile, either of them possibly empty (the ruling); the opponent, chosen as it resolves and not targeted (CR 115.10a), sees the
/// face-up pile's cards and only how many the face-down pile holds, and chooses a pile even if it is empty. The cards are not
/// revealed, so the face-down pile need not be shown when it goes into the hand. The piles go to hand, then graveyard.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let top = top_of_library(g, you, 4);
    if top.is_empty() {
        return Ok(());
    }
    let (face_down, face_up) = choose::separate_face_down_and_face_up(g, you, item.object, top)?;
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let verb = "decides which of the two piles you put into your hand; the other goes into your graveyard";
    let Some(chooser) = choose::player(g, you, item.object, opponents, verb, 0)? else { return Ok(()) };
    let (keep, rest) = choose::face_down_or_face_up_pile(g, chooser, item.object, face_down, face_up)?;
    g.return_to_hand(&keep);
    g.put_into_graveyard(&rest);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::RIDDLES_IN_THE_DARK };
