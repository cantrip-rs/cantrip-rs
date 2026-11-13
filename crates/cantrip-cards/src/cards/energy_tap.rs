use cantrip_api::*;
use cantrip_mechanics::{targets, untapped_creature_you_control};
use cantrip_oracle as oracle;

/// An untapped creature controlled by the spell's controller (CR 115.1a).
fn target_creature(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    matches!(target, Target::Object(id) if targets::permanent(q, id) && untapped_creature_you_control(q, asker, id))
}

/// Tap the creature, then add colorless mana equal to its mana value (CR 701.26a, 106.3).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    // A legal target is untapped, so tapping it succeeds (CR 701.26a).
    g.tap(creature);
    g.end_sentence();
    g.add_mana(item.controller, Mana::C, g.mana_value(creature));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(target_creature)], resolve),
    ..oracle::ENERGY_TAP
};
