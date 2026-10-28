use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_treasure, "When Dori enters, create a Treasure token.")],
    ..oracle::DORI_BEARER_OF_FRIENDS
};
