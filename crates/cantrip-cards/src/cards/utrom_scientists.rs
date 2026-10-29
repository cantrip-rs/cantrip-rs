use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::tap_and_stun::<1>, "When this creature enters, tap up to one target creature and put a stun counter on it.")
        .targets(&[up_to_one(targets::CREATURE.filter)])],
    ..oracle::UTROM_SCIENTISTS
};
