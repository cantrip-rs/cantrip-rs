use cantrip_api::*;
use cantrip_mechanics::amass::amass_goblins;
use cantrip_mechanics::{tokens, triggers::YOU_ATTACK};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_ATTACK, amass_goblins::<2>, "Whenever you attack, amass Goblins 2.")],
    ..oracle::MISTY_MOUNTAINS_RAIDER
};
