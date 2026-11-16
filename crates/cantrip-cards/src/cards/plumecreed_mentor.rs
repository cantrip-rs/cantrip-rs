use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature or another creature you control with flying enters" (CR 603.6a): this creature entering, flying or not, or
/// another creature you control that has flying as it enters.
fn this_or_another_flyer_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::this_enters(q, me.source, event).or_else(|| triggers::another_creature_you_control_with_flying_enters(q, me, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_flyer_enters),
        effects::plus_one_counters_on_target::<1>,
        "Whenever this creature or another creature you control with flying enters, put a +1/+1 counter on target creature you control without flying.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL_WITHOUT_FLYING])],
    ..oracle::PLUMECREED_MENTOR
};
