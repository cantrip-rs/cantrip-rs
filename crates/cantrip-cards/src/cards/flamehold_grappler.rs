use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, triggers::{self, THIS_ENTERS}};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        delayed::watch_for_the_next,
        "When this creature enters, copy the next spell you cast this turn when you cast it. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)",
    )],
    delayed: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_about_it),
        copy::copy_the_cast_spell,
        "Copy the next spell you cast this turn when you cast it. You may choose new targets for the copy.",
    )],
    ..oracle::FLAMEHOLD_GRAPPLER
};
