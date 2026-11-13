use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}, {T}"),
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Haste]).0 }>,
        "{1}, {T}: Target creature gains haste until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::TAXI_DRIVER
};
