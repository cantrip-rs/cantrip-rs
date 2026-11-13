use cantrip_api::*;
use cantrip_mechanics::{crew, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        crew::crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
        triggers::cycling(mana("{R}"), "Cycling {R} ({R}, Discard this card: Draw a card.)"),
    ],
    ..oracle::CLAMOROUS_IRONCLAD
};
