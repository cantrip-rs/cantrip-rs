use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::plus_one_counter_on_target, "When this creature enters, put a +1/+1 counter on another target creature you control.")
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::STERLING_SUPPLIER
};
