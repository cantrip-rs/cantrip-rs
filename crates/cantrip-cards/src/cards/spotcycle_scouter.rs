use cantrip_api::*;
use cantrip_mechanics::{crew, effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::scry_two, "When this Vehicle enters, scry 2."),
        crew::crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::SPOTCYCLE_SCOUTER
};
