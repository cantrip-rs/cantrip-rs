use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this enchantment enters, target opponent discards two cards." (CR 701.9): their choice; nothing if they are no longer a
/// legal target as the ability resolves (CR 608.2b).
/// "Whenever a creature you control attacks alone, target opponent loses 1 life and you gain 1 life." (CR 506.5, 119.3): the
/// target is chosen as the ability goes on the stack; the life is gained only if the ability still resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::target_player_discards::<2>, "When this enchantment enters, target opponent discards two cards.")
            .targets(&[targets::OPPONENT]),
        triggered(
            triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE,
            effects::target_player_loses_1_life_you_gain_1,
            "Whenever a creature you control attacks alone, target opponent loses 1 life and you gain 1 life.",
        )
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::HYDRA_INFILTRATION
};
