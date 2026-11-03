use cantrip_api::*;
use cantrip_oracle as oracle;

/// Each non-Wall creature that blocks or is blocked by this creature (CR 509.1, 603.2).
fn blocks_or_is_blocked_by(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let other = if *blocker == asker.source {
        *attacker
    } else if *attacker == asker.source {
        *blocker
    } else {
        return None;
    };
    let wall = q.words(asker.source).subtype(Subtype::Wall);
    if q.chars(other).is(CardType::Creature) && !q.chars(other).has_subtype(wall) {
        Fired::about(Target::Object(other))
    } else {
        None
    }
}

/// Set up the delayed ability about that creature, even if the Basilisk has left (CR 603.7, 113.7a).
fn destroy_at_end_of_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::Step { step: Step::EndOfCombat, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        item.subject,
    );
    Ok(())
}

/// Destroy the same object if it is still a permanent (CR 603.7c, 701.8a, 400.7).
fn destroy_that_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        g.destroy(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::BLOCKS, blocks_or_is_blocked_by),
        destroy_at_end_of_combat,
        "Whenever this creature blocks or becomes blocked by a non-Wall creature, destroy that creature at end of combat.",
    )],
    delayed: &[delayed_trigger(destroy_that_creature, "Destroy that creature at end of combat.")],
    ..oracle::THICKET_BASILISK
};
