use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Prevent all combat damage that would be dealt to and dealt by enchanted creature." (CR 615.1).
fn prevent_all_combat_damage_would_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    let Some(creature) = q.attached_to(asker.source) else { return false };
    e.combat && (e.target == Target::Object(creature) || e.source == creature)
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all combat damage that would be dealt to and dealt by enchanted creature",
            applies: prevent_all_combat_damage_would_applies,
            amount: PreventionAmount::All,
        },
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::GASEOUS_FORM
};
