use cantrip_api::*;
use cantrip_mechanics::{discard_at_random, targets};
use cantrip_oracle as oracle;

/// Target player discards a card at random (CR 701.9a, 608.2b).
fn discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    discard_at_random(g, player, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        discard,
        "{T}: Target player discards a card at random. Activate only during your turn.",
    )
    .targets(&[targets::PLAYER])
    .timing(Timing::OwnTurn)],
    ..oracle::GWENDLYN_DI_CORCI
};
