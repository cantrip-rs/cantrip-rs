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

const THIS_MAY_FIGHT: Question = Question::new("Fight", "have {source} fight that creature?", "Chosen as the ability resolves", "Fight", "Don't");

/// "You may have [this creature] fight [that creature]" (CR 701.14a-b, 608.2d): the source's controller chooses as the ability
/// resolves, and only on a yes do the source and `prey` fight; the fight does nothing unless both are still creatures on the
/// battlefield (CR 701.14b). Affectionate Indrik and Kraul Harpooner print it.
pub fn may_have_this_fight(g: &mut dyn Game, item: &StackItem, prey: ObjectId) -> Result<(), Suspend> {
    if crate::effects::ask_may(g, item.controller, item.object, Purpose::Ask(&THIS_MAY_FIGHT))? {
        g.fight(item.object, prey);
    }
    Ok(())
}

/// "Put `N` +1/+1 counters on target creature you control. [Then] it fights target creature an opponent controls." (CR 122.1,
/// 701.14, 608.2b-c): the creature is the spell's first target and the one fought its second, each judged as the spell resolves.
/// With the first gone nothing is put on and nothing fights; with only the second gone the counters are still put on and no
/// fight happens. The counters come first, so the fight is with the power they give (CR 608.2c). Troll Negotiations (two) and
/// Kapow! (one) print it.
pub fn plus_one_counters_on_first_target_then_it_fights_second<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, N);
    }
    g.end_sentence();
    targets_fight(g, item, 0, 1);
    Ok(())
}

/// "Put `N` +1/+1 counters on target creature you control. [Then] it deals damage equal to its power to target creature an opponent
/// controls." (CR 122.1, 120.3, 608.2b-c): the creature is the spell's first target and the one hit its second, each judged as the
/// spell resolves. With the first gone nothing is put on and nothing is dealt; with only the second gone the counters are still put
/// on. The counters come first, so the damage is the power they give (CR 608.2c). Knockout Maneuver and Felling Blow print it.
pub fn plus_one_counters_on_first_target_then_it_bites_second<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, N);
    }
    g.end_sentence();
    targets_bite(g, item, 0, 1);
    Ok(())
}

/// "Enchanted creature fights up to one target creature an opponent controls" / "equipped creature fights ..." (CR 701.14), the
/// ability's first target being the one fought. The enchanted or equipped creature is the one the source is attached to as the
/// ability resolves, or, if the source has left the battlefield by then, the one it was attached to as it left (CR 608.2h). The
/// fight does nothing unless both are creatures on the battlefield (CR 701.14b); with no target chosen, or the target gone,
/// there is nothing to fight. Meltstrider's Resolve and Novel Nunchaku print it.
pub fn attached_creature_fights_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(prey) = targets::object(g, item, 0) else { return Ok(()) };
    if let Some(attached) = g.obj(item.object).attached_to {
        g.fight(attached, prey);
    }
    Ok(())
}

/// "Target creature you control gets +P/+T until end of turn. Then it fights up to one target creature [an opponent controls / you don't
/// control]" (CR 611.2a, 608.2c, 701.14): the creature is the spell's first target and the one fought its second, each judged as the
/// spell resolves (CR 608.2b). The pump comes first and ends a sentence, so the fight is with the power it gives; with no second
/// target, or a gone one, only the pump happens, and with the first gone nothing does (CR 701.14b). Chelonian Tackle (+0/+10) and
/// Primal Might (+X/+X) print it.
pub fn pump_first_target_then_it_fights_second(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32) {
    if let Some(creature) = targets::object(g, item, 0) {
        crate::pump(g, item.object, creature, power, toughness);
    }
    g.end_sentence();
    targets_fight(g, item, 0, 1);
}

/// "Target creature you control gets +P/+T until end of turn. Then it deals damage equal to its power to [up to one] target creature [an
/// opponent controls / you don't control]" (CR 611.2a, 608.2c, 120.3): the creature is the spell's first target and the victim its second,
/// each judged as the spell resolves (CR 608.2b). The pump comes first and ends a sentence, so the damage is the power it gives; with
/// no second target, or a gone one, only the pump happens, and with the first gone nothing does. Assert Perfection (+1/+0) and
/// Rabid Gnaw print it.
pub fn pump_first_target_then_it_bites_second(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32) {
    if let Some(creature) = targets::object(g, item, 0) {
        crate::pump(g, item.object, creature, power, toughness);
    }
    g.end_sentence();
    targets_bite(g, item, 0, 1);
}

