use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_target;
use cantrip_mechanics::targets::permanent_of_any_subtype;
use cantrip_oracle as oracle;

/// "Target Mount or Vehicle" (CR 115.1, 205.3m, 301.7): any permanent with either type, whoever controls it, and whether or not
/// the Vehicle is a creature right now.
fn mount_or_vehicle(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    permanent_of_any_subtype(q, asker, t, &[Subtype::Mount, Subtype::Vehicle])
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}{W}"), plus_one_counter_on_target, "{3}{W}: Put a +1/+1 counter on target Mount or Vehicle.").targets(&[target(mount_or_vehicle)])],
    ..oracle::DARING_MECHANIC
};
