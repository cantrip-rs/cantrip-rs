use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::surveil(g, item, 2), "When this creature enters, surveil 2.")],
    ..oracle::A_I_M_SYNTHOIDS
};
