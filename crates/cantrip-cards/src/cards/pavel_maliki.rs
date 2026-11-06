use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{B}{R}"), effects::this_gets::<1, 0>, "{B}{R}: Pavel Maliki gets +1/+0 until end of turn.")],
    ..oracle::PAVEL_MALIKI
};
