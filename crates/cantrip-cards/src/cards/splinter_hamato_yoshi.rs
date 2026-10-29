use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::sneak;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_oracle as oracle;

/// "Other Ninjas you control get +1/+1." (CR 613.4c, layer 7c): Splinter is a Mutant Ninja Rat itself, so it is left out.
pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{B}"), "Sneak {B}")],
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Ninja) }>,
        plus_pt::<1, 1>,
        "Other Ninjas you control get +1/+1.",
    )],
    ..oracle::SPLINTER_HAMATO_YOSHI
};
