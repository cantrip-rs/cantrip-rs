use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}{B}"), effects::damage_each_opponent_n::<1>, "{3}{B}: This creature deals 1 damage to each opponent.")],
    ..oracle::MERCILESS_ENFORCERS
};
