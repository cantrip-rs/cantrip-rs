use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::tap_and_stun::<1>, "I \u{2014} Heavenly Strike \u{2014} Tap target creature an opponent controls. Put a stun counter on it.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, effects::tap_and_stun::<1>, "II \u{2014} Heavenly Strike \u{2014} Tap target creature an opponent controls. Put a stun counter on it.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(
            3,
            effects::draw_a_card_for_each_tapped_creature_your_opponents_control,
            "III \u{2014} Diamond Dust \u{2014} Draw a card for each tapped creature your opponents control.",
        ),
    ],
    ..oracle::SUMMON_SHIVA
};
