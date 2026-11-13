use cantrip_api::*;
use cantrip_mechanics::{mutate, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[mutate::way(mana("{2}{W}{W}"), "Mutate {2}{W}{W}")],
    abilities: &[triggered(mutate::THIS_MUTATES, tokens::create_lifelink_cats::<2>, "Whenever this creature mutates, create two 1/1 white Cat creature tokens with lifelink.")],
    ..oracle::CUBWARDEN
};

pub static EXTRA: &[&CardDef] = &[&tokens::CAT_LIFELINK];
