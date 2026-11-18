use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_TURNED_FACE_UP;
use cantrip_mechanics::{disguise, tokens};
use cantrip_oracle as oracle;

/// "When this creature enters or is turned face up, create a 1/1 colorless Thopter artifact creature token with flying" (CR 603.6a,
/// 701.40, 702.168). Disguise {U/R}{U/R} is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_TURNED_FACE_UP,
        tokens::create_a_thopter,
        "When this creature enters or is turned face up, create a 1/1 colorless Thopter artifact creature token with flying.",
    )],
    ways: &[disguise::way(Cost::mana(mana("{U/R}{U/R}")), "Disguise {U/R}{U/R}")],
    ..oracle::GADGET_TECHNICIAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::THOPTER];
