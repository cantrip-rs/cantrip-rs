use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &lands::basic_land_dual(
        Color::Blue,
        Color::Black,
        "{T}: Add {U} or {B}. Activate only if this land entered this turn or if you control a basic land.",
    ),
    ..oracle::HIDDEN_LAIR
};
