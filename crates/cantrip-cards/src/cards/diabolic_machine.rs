use cantrip_api::*;
use cantrip_mechanics::regenerate;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}"), regenerate::this_effect, "{3}: Regenerate this creature.")],
    ..oracle::DIABOLIC_MACHINE
};
