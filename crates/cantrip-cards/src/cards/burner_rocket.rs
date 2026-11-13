use cantrip_api::*;
use cantrip_mechanics::{crew, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::target_creature_gets_and_gains_trample::<2, 0>,
            "When this Vehicle enters, target creature you control gets +2/+0 and gains trample until end of turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        crew::crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::BURNER_ROCKET
};
