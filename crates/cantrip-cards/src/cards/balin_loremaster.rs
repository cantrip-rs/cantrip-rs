use cantrip_api::*;
use cantrip_mechanics::{effects, lands, storied, triggers};
use cantrip_oracle as oracle;

const ASK: Question =
    Question::new("DiscardHandToDraw", "discard your hand?", "Then draw a card for each card discarded", "Discard hand", "Keep hand").no_default();

/// "You may discard your hand. Draw X cards, where X is the number of cards discarded this way. If you have an enduring
/// story, Balin deals X damage to each opponent." An empty hand may be discarded too (the rulings), and X counts the cards
/// that were discarded (CR 701.9). Whether you have an enduring story is asked as the ability resolves; the damage is
/// Balin's, by last-known information if it has left (CR 113.7a).
fn wheel(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if !effects::ask_may(g, you, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let x = g.discard_hand(you).len();
    g.draw(you, x as u32);
    if storied::has_an_enduring_story(g.state(), you) {
        let hits: Vec<(ObjectId, Target, i64)> = g.state().opponents(you).map(|p| (item.object, Target::Player(p), i64::from(x as u16))).collect();
        g.damage_together(&hits);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    storied: true,
    abilities: &[triggered(when_asked(EventMask::ZONE_CHANGE, triggers::this_or_another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Dwarf) }>), wheel, "Whenever Balin or another Dwarf you control enters, you may discard your hand. Draw X cards, where X is the number of cards discarded this way. If you have an enduring story, Balin deals X damage to each opponent.")],
    ..oracle::BALIN_LOREMASTER
};
