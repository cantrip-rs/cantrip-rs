use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Menace]).0 }>,
        "When this creature enters, target creature you control gains menace until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::DINOTOMATON
};
