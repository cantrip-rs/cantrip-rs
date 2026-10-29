use cantrip_api::*;
use cantrip_mechanics::effects::destroy_first_target;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, destroy_first_target, "When this creature enters, destroy up to one target noncreature artifact.")
        .targets(&[targets::NONCREATURE_ARTIFACT.optional()])],
    ..oracle::ROCK_SOLDIERS
};
