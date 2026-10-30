use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, |g, item| effects::surveil(g, item, 1), "When this creature enters, surveil 1."),
        triggered(triggers::THIS_DIES, |g, item| effects::surveil(g, item, 1), "When this creature dies, surveil 1."),
    ],
    ..oracle::LYS_ALANA_INFORMANT
};
