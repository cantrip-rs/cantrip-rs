use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Deal damage equal to the target player's hand size as this resolves (CR 608.2h, 120.3).
fn damage_for_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let amount = effects::damage_for_cards_in_hand(g, player);
    g.damage_together(&[(item.object, Target::Player(player), amount as u32)]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], damage_for_hand),
    ..oracle::STORM_SEEKER
};
