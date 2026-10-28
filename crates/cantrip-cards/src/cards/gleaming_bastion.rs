use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &lands::basic_land_dual(
        Color::White,
        Color::Blue,
        "{T}: Add {W} or {U}. Activate only if this land entered this turn or if you control a basic land.",
    ),
    ..oracle::GLEAMING_BASTION
};
