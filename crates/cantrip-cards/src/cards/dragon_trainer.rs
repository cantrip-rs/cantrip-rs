use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_dragon_4_4, "When this creature enters, create a 4/4 red Dragon creature token with flying.")],
    ..oracle::DRAGON_TRAINER
};

pub static EXTRA: &[&CardDef] = &[&tokens::DRAGON_4_4];
