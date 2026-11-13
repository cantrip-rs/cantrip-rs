use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::distribute_plus_one_counters,
            "When this Vehicle enters, distribute two +1/+1 counters among one or two other target Vehicles and/or creatures you control.",
        )
        .targets(&[targets::OTHER_CREATURE_OR_VEHICLE_YOU_CONTROL.between(1, 2)])
        .divide(Divide { total: 2, reads: None }),
        crew(1, "Crew 1"),
    ],
    ..oracle::CLOUDSPIRE_SKYCYCLE
};
