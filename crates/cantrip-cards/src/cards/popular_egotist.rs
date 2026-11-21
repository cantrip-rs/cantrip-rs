use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}{B}").and(sacrifice(1, targets::filters::another_creature_or_enchantment)),
            effects::this_gains_indestructible_then_tap,
            "{1}{B}, Sacrifice another creature or enchantment: This creature gains indestructible until end of turn. Tap it.",
        ),
        triggered(
            triggers::YOU_SACRIFICE_A_PERMANENT,
            effects::target_player_loses_1_life_you_gain_1,
            "Whenever you sacrifice a permanent, target opponent loses 1 life and you gain 1 life.",
        )
        .on_sacrifice()
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::POPULAR_EGOTIST
};
