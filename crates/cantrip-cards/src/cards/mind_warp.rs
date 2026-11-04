use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// Look at the player's hand, choose as many as X cards as possible, then that player discards them (CR 608.2d, 701.9a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    g.look_at(item.controller, &hand);
    let chosen = choose::select(
        g,
        item.controller,
        item.object,
        Purpose::TakeFromHand,
        hand,
        select::cards(select::as_many_as_can(u32::from(item.x))),
    )?;
    g.discard_together(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effect),
    ..oracle::MIND_WARP
};
