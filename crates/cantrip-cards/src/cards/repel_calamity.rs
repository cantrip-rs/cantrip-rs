use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature with power or toughness 4 or greater" (CR 115.1, 208.1): either number, as the layers give it now, is 4 or
/// more, judged as the spell is cast and again as it resolves (CR 608.2b). A creature with no power or toughness has neither.
fn creature_with_power_or_toughness_four_or_greater(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_filter(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).power.is_some_and(|v| v >= 4) || q.chars(id).toughness.is_some_and(|v| v >= 4))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_with_power_or_toughness_four_or_greater)], effects::destroy_first_target),
    ..oracle::REPEL_CALAMITY
};
