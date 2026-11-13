use cantrip_api::*;
use cantrip_mechanics::effects::each_opponent_loses_1_life_you_gain_1;
use cantrip_mechanics::triggers::ARTIFACT_YOU_CONTROL_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ARTIFACT_YOU_CONTROL_ENTERS,
        each_opponent_loses_1_life_you_gain_1,
        "Whenever this creature or another artifact you control enters, each opponent loses 1 life and you gain 1 life.",
    )],
    ..oracle::PACTDOLL_TERROR
};
