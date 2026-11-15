use cantrip_api::*;
use cantrip_mechanics::{choose, top_of_library, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Reveal the top ten cards of your library. For each card type, you may put a card of that type from among the
/// revealed cards into your hand. Put the rest on the bottom of your library in a random order."
///
/// One question per card type (`choose::one_of_each_card_type`).
fn unify(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 10);
    if looked.is_empty() {
        return Ok(());
    }
    g.reveal(you, &looked);

    let taken = choose::one_of_each_card_type(g, you, item.object, Purpose::PutIntoHand, &looked)?;

    // What goes to hand leaves first; the rest are rearranged in the library, where they stay (nothing changes zones).
    let rest: Vec<ObjectId> = looked.iter().copied().filter(|c| !taken.contains(c)).collect();
    g.return_to_hand_revealed(&taken);
    g.bottom_in_random_order(you, &rest);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, unify, "When Atraxa enters, reveal the top ten cards of your library. For each card type, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.")],
    ..oracle::ATRAXA_GRAND_UNIFIER
};
