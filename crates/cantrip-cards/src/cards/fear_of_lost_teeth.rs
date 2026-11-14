use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        |g, item| effects::damage_target_and_gain_life(g, item, 1, 1),
        "When this creature dies, it deals 1 damage to any target and you gain 1 life.",
    )
    .targets(&[targets::ANY])],
    ..oracle::FEAR_OF_LOST_TEETH
};
