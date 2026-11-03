use cantrip_api::*;
use cantrip_mechanics::{hand, targets};
use cantrip_oracle as oracle;

/// Look at target player's hand; draw a card. The look is recorded card by card (`Game::look_at`, `Event::Looked`), so
/// what its caster has seen is known to the history and the view.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::look_at_target_players_hand(g, item)?;
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effect),
    ..oracle::GITAXIAN_PROBE
};
