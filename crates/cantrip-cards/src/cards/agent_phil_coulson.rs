use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_each_other_of_subtype_you_control;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// Vigilance is a core keyword. "{T}: Put a +1/+1 counter on each other Hero you control" (CR 122.1, 602.2): the counters are
/// put on together as the ability resolves, on the Heroes its controller controls then other than this creature.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        plus_one_counter_on_each_other_of_subtype_you_control::<{ lands::sub(Subtype::Hero) }>,
        "{T}: Put a +1/+1 counter on each other Hero you control.",
    )],
    ..oracle::AGENT_PHIL_COULSON
};
