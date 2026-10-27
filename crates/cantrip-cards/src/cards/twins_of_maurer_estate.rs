use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    madness: Some(mana("{2}{B}")),
    ..oracle::TWINS_OF_MAURER_ESTATE
};
