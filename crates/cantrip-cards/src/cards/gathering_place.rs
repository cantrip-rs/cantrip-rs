use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &lands::basic_land_dual(
        Color::Green,
        Color::White,
        "{T}: Add {G} or {W}. Activate only if this land entered this turn or if you control a basic land.",
    ),
    ..oracle::GATHERING_PLACE
};
