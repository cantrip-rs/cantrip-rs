use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    madness: Some(mana("{B}")),
    ..oracle::KITCHEN_IMP
};
