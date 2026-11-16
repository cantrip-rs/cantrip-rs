use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS_OR_DIES};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_OR_DIES, tokens::create_a_food_token, "When this creature enters or dies, create a Food token.")],
    ..oracle::VINEREAP_MENTOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
