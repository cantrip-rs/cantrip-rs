use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Another target creature you control with power 2 or less" (CR 115.1, 109.5, 208.1): a creature on the battlefield under
/// this ability's controller's control other than the source, with power 2 or less as it is now.
fn another_creature_you_control_with_power_two_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && id != asker.source
        && q.controller(id) == asker.controller
        && q.chars(id).is(CardType::Creature)
        && q.chars(id).power.is_some_and(|p| p <= 2))
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}").and(TAP), effects::target_creature_cant_be_blocked_this_turn, "{1}, {T}: Another target creature you control with power 2 or less can't be blocked this turn.")
        .targets(&[target(another_creature_you_control_with_power_two_or_less)])],
    ..oracle::EARDRUM_RATTLER
};
