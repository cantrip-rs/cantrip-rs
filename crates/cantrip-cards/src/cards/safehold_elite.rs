use cantrip_api::*;
use cantrip_mechanics::persist;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        persist::trigger("Persist (When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)"),
    ],
    ..oracle::SAFEHOLD_ELITE
};
