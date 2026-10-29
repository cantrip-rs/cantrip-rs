use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "If it's the first combat phase of the turn" (CR 603.4, 500.8): the turn's beginning of combat steps are in its history, and the
/// first combat phase is the only one of them so far. It is asked as the ability triggers and again as it resolves.
fn first_combat_phase_of_the_turn(q: &dyn Query, _: &Asker) -> bool {
    q.history().this_turn().iter().filter(|r| matches!(r.event, Event::StepBegan { step: Step::BeginCombat })).count() == 1
}

/// Untap one or two target attacking creatures (CR 701.26a). After this phase, there is an additional combat phase (CR 500.8).
fn untap_and_extra_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::untap_targets(g, item)?;
    g.add_combat_phase();
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        untap_and_extra_combat,
        "Whenever Raph & Leo attack, if it's the first combat phase of the turn, untap one or two target attacking creatures. After this phase, there is an additional combat phase.",
    )
    .targets(&[targets::ATTACKING_CREATURE.between(1, 2)])
    .condition(first_combat_phase_of_the_turn)],
    ..oracle::RAPH_LEO_SIBLING_RIVALS
};
