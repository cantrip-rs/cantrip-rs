use cantrip_api::*;
use cantrip_mechanics::{combat, prevention, select};
use cantrip_oracle as oracle;

/// "An unblocked creature" (CR 509.1h), as the choice's rule.
fn unblocked_creature(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| combat::is_unblocked(q, id))
}

/// The damage the effect watches for: combat damage (CR 510.2) dealt by the creature chosen, while it is still an unblocked
/// creature as it would deal it (CR 609.7b) - so not a blocked creature's trample damage (the ruling). "To you" is the effect's
/// own recipient. Once the creature has left the battlefield it is a new object (CR 400.7), which the effect is not about.
fn from_chosen_unblocked_creature(q: &dyn Query, _: &Asker, chosen: ObjectId, d: &DamageEvent) -> bool {
    d.combat && d.source == chosen && combat::is_unblocked(q, d.source)
}

/// "Prevent all but 1 of that damage" (CR 615.1): of the damage as the effects applied before it have left it (CR 616.1f), all
/// of it but 1. Of 1 it prevents none, and a shield that prevents no damage is not used up (CR 609.7b).
fn all_but_one(_: &dyn Query, _: &Asker, d: &DamageEvent) -> u32 {
    d.amount - d.amount.min(1)
}

/// "{1}: The next time an unblocked creature of your choice would deal combat damage to you this turn, prevent all but 1 of
/// that damage." The creature is chosen as the ability resolves (CR 609.7a) - not targeted - among the creatures unblocked
/// then; with none (before blockers are declared, say), nothing is chosen and nothing happens. The effect watches for the
/// next time it would deal combat damage to you (CR 615.8) until the turn ends.
fn shield_from_chosen_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = prevention::SOURCE
        .called("unblocked creature")
        .taking(select::as_many_as_can(1))
        .with(select::part(&(unblocked_creature as SelectFn), 0, ""));
    prevention::next_time_source_would_damage_you(g, item, 0, rule)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}"),
        shield_from_chosen_creature,
        "{1}: The next time an unblocked creature of your choice would deal combat damage to you this turn, prevent all but 1 of that damage.",
    )],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents all but 1 of the combat damage from the chosen unblocked creature",
        damage: "combat damage from the chosen unblocked creature",
        applies: from_chosen_unblocked_creature,
        portion: Some(PreventionPortion { amount: all_but_one, words: "all but 1 of" }),
        text: "The next time an unblocked creature of your choice would deal combat damage to you this turn, prevent all but 1 of that damage.",
        ..prevention::FROM_CHOSEN_SOURCE
    }],
    ..oracle::FORCEFIELD
};
