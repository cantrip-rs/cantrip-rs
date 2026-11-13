use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_DIES,
        effects::target_player_loses_1_life_you_gain_1,
        "Whenever this creature or another creature you control dies, target opponent loses 1 life and you gain 1 life.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::VENGEFUL_BLOODWITCH
};
