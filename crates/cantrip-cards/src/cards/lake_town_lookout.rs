use cantrip_api::*;
use cantrip_mechanics::{recruit, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, recruit::effect, "When this creature dies, recruit.")],
    ..oracle::LAKE_TOWN_LOOKOUT
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
