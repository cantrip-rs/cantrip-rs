use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;


pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, effects::this_connives, "When this creature enters, it connives.")],
    ..oracle::RED_ROOM_RECRUIT
};
