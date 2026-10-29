use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, effects::minus_one_counters_on_target::<1>, "When this creature dies, put a -1/-1 counter on up to one target creature.")
        .targets(&[targets::CREATURE.optional()])],
    ..oracle::BILE_VIAL_BOGGART
};