/// "[Creature] deals damage equal to its power to [target]" (CR 120.3, 609.7, 120.8): one-sided, the creature being the
/// source of the damage, so its abilities (lifelink, deathtouch) and colors count. Its power is read now, as the spell or
/// ability resolves (CR 608.2c); a creature with no power deals none. Hard-Hitting Question and Archdruid's Charm print it.
pub fn bite(g: &mut dyn Game, creature: ObjectId, victim: ObjectId) {
    let power = damage_equal_to_power(g, creature);
    if power > 0 {
        g.damage(creature, Target::Object(victim), power);
    }
}

/// "Damage equal to its power" (CR 120.3, 608.2h): the power `creature` has now, as the spell or ability resolves, as an amount
/// of damage - none for a creature with no power or a negative one (CR 120.8). `bite`, Chocobo Kick, Self-Destruct, Blazing
/// Bomb and Iron Fist, Living Weapon use it.
pub fn damage_equal_to_power(q: &dyn Query, creature: ObjectId) -> u32 {
    q.chars(creature).power.unwrap_or(0).max(0) as u32
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

/// "Target creature you control deals damage equal to its power to target [creature or planeswalker]" as a whole effect
/// (CR 120.3, 609.7, 608.2b): `targets_bite` from the first target to the second. Giantfall, Hard-Hitting Question, Quarrel,
/// Tenderize, Compel Brutality and Vivien, Arkbow Ranger print it.
pub fn first_target_bites_second(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    targets_bite(g, item, 0, 1);
    Ok(())
}

/// "One or two target creatures you control each deal damage equal to their power to target creature an opponent controls"
/// (CR 120.2, 120.3, 120.4b, 608.2b): the targets before the last are the sources, the last the recipient. The powers are read as
/// this resolves and all the damage is dealt together; a source that is no longer a legal target deals none, and with the
/// recipient illegal nothing is dealt. Common Goal and Terrific Team-Up print it.
pub fn targets_each_bite_last(g: &mut dyn Game, item: &StackItem) {
    let recipient_slot = item.target_count as usize - 1;
    let Some(recipient) = targets::object(g, item, recipient_slot) else { return };
    let hits: Vec<(ObjectId, Target, u32)> = (0..recipient_slot)
        .filter_map(|slot| {
            let source = targets::object(g, item, slot)?;
            let power = g.chars(source).power.unwrap_or(0).max(0) as u32;
            Some((source, Target::Object(recipient), power))
        })
        .collect();
    g.damage_together(&hits);
}

/// "Target creature you control deals damage equal to twice its power to target [creature]" as a whole effect (CR 120.3, 609.7,
/// 608.2b): `first_target_bites_second` with the damage doubled. The power is read as the spell resolves (CR 608.2c), a creature
/// with none (or less) dealing none (CR 120.8); twice a positive 32-bit power fits an unsigned one. If either target is illegal
/// then, no damage is dealt. Punishing Punch and Polliwallop print it.
pub fn first_target_bites_second_for_twice_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(creature), Some(victim)) = (targets::object(g, item, 0), targets::object(g, item, 1)) {
        let power = damage_equal_to_power(g, creature);
        if power > 0 {
            g.damage(creature, Target::Object(victim), 2 * power);
        }
    }
    Ok(())
}

/// "[Creature] deals damage equal to its power to each other creature" (CR 120.3, 608.2h, 120.8): the creature is the source (so its
/// abilities and colors count), its power is the one it has as the spell resolves, and every other creature on the battlefield, whoever
/// controls it, is dealt that much at once; a creature with no power deals none. Nibelheim Aflame and Waltz of Rage print it.
pub fn deals_damage_equal_to_its_power_to_each_other_creature(g: &mut dyn Game, creature: ObjectId) {
    let power = damage_equal_to_power(g, creature);
    let hits: Vec<(ObjectId, Target, u32)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != creature && g.chars(id).is(CardType::Creature))
        .map(|id| (creature, Target::Object(id), power))
        .collect();
    g.damage_together(&hits);
}
