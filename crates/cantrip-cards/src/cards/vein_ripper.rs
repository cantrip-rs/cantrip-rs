use cantrip_api::*;
use cantrip_mechanics::triggers::{CREATURE_DIES, ward_sacrifice_a_creature};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward_sacrifice_a_creature("Ward\u{2014}Sacrifice a creature."),
        // "Target opponent loses 2 life and you gain 2 life" (CR 119.3): its only target gone, the ability does not resolve (CR 608.2b).
        triggered(
            CREATURE_DIES,
            |g, item| effects::target_player_loses_life_you_gain_life(g, item, 2),
            "Whenever a creature dies, target opponent loses 2 life and you gain 2 life.",
        )
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::VEIN_RIPPER
};
