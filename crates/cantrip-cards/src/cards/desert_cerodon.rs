use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{R}"), "Cycling {R} ({R}, Discard this card: Draw a card.)"),
    ],
    ..oracle::DESERT_CERODON
};
