use cantrip_api::*;
use cantrip_mechanics::{crew, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        crew::crew(2, "Crew 2"),
        triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
    ],
    ..oracle::SKYBOX_FERRY
};
