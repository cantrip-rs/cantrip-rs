use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // At each end step, the ability's controller sacrifices this creature (CR 603.2, 701.21a).
    abilities: &[triggered(
        triggers::EACH_END_STEP,
        effects::sacrifice_self,
        "At the beginning of the end step, sacrifice this creature.",
    )],
    ..oracle::BALL_LIGHTNING
};
