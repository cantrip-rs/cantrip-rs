use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, tokens::create_zombies::<2>, "When this creature dies, create two 2/2 black Zombie creature tokens.")],
    ..oracle::MAALFELD_TWINS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE];
