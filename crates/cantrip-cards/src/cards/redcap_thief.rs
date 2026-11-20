use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_treasure, "When this creature enters, create a Treasure token.")],
    ..oracle::REDCAP_THIEF
};
