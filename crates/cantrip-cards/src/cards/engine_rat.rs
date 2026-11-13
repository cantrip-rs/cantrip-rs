use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{5}{B}"), effects::each_opponent_loses_2_life, "{5}{B}: Each opponent loses 2 life.")],
    ..oracle::ENGINE_RAT
};
