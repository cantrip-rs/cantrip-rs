use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::other_creatures_get::<-2, -2>, "When this creature enters, all other creatures get -2/-2 until end of turn."),
        triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
    ],
    ..oracle::SHEFET_ARCHFIEND
};
