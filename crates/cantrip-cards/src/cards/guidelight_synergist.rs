use cantrip_api::*;
use cantrip_mechanics::statics::{itself, plus_per_artifact_you_control};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This creature gets +1/+0 for each artifact you control." (CR 613.4c): it is an artifact itself, so it counts itself.
    statics: &[static_ability(Layer::PtModify, itself, plus_per_artifact_you_control::<1, 0>, "This creature gets +1/+0 for each artifact you control.")],
    ..oracle::GUIDELIGHT_SYNERGIST
};
