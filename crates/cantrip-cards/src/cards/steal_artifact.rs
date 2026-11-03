use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[control_changing(attach::attached_object, "You control enchanted artifact.")],
    enchant: Some(targets::ARTIFACT),
    ..oracle::STEAL_ARTIFACT
};
