use cantrip_api::*;
use cantrip_mechanics::{explore, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, |g, item| explore(g, item.object), "When this creature enters, it explores.")],
    ..oracle::RIVER_HERALD_GUIDE
};
