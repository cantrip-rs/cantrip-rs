use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::REPARTEE};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        REPARTEE,
        effects::each_opponent_loses_1_life_you_gain_1,
        "Repartee \u{2014} Whenever you cast an instant or sorcery spell that targets a creature, each opponent loses 1 life and you gain 1 life.",
    )],
    ..oracle::MELANCHOLIC_POET
};
