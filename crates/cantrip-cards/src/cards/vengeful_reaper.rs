use cantrip_api::*;
use cantrip_mechanics::foretell;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    foretell: Some(mana("{1}{B}")),
    abilities: &[
        foretell::ABILITY,
    ],
    ..oracle::VENGEFUL_REAPER
};
