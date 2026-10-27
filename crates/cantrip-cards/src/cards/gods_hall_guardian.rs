use cantrip_api::*;
use cantrip_mechanics::foretell;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    foretell: Some(mana("{3}{W}")),
    abilities: &[
        foretell::ABILITY,
    ],
    ..oracle::GODS_HALL_GUARDIAN
};
