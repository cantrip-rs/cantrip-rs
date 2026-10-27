use cantrip_api::*;
use cantrip_mechanics::{enlist, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        enlist::ENLIST,
    ],
    ..oracle::HEXBANE_TORTOISE
};
