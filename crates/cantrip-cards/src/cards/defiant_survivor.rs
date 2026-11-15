use cantrip_api::*;
use cantrip_mechanics::{manifest_dread::controller_manifests_dread, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::survival(
        controller_manifests_dread,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, manifest dread.",
    )],
    ..oracle::DEFIANT_SURVIVOR
};
