//! Class levels (CR 716): "{cost}: Level N" is an activated ability, activated only as a sorcery and only while the Class
//! is level N-1 (CR 716.2b); when it resolves the Class becomes level N (CR 716.3). The level is a designation, never a
//! counter, and a Class that has none is level 1 (CR 716.2a). Does Machines, Innkeeper's Talent, Stormchaser's Talent and
//! Caretaker's Talent print it.

use cantrip_api::*;

/// The level a Class is at: 1 until a level ability has resolved (CR 716.2a).
pub fn level(q: &dyn Query, source: ObjectId) -> u8 {
    q.obj(source).class_level.max(1)
}

fn at_level_one(q: &dyn Query, asker: &Asker) -> bool {
    level(q, asker.source) == 1
}

fn at_level_two(q: &dyn Query, asker: &Asker) -> bool {
    level(q, asker.source) == 2
}

fn become_level_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_class_level(item.object, 2);
    Ok(())
}

fn become_level_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_class_level(item.object, 3);
    Ok(())
}

/// "{cost}: Level 2" (CR 716.2b): only as a sorcery, only while the Class is level 1.
pub const fn level_two(mana: &str, text: &'static str) -> AbilityDef {
    ability(cost(mana), become_level_two, text).sorcery_speed().condition(at_level_one)
}

/// "{cost}: Level 3" (CR 716.2b): only as a sorcery, only while the Class is level 2.
pub const fn level_three(mana: &str, text: &'static str) -> AbilityDef {
    ability(cost(mana), become_level_three, text).sorcery_speed().condition(at_level_two)
}

fn became_level_two(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ClassLevel { object, level: 2 } if *object == source => Fired::PLAIN,
        _ => None,
    }
}

/// "When this Class becomes level 2" (CR 716.4).
pub const BECOMES_LEVEL_TWO: Trigger = when(EventMask::CLASS_LEVEL, became_level_two);

fn became_level_three(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ClassLevel { object, level: 3 } if *object == source => Fired::PLAIN,
        _ => None,
    }
}

/// "When this Class becomes level 3" (CR 716.4). Cool but Rude and Builder's Talent print it.
pub const BECOMES_LEVEL_THREE: Trigger = when(EventMask::CLASS_LEVEL, became_level_three);
