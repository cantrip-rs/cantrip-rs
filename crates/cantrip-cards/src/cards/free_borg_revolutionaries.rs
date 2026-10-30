use cantrip_api::*;
use cantrip_mechanics::{effects, typecycling, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::creatures_you_control_get::<1, 1>, "When this creature enters, creatures you control get +1/+1 until end of turn."),
        typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2} ({2}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::FREE_BORG_REVOLUTIONARIES
};
