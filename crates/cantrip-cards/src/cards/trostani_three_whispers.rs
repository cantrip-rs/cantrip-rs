use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}{G}"),
            effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Deathtouch]).0 }>,
            "{1}{G}: Target creature gains deathtouch until end of turn.",
        )
        .targets(&[targets::CREATURE]),
        ability(
            cost("{G/W}"),
            effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Vigilance]).0 }>,
            "{G/W}: Target creature gains vigilance until end of turn.",
        )
        .targets(&[targets::CREATURE]),
        ability(
            cost("{2}{W}"),
            effects::target_gains_double_strike,
            "{2}{W}: Target creature gains double strike until end of turn.",
        )
        .targets(&[targets::CREATURE]),
    ],
    ..oracle::TROSTANI_THREE_WHISPERS
};
