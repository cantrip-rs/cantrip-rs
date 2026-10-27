use cantrip_api::*;
use cantrip_mechanics::{triggers, unearth};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
        unearth::ability(mana("{1}{B}"), "Unearth {1}{B}"),
    ],
    delayed: &[unearth::EXILE],
    ..oracle::VISCERA_DRAGGER
};
