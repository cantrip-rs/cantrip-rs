use cantrip_api::*;
use cantrip_mechanics::statics::{artifacts_you_control, plus_pt};
use cantrip_oracle as oracle;

/// "As long as you control two or more artifacts, this creature gets +2/+0" (CR 613.4c): itself, while its controller has two or
/// more artifacts as the earlier layers left them, itself among them if it is one.
fn itself_with_two_artifacts(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    affected == source && artifacts_you_control(view, source) >= 2
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, itself_with_two_artifacts, plus_pt::<2, 0>, "As long as you control two or more artifacts, this creature gets +2/+0.")],
    ..oracle::GAELICAT
};
