use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// A creature you control with toughness 2 or less (CR 115.1).
fn target_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t)
        && q.chars(id).toughness.is_some_and(|toughness| toughness <= 2))
}

/// Flip at the next end step; only the ability's controller can sacrifice the creature (CR 603.7c, 701.21a).
fn flip_and_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.flip_coin(item.controller) {
        delayed::sacrifice_created(g, item)?;
    }
    Ok(())
}

/// Grant flying until end of turn and schedule the flip about that creature (CR 611.2a, 603.7).
fn lift(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::first_target_gains_flying(g, item)?;
    g.end_sentence();
    delayed::at_next_end_step_about(g, item, 0, &[creature]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{R}"), lift, "{R}: Target creature you control with toughness 2 or less gains flying until end of turn. Flip a coin at the beginning of the next end step. If you lose the flip, sacrifice that creature.")
        .targets(&[target(target_creature)])],
    delayed: &[delayed_trigger(flip_and_sacrifice, "Flip a coin at the beginning of the next end step. If you lose the flip, sacrifice that creature.")],
    ..oracle::GOBLIN_KITES
};
