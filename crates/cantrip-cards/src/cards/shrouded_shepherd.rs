use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Cleave Shadows gives creatures your opponents control -1/-1 until end of turn.
pub static CLEAVE_SHADOWS: CardDef = CardDef {
    spell: spell(&[], effects::creatures_your_opponents_control_get::<-1, -1>),
    ..oracle::CLEAVE_SHADOWS
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::target_creature_gets::<2, 2>,
        "When this creature enters, target creature you control gets +2/+2 until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    adventure: Some(&CLEAVE_SHADOWS),
    ..oracle::SHROUDED_SHEPHERD
};

pub static EXTRA: &[&CardDef] = &[&CLEAVE_SHADOWS];
