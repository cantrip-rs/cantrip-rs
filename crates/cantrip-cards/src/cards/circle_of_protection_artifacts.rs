use cantrip_api::*;
use cantrip_mechanics::{prevention, select};
use cantrip_oracle as oracle;

/// The chosen source must still be an artifact when it would deal damage (CR 609.7b).
fn artifact_source(q: &dyn Query, _: &Asker, chosen: ObjectId, event: &DamageEvent) -> bool {
    prevention::by_chosen_source(q, chosen, event) && q.chars(event.source).is(CardType::Artifact)
}

/// Choose an artifact source as the ability resolves and prevent its next damage to you this turn (CR 609.7a, 615.8).
fn protect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = prevention::SOURCE.taking(select::as_many_as_can(1)).with(select::ARTIFACT);
    prevention::next_time_source_would_damage_you(g, item, 0, rule)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}"), protect, "{2}: The next time an artifact source of your choice would deal damage to you this turn, prevent that damage.")],
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents damage from the chosen artifact source",
        damage: "damage from the chosen artifact source",
        of_it: None,
        applies: artifact_source,
        portion: None,
        then: None,
        gain_prevented: false,
        text: "The next time an artifact source of your choice would deal damage to you this turn, prevent that damage.",
    }],
    ..oracle::CIRCLE_OF_PROTECTION_ARTIFACTS
};
