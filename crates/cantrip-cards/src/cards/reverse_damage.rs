use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // The source is chosen as the spell resolves (CR 609.7a, 615.8).
    spell: spell(&[], prevention::resolve_next_time_chosen_source_would_damage_you),
    granted_preventions: &[GrantedPreventionDef {
        // "You gain life equal to the damage prevented this way": part of the damage event (CR 120.4's Awe Strike example).
        gain_prevented: true,
        text: "The next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way.",
        ..prevention::FROM_CHOSEN_SOURCE
    }],
    ..oracle::REVERSE_DAMAGE
};
