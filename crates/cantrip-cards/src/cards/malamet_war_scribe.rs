use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::creatures_you_control_get::<2, 1>,
        "When this creature enters, creatures you control get +2/+1 until end of turn.",
    )],
    ..oracle::MALAMET_WAR_SCRIBE
};
