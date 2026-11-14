use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_DIES};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, tokens::create_black_green_insects::<1>, "When this creature dies, create a 1/1 black and green Insect creature token with flying.")],
    ..oracle::INFESTATION_SAGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::INSECT_BLACK_GREEN_FLYING];
