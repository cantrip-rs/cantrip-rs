use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Target player reveals their hand and discards all nonland cards (CR 701.20a, 701.9a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    g.reveal(player, &hand);
    let nonlands: Vec<_> = hand.into_iter().filter(|&id| !g.chars(id).is(CardType::Land)).collect();
    g.discard_revealed(&nonlands);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effect),
    ..oracle::AMNESIA
};
