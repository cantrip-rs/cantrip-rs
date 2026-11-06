use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// "Target player chooses three cards from their hand and puts them on top of their library in any order." (CR 608.2c, 401.4).
fn put_three_on_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let cards = choose::select(g, player, item.object, Purpose::PutOnTop, hand, select::cards(select::as_many_as_can(3)))?;
    let order: Vec<ObjectId> = cards.into_iter().rev().collect();
    g.put_into_library(&order, LibraryPlace::Top);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], put_three_on_top),
    ..oracle::STUNTED_GROWTH
};
