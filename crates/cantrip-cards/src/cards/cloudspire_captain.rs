use cantrip_api::*;
use cantrip_mechanics::statics::{mounts_and_vehicles_you_control, plus_pt};
use cantrip_oracle as oracle;

/// "This creature saddles Mounts and crews Vehicles as though its power were 2 greater" (CR 702.171c, 702.122b) are the generated
/// keywords `SaddlesAsThoughPowerTwoGreater` and `CrewsAsThoughPowerTwoGreater`, which the crew and saddle costs read.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, mounts_and_vehicles_you_control, plus_pt::<1, 1>, "Mounts and Vehicles you control get +1/+1.")],
    ..oracle::CLOUDSPIRE_CAPTAIN
};
