use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Another target creature with power 2 or less" (CR 115.1, 109.5): any controller's creature other than the source.
fn other_creature_with_power_2_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::other_creature_filter(q, asker, t) && targets::creature_with_power_at_most::<2>(q, asker, t)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, effects::target_creature_cant_be_blocked_this_turn,
        "{T}: Another target creature with power 2 or less can't be blocked this turn.")
        .targets(&[target(other_creature_with_power_2_or_less)])],
    ..oracle::GOBLIN_SMUGGLER
};
