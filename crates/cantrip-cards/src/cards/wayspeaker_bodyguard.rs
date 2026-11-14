use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, flurry, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::return_first_target_to_hand,
            "When this creature enters, return target nonland permanent card with mana value 2 or less from your graveyard to your hand.",
        )
        .targets(&[targets::NONLAND_PERMANENT_CARD_WITH_MANA_VALUE_2_OR_LESS_IN_YOUR_GRAVEYARD]),
        flurry::trigger(
            "Flurry \u{2014} Whenever you cast your second spell each turn, tap target creature an opponent controls.",
            effects::tap_targets,
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::WAYSPEAKER_BODYGUARD
};
