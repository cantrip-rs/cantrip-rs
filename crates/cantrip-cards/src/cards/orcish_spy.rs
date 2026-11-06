use cantrip_api::*;
use cantrip_mechanics::{targets, top_of_library};
use cantrip_oracle as oracle;

/// "{T}: Look at the top three cards of target player's library." (CR 608.2b).
fn look_top_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let cards = top_of_library(g, player, 3);
    g.look_at(item.controller, &cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, look_top_three, "{T}: Look at the top three cards of target player's library.")
        .targets(&[targets::PLAYER])],
    ..oracle::ORCISH_SPY
};
