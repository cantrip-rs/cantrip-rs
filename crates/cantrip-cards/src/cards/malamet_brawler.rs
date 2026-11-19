use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Trample]).0 }>,
        "Whenever this creature attacks, target attacking creature gains trample until end of turn.",
    )
    .targets(&[targets::ATTACKING_CREATURE])],
    ..oracle::MALAMET_BRAWLER
};
