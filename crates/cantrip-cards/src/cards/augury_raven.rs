use cantrip_api::*;
use cantrip_mechanics::foretell;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    foretell: Some(mana("{1}{U}")),
    abilities: &[
        foretell::ABILITY,
    ],
    ..oracle::AUGURY_RAVEN
};
