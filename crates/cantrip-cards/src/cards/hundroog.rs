use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{3}"), "Cycling {3} ({3}, Discard this card: Draw a card.)"),
    ],
    ..oracle::HUNDROOG
};
