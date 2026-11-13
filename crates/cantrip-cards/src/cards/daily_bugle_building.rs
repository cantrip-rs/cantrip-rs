use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        lands::ANY_COLOR_FOR_ONE,
        ability(cost("{1}, {T}"), effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Menace]).0 }>, "Smear Campaign \u{2014} {1}, {T}: Target legendary creature gains menace until end of turn. Activate only as a sorcery.")
            .sorcery_speed()
            .targets(&[targets::LEGENDARY_CREATURE]),
    ],
    ..oracle::DAILY_BUGLE_BUILDING
};
