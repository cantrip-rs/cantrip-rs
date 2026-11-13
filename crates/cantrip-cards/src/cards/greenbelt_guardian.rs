use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{G}"), effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Trample]).0 }>, "{G}: Target creature gains trample until end of turn.").targets(&[targets::CREATURE]),
        ability(
            cost("{3}{G}"),
            effects::three_plus_one_counters_on_this,
            "Exhaust \u{2014} {3}{G}: Put three +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
    ],
    ..oracle::GREENBELT_GUARDIAN
};
