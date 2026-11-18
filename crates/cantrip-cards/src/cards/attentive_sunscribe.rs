use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_BECOMES_TAPPED, |g, item| effects::scry(g, item, 1), "Whenever this creature becomes tapped, scry 1.")],
    ..oracle::ATTENTIVE_SUNSCRIBE
};
