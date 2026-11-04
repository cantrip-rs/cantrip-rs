use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets, top_of_library};
use cantrip_oracle as oracle;

/// Look at and reorder the targeted player's top three cards (CR 608.2b, 608.2c).
fn look_and_reorder(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let cards = top_of_library(g, player, 3);
    g.look_at(item.controller, &cards);
    // Choose bottom first, so the last remaining card ends up on top.
    let mut remaining = cards;
    let mut order = Vec::new();
    while let [_, _, ..] = remaining.as_slice() {
        let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, remaining.clone(), select::CARD.taking(select::exactly(1)).called("card to place lowest among the remaining cards"))?;
        let card = chosen[0];
        remaining.retain(|&id| id != card);
        order.push(card);
    }
    order.extend(remaining);
    order.reverse();
    g.arrange_library(player, &order, &[]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}"),
        look_and_reorder,
        "{3}: Look at the top three cards of target player's library, then put them back in any order.",
    ).targets(&[targets::PLAYER])],
    ..oracle::ELEMENTAL_AUGURY
};
