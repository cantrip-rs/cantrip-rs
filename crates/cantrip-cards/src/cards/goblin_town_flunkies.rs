use cantrip_api::*;
use cantrip_mechanics::amass::amass_goblins;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, amass_goblins::<1>, "When this creature enters, amass Goblins 1.")],
    ..oracle::GOBLIN_TOWN_FLUNKIES
};
