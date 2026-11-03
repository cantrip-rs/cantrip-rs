use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}"),
        effects::this_gets::<1, 0>,
        "{1}: This creature gets +1/+0 until end of turn.",
    )],
    ..oracle::WALL_OF_OPPOSITION
};
