use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::tokens::{self, FIREBENDING_SOLDIER};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever a creature you control attacking causes a triggered ability of that creature to trigger" (CR 508.1m, 603.2, 603.3b): an
/// ability of an attacking creature you control set off by its being declared as an attacker, "whenever this creature attacks" or
/// "whenever you attack" alike (as `statics::creature_attacking_triggers_again` reads Isshin's rule, the rulings), once it is put on
/// the stack, about it there (to copy it, CR 707.10). Its "whenever this creature attacks" ability set off by another creature's
/// attack, or one the declaration did not set off (a delayed or reflexive one, one set off by a death or a counter), is none of
/// these; nor is a "whenever you attack" ability of a creature that did not attack. A "whenever you attack" ability is put on the
/// stack as attackers have just been declared (CR 508.2b), so the creature attacking then was declared in that declaration. One
/// removed from the stack for want of a legal target (CR 603.3d) still triggered: a counter, and nothing to copy.
fn your_attacking_creature_causes_its_ability_to_trigger(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::TriggerOnStack { source, serial, cause: Some(cause), .. } = *event else { return None };
    let attacking = cause.kind == EventMask::ATTACKS.bit() && cause.about_source || cause.kind == EventMask::ATTACKERS_DECLARED.bit();
    if !attacking || q.controller(source) != me.controller || !q.is_attacking(source) {
        return None;
    }
    match serial {
        Some(serial) => Fired::about(Target::Ability(serial)),
        None => Fired::PLAIN,
    }
}

/// "Create a 2/2 red Soldier creature token with firebending 1" (CR 111.1, 701.7).
fn create_a_firebending_soldier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &FIREBENDING_SOLDIER, 1)?;
    Ok(())
}

const ASK: Question = Question::new("CopyAbility", "copy that triggered ability?", "You may choose new targets for the copy", "Copy it", "Don't copy");

/// "Put a quest counter on this enchantment. Then if it has four or more quest counters on it, you may copy that ability. You may
/// choose new targets for the copy." (CR 707.10, 707.10c) Not an intervening "if" (CR 603.4): the count is read once the counter
/// has been put on, so the fourth counter and every one after it may copy, and an enchantment that has left the battlefield gets no
/// counter (CR 122.6) and is read as it last was (CR 608.2h). The ability is the one this one is about, copied even if it has left
/// the stack (as it last was there); the copy is put on the stack above it and resolves first. One removed as it was put on the
/// stack (CR 603.3d) is nothing to copy, and the copy is not offered.
fn quest_counter_then_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Quest, 1);
    if g.obj(item.object).counters(CounterKind::Quest) >= 4 && item.subject != Target::None && ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.copy_ability(item.subject, item.controller, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_a_firebending_soldier, "When this enchantment enters, create a 2/2 red Soldier creature token with firebending 1."),
        triggered(
            when_asked(EventMask::TRIGGER_ON_STACK, your_attacking_creature_causes_its_ability_to_trigger),
            quest_counter_then_copy,
            "Whenever a creature you control attacking causes a triggered ability of that creature to trigger, put a quest counter on this enchantment. Then if it has four or more quest counters on it, you may copy that ability. You may choose new targets for the copy.",
        ),
    ],
    ..oracle::FIREBENDER_ASCENSION
};

pub static EXTRA: &[&CardDef] = &[&FIREBENDING_SOLDIER];
