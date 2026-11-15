use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Deathtouch]).0 }>,
        "Whenever this creature attacks, another target creature you control gains deathtouch until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::FLESH_BURROWER
};
