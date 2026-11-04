use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, |g, item| effects::surveil(g, item, 1), "Whenever this creature attacks, surveil 1.")],
    ..oracle::IL_MHEG_PIXIE
};
