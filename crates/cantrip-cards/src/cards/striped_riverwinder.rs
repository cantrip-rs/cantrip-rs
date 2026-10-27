use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{U}"), "Cycling {U} ({U}, Discard this card: Draw a card.)"),
    ],
    ..oracle::STRIPED_RIVERWINDER
};
