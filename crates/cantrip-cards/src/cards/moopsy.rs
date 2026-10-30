use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}"), effects::this_gains_reach, "{2}: This creature gains reach until end of turn.")],
    ..oracle::MOOPSY
};
