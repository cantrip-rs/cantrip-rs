use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, effects::damage_each_opponent_n::<2>, "{T}: This creature deals 2 damage to each opponent.")],
    ..oracle::PANICKED_ALTISAUR
};
