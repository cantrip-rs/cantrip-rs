use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS_OR_DIES, |g, item| effects::surveil(g, item, 1), "When this creature enters or dies, surveil 1.")],
    ..oracle::WARY_WATCHDOG
};
