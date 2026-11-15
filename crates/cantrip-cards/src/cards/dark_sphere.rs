use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "Prevent half that damage, rounded down" (CR 615.1): half of the damage as the effects applied before it have left it
/// (CR 616.1f) - two of these apply one after the other, 5 damage becoming 3 and then 2 (the ruling). Half of 1 is none,
/// and a shield that prevents no damage is not used up (CR 609.7b).
fn half_rounded_down(_: &dyn Query, _: &Asker, d: &DamageEvent) -> u32 {
    d.amount / 2
}

pub static CARD: CardDef = CardDef {
    // The source is chosen as the ability resolves (CR 609.7a); the effect watches for the next time it would deal damage to
    // you (CR 615.8) until the turn ends.
    abilities: &[ability(TAP.and(SACRIFICE_THIS), prevention::resolve_next_time_chosen_source_would_damage_you,
        "{T}, Sacrifice this artifact: The next time a source of your choice would deal damage to you this turn, prevent half that damage, rounded down.")],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents half the damage from the chosen source, rounded down",
        portion: Some(PreventionPortion { amount: half_rounded_down, words: "half, rounded down, of" }),
        text: "The next time a source of your choice would deal damage to you this turn, prevent half that damage, rounded down.",
        ..prevention::FROM_CHOSEN_SOURCE
    }],
    ..oracle::DARK_SPHERE
};
