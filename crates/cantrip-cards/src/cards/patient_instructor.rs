use cantrip_api::*;
use cantrip_mechanics::{recruit, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, recruit::effect, "When this creature enters, recruit.")],
    ..oracle::PATIENT_INSTRUCTOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
