use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{1}"), "Cycling {1} ({1}, Discard this card: Draw a card.)"),
    ],
    ..oracle::IMPOSING_VANTASAUR
};
