use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_DIES,
            effects::add_colorless::<4>,
            "When this creature dies, add {C}{C}{C}{C}.",
        ),
    ],
    ..oracle::SU_CHI
};
