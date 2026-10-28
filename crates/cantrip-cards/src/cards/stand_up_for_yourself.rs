use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature with power 3 or greater" (CR 115.1) as the layers give the power now, so a counter or an effect moves a
/// creature in and out of range.
fn creature_with_power_three_or_greater(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).power.is_some_and(|power| power >= 3))
}

/// "Destroy target creature with power 3 or greater." (CR 701.8a.)
pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_with_power_three_or_greater)], effects::destroy_first_target),
    ..oracle::STAND_UP_FOR_YOURSELF
};
