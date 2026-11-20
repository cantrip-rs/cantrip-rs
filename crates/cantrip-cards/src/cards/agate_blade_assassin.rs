use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::defending_player_loses_life_you_gain_life::<1>,
        "Whenever this creature attacks, defending player loses 1 life and you gain 1 life.",
    )],
    ..oracle::AGATE_BLADE_ASSASSIN
};
