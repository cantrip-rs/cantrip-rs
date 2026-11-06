use cantrip_api::*;
use cantrip_mechanics::regenerate;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{U}{U}{U}"), regenerate::this_effect, "{U}{U}{U}: Regenerate this creature.")],
    ..oracle::GHOST_SHIP
};
