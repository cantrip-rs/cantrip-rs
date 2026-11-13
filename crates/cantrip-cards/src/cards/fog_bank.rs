use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Prevent all combat damage that would be dealt to and dealt by this creature." (CR 615.1): combat damage with this creature as
/// its target or its source.
fn prevent_all_combat_damage_to_and_by_this(_: &dyn Query, asker: &Asker, e: &DamageEvent) -> bool {
    e.combat && (e.target == Target::Object(asker.source) || e.source == asker.source)
}

pub static CARD: CardDef = CardDef {
    preventions: &[PreventionDef {
        hint: "prevents all combat damage that would be dealt to and dealt by this creature",
        applies: prevent_all_combat_damage_to_and_by_this,
        amount: PreventionAmount::All,
    }],
    ..oracle::FOG_BANK
};
