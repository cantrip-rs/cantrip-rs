use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target player draws 2^X cards" (CR 107.3, 121.1): X as the spell was cast. From X of 32 the count is more cards than
/// any library holds (a library of 2^32 cards is out of play's reach), so it is the most one instruction counts: the
/// library is emptied and the draw from it that follows loses the game (CR 121.4) exactly as the full count would. With its
/// target gone the spell does not resolve (CR 608.2b).
fn draw_two_to_the_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 1u32.checked_shl(item.x as u32).unwrap_or(u32::MAX));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], draw_two_to_the_x),
    ..oracle::MATHEMAGICS
};
