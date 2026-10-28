use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &lands::basic_land_dual(
        Color::Black,
        Color::Red,
        "{T}: Add {B} or {R}. Activate only if this land entered this turn or if you control a basic land.",
    ),
    ..oracle::DARK_FORTRESS
};
