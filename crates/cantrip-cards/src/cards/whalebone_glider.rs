use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}, {T}: Target creature with power 3 or less gains flying until end of turn." (CR 115.1).
fn target_creature_power_3_less_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).power.is_some_and(|power| power <= 3))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(TAP),
                effects::first_target_gains_flying,
                "{2}, {T}: Target creature with power 3 or less gains flying until end of turn.",
            )
            .targets(&[target(target_creature_power_3_less_target_0)]),
    ],
    ..oracle::WHALEBONE_GLIDER
};
