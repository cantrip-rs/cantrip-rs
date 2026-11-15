use cantrip_api::*;
use cantrip_mechanics::{manifest_dread::controller_manifests_dread, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, controller_manifests_dread, "When this creature dies, manifest dread.")],
    ..oracle::BASHFUL_BEASTIE
};
