use cantrip_api::*;
use cantrip_mechanics::{effects, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "Creatures you control with +1/+1 counters on them have trample." (CR 613.1f, 702.19);
/// "At the beginning of combat on your turn, put a +1/+1 counter on target creature you control." (CR 507.1, 122.1, 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        effects::plus_one_counter_on_target,
        "At the beginning of combat on your turn, put a +1/+1 counter on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_creatures_with_plus_one_counters,
        statics::have_trample,
        "Creatures you control with +1/+1 counters on them have trample.",
    )],
    ..oracle::TRAINING_REGIMEN
};
