use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// Make the target unblockable and schedule destruction at the next end of combat (CR 603.7, 701.8).
fn make_unblockable(g: &mut dyn Game, item: &StackItem, destroy_source: bool) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    effects::target_creature_cant_be_blocked_this_turn(g, item)?;
    g.end_sentence();
    let mut objects = vec![target];
    if destroy_source && item.object != target {
        objects.push(item.object);
    }
    g.delayed_trigger_about(
        DelayedWhen::Step { step: Step::EndOfCombat, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, if destroy_source { 0 } else { 1 }),
        &objects,
    );
    Ok(())
}

fn destroy_both(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    make_unblockable(g, item, true)
}

fn destroy_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    make_unblockable(g, item, false)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{R}{R}").and(TAP), destroy_both, "{R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it and this creature at end of combat.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        ability(cost("{R}{R}{R}{R}").and(TAP), destroy_target, "{R}{R}{R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it at end of combat.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    delayed: &[
        delayed_trigger(delayed::destroy_carried, "Destroy it and this creature at end of combat."),
        delayed_trigger(delayed::destroy_carried, "Destroy it at end of combat."),
    ],
    ..oracle::GOBLIN_SAPPERS
};
