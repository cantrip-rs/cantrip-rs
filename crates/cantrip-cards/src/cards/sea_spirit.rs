use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{U}"), effects::this_gets::<1, 0>, "{U}: This creature gets +1/+0 until end of turn.")],
    ..oracle::SEA_SPIRIT
};
