use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_plus_one_counters::<2>),
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_WITH_COUNTERS_DIES,
        effects::put_the_subjects_counters_on_target,
        "Whenever this creature or another creature you control dies, if it had counters on it, put its counters on up to one target creature you control.",
    )
    .targets(&[up_to_one(targets::CREATURE_YOU_CONTROL.filter)])],
    ..oracle::HOST_OF_THE_HEREAFTER
};
