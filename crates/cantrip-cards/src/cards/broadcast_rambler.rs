use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tokens::create_a_thopter, "When this Vehicle enters, create a 1/1 colorless Thopter artifact creature token with flying."),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::BROADCAST_RAMBLER
};

pub static EXTRA: &[&CardDef] = &[&tokens::THOPTER];
