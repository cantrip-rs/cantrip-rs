use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Creature, Vehicle, or nonbasic land" (CR 115.1, 205.3g, 301.7): a permanent on the battlefield that is a creature, a Vehicle
/// whether or not it is crewed, or a land that is not basic.
fn creature_vehicle_or_nonbasic_land(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::creature_or_vehicle(q, id)) || targets::nonbasic_land_filter(q, asker, t)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_vehicle_or_nonbasic_land)], effects::exile_first_target_then_scry::<1>),
    ..oracle::RAY_OF_RUIN
};
