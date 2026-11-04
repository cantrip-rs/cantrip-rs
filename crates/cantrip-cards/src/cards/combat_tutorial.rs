use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target player draws two cards. Put a +1/+1 counter on up to one target creature you control." Each part resolves on its own
/// if its target is still legal (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 2);
    if let Some(creature) = targets::object(g, item, 1) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::PLAYER, targets::CREATURE_YOU_CONTROL.optional()], effect), ..oracle::COMBAT_TUTORIAL };
