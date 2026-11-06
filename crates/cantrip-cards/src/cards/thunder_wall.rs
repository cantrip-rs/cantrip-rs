use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{U}"),
        effects::this_gets::<1, 1>,
        "{U}: This creature gets +1/+1 until end of turn.",
    )],
    ..oracle::THUNDER_WALL
};
