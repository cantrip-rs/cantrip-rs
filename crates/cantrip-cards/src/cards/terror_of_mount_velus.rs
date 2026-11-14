use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::DoubleStrike]).0 }>,
        "When this creature enters, creatures you control gain double strike until end of turn.",
    )],
    ..oracle::TERROR_OF_MOUNT_VELUS
};
