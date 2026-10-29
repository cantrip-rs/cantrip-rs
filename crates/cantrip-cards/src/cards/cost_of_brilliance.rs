use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target player draws two cards and loses 2 life. Put a +1/+1 counter on up to one target creature." Each target is
/// looked at as this resolves (CR 608.2b): a player who is gone is drawn and drained of nothing, a creature that is no longer
/// legal gets no counter, and only when every target has gone does the spell not resolve.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.draw(player, 2);
        g.lose_life(player, 2);
    }
    if let Some(creature) = targets::object(g, item, 1) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER, targets::CREATURE.optional()], effect),
    ..oracle::COST_OF_BRILLIANCE
};
