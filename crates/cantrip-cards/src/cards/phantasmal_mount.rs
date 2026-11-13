use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

/// A creature you control with toughness 2 or less (CR 115.1).
fn target_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t)
        && q.chars(id).toughness.is_some_and(|toughness| toughness <= 2))
}

/// The delayed ability's controller sacrifices the original Mount if able (CR 701.21a, 603.7e).
fn sacrifice_mount(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(item.controller, &[item.object]);
    Ok(())
}

/// Grant the bonus and set up both leave-the-battlefield triggers for this turn (CR 603.7, 611.2a).
fn lift(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.add_effect(item.object, creature, EffectKind::Pump(1, 1), Duration::EndOfTurn);
        g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])), Duration::EndOfTurn);
    });
    g.end_sentence();
    g.delayed_trigger_about(DelayedWhen::ThisTurn, item.object, item.controller,
        AbilityRef::delayed(item.ability.card, 0), &[creature]);
    g.end_sentence();
    g.delayed_trigger(DelayedWhen::WatchingThisTurn, item.object, item.controller,
        AbilityRef::delayed(item.ability.card, 1), Target::Object(creature));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, lift,
        "{T}: Target creature you control with toughness 2 or less gets +1/+1 and gains flying until end of turn. When this creature leaves the battlefield this turn, sacrifice that creature. When the creature leaves the battlefield this turn, sacrifice this creature.")
        .targets(&[target(target_creature)])],
    delayed: &[
        triggered(triggers::THIS_LEAVES, delayed::sacrifice_created,
            "When this creature leaves the battlefield this turn, sacrifice that creature."),
        triggered(triggers::THIS_LEAVES, sacrifice_mount,
            "When the creature leaves the battlefield this turn, sacrifice this creature."),
    ],
    ..oracle::PHANTASMAL_MOUNT
};
