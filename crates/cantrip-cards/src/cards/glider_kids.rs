use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, |g, item| effects::scry(g, item, 1), "When this creature enters, scry 1.")],
    ..oracle::GLIDER_KIDS
};
