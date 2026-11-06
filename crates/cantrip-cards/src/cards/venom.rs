use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Check the creature opposing the enchanted creature as each block is declared (CR 509.3b, 509.3d).
fn blocks_or_becomes_blocked(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let enchanted = q.obj(asker.source).attached_to?;
    let other = if *blocker == enchanted {
        *attacker
    } else if *attacker == enchanted {
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

/// Schedule the same object for destruction at end of combat (CR 603.7c, 400.7).
fn schedule_destruction(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::Step { step: Step::EndOfCombat, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        item.subject,
    );
    Ok(())
}

/// Destroy that object even if it is no longer a creature (CR 701.8a).
fn destroy_other(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(other) = item.subject {
        g.destroy(other);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::BLOCKS, blocks_or_becomes_blocked),
        schedule_destruction,
        "Whenever enchanted creature blocks or becomes blocked by a non-Wall creature, destroy the other creature at end of combat.",
    )],
    delayed: &[delayed_trigger(destroy_other, "Destroy the other creature.")],
    enchant: Some(targets::CREATURE),
    ..oracle::VENOM
};
