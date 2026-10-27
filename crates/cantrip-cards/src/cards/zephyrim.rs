use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: multikicker(mana("{2}")),
    miracle: Some(mana("{1}{W}")),
    abilities: &[
        triggers::SQUAD,
    ],
    ..oracle::ZEPHYRIM
};
