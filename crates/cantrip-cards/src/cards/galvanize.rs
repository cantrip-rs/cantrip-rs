use cantrip_api::*;
use cantrip_mechanics::history::drawn_this_turn;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Galvanize deals 3 damage to target creature. If you've drawn two or more cards this turn, Galvanize deals 5 damage to
/// that creature instead": one damage event, the amount decided as it resolves.
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = if drawn_this_turn(g, item.controller) >= 2 { 5 } else { 3 };
    damage_target(g, item, 0, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], damage), ..oracle::GALVANIZE };
