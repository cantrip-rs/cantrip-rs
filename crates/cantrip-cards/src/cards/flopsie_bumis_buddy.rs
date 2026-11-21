use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Each creature you control with power 4 or greater can't be blocked by more than one creature" (CR 509.1b, 113.1a): asked of
/// each attacker with Flopsie as the source, the power the attacker has as blockers are declared (CR 613).
fn power_four_creatures_you_control_one_blocker_at_most(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> Option<usize> {
    let strong = q.chars(attacker).power.is_some_and(|power| power >= 4);
    (q.controller(attacker) == q.controller(source) && strong).then_some(1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::plus_one_counter_on_each_creature_you_control,
        "When Flopsie enters, put a +1/+1 counter on each creature you control.",
    )],
    blocker_limits: &[power_four_creatures_you_control_one_blocker_at_most],
    ..oracle::FLOPSIE_BUMIS_BUDDY
};
