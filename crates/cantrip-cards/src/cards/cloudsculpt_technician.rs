use cantrip_api::*;
use cantrip_mechanics::statics::{itself_while_you_control_an_artifact, plus_pt};
use cantrip_oracle as oracle;

/// Flying is a core keyword read from its characteristics. "As long as you control an artifact, this creature gets +1/+0" (CR 604.1,
/// 613.4c): a static ability that applies only while its controller controls an artifact.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_while_you_control_an_artifact,
        plus_pt::<1, 0>,
        "As long as you control an artifact, this creature gets +1/+0.",
    )],
    ..oracle::CLOUDSCULPT_TECHNICIAN
};
