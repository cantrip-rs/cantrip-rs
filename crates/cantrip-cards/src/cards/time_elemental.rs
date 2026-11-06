use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ATTACKS_OR_BLOCKS};
use cantrip_oracle as oracle;

/// "At end of combat, sacrifice it and it deals 5 damage to you" (CR 701.21a, 113.7a).
fn delayed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&source) = g.event_objects(item).first() else { return Ok(()) };
    instruction(g, |g| {
        g.sacrifice(item.controller, &[source]);
        g.damage_together(&[(source, Target::Player(item.controller), 5)]);
    });
    Ok(())
}

/// "When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you" (CR 603.7).
fn combat_sacrifice_deals_5_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.delayed_trigger_about(
        DelayedWhen::Step { step: Step::EndOfCombat, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        &[item.object],
    );
    Ok(())
}

/// "Return target permanent that isn't enchanted to its owner's hand" (CR 115.1).
fn unenchanted_permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id)
        && !q.state().battlefield().iter().any(|&a| q.state().obj(a).attached_to == Some(id)
            && q.chars(a).has_subtype(Subtype::Aura)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_OR_BLOCKS,
            combat_sacrifice_deals_5_damage,
            "When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you.",
        ),
        ability(
            cost("{2}{U}{U}").and(TAP),
            effects::return_first_target_to_hand,
            "{2}{U}{U}, {T}: Return target permanent that isn't enchanted to its owner's hand.",
        ).targets(&[target(unenchanted_permanent)]),
    ],
    delayed: &[delayed_trigger(
        delayed,
        "When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you.",
    )],
    ..oracle::TIME_ELEMENTAL
};
