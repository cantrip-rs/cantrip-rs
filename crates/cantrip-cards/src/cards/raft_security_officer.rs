use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This ability costs {1} less to activate if it targets a creature with power 3 or less" (CR 601.2f, 602.2b): the target as the
/// total cost is determined, once it is announced (CR 601.2c); generic mana only (CR 118.7a). Its power is as the layers have it
/// now, a negative one included (CR 107.1b).
fn less_for_a_creature_with_power_three_or_less(q: &dyn Query, _: &Asker, _: Costed, targets: &[Target]) -> CostDelta {
    match targets.first() {
        Some(&Target::Object(creature)) if q.chars(creature).power.is_some_and(|power| power <= 3) => CostDelta::less(1),
        _ => CostDelta::NONE,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}, {T}"), effects::tap_targets, "{2}, {T}: Tap target creature. This ability costs {1} less to activate if it targets a creature with power 3 or less.")
        .targets(&[targets::CREATURE])
        .cost_change(less_for_a_creature_with_power_three_or_less)],
    ..oracle::RAFT_SECURITY_OFFICER
};
