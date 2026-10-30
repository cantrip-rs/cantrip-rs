//! Fight (CR 701.14), the keyword action, for the spells and abilities that name two targets.

use cantrip_api::*;

use crate::targets;

/// "Target creature fights target creature" (CR 701.14a): the creatures `item` targeted in `first` and `second` each deal
/// damage equal to their power to the other, at once (not combat damage, CR 701.14d). If either target is an illegal
/// target as the spell or ability resolves, neither fights or deals damage (CR 701.14b); the verb itself checks that both
/// are still creatures on the battlefield. Bushwhack prints it; Mawloc and Wolverine, Fierce Fighter, whose one target is
/// fought by the source itself, call `Game::fight` directly.
pub fn targets_fight(g: &mut dyn Game, item: &StackItem, first: usize, second: usize) {
    if let (Some(a), Some(b)) = (targets::object(g, item, first), targets::object(g, item, second)) {
        g.fight(a, b);
    }
}

/// "When this Aura enters, enchanted creature fights up to one target creature an opponent controls" (CR 701.14): the ability's
/// first target is the one fought, and none or one gone means nothing fights. The enchanted creature is the one the Aura is attached
/// to as the ability resolves, or, if the Aura has left the battlefield by then, the one it was attached to as it left (CR 608.2h);
/// the fight itself does nothing unless both are creatures on the battlefield (CR 701.14b). Meltstrider's Resolve and Pitiless Fists
/// print it.
pub fn enchanted_creature_fights_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(prey) = targets::object(g, item, 0) else { return Ok(()) };
    if let Some(enchanted) = g.obj(item.object).attached_to {
        g.fight(enchanted, prey);
    }
    Ok(())
}

/// "[Creature] deals damage equal to its power to [target]" (CR 120.3, 609.7, 120.8): one-sided, the creature being the
/// source of the damage, so its abilities (lifelink, deathtouch) and colors count. Its power is read now, as the spell or
/// ability resolves (CR 608.2c); a creature with no power deals none. Hard-Hitting Question and Archdruid's Charm print it.
pub fn bite(g: &mut dyn Game, creature: ObjectId, victim: ObjectId) {
    let power = g.chars(creature).power.unwrap_or(0);
    if power > 0 {
        g.damage(creature, Target::Object(victim), power as u16);
    }
}

/// "Target creature you control deals damage equal to its power to target [creature]" (CR 120.3, 609.7, 608.2b): the creature
/// `item` targeted in `source_slot` bites the object targeted in `victim_slot` (`bite`). Each target is judged as the spell resolves:
/// if either is illegal then, no damage is dealt, a creature that is not a legal target being no source and nothing left to deal
/// it to. Compel Brutality's first mode, Hard-Hitting Question and Quarrel print it.
pub fn targets_bite(g: &mut dyn Game, item: &StackItem, source_slot: usize, victim_slot: usize) {
    if let (Some(creature), Some(victim)) = (targets::object(g, item, source_slot), targets::object(g, item, victim_slot)) {
        bite(g, creature, victim);
    }
}
