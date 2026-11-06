use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature or enchantment an opponent controls" (CR 115.1).
fn creature_or_enchantment_an_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_or_enchantment(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

/// "Destroy target creature or enchantment an opponent controls. You lose life equal to that permanent's mana value."
/// The mana value is the permanent's as it last existed on the battlefield (X is 0 there, CR 202.3e), which is what it is
/// the moment before it is destroyed. The life is lost whether or not it was destroyed (an indestructible target is still
/// a legal target that resolves); with no legal target the spell does not resolve and nothing is lost (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let value = g.mana_value(target) as i32;
    g.destroy(target);
    if value > 0 {
        g.lose_life(item.controller,(value) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_or_enchantment_an_opponent_controls)], effect),
    ..oracle::FEED_THE_SWARM
};
