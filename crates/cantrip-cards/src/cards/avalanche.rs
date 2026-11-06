use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// A snow land (CR 115.1, 205.4g).
fn snow_land(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::LAND.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).supertypes.snow)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(snow_land).counted()], effects::destroy_each_target),
    ..oracle::AVALANCHE
};
