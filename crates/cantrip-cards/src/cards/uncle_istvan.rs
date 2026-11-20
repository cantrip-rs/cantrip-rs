use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt to this creature by creatures." (CR 615.1): `prevention::damage_by_a_creature`, to this.
fn damage_by_a_creature_to_this(q: &dyn Query, asker: &Asker, e: &DamageEvent) -> bool {
    prevention::damage_by_a_creature(q, e)
        && matches!(e.target, Target::Object(o) if o == asker.source)
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to this creature by creatures",
            applies: damage_by_a_creature_to_this,
            amount: PreventionAmount::All,
            then: None,
        },
    ],
    ..oracle::UNCLE_ISTVAN
};
