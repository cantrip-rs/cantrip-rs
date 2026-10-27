use cantrip_api::*;
use cantrip_mechanics::foretell;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    foretell: Some(ManaCost::ZERO),
    abilities: &[
        foretell::ABILITY,
    ],
    ..oracle::SCORN_EFFIGY
};
