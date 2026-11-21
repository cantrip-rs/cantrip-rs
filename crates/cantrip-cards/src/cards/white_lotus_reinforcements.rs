use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

/// "Other Allies you control get +1/+1" (CR 613.4c, layer 7c): Ally creatures under its controller's control, not the Reinforcements
/// themselves. Vigilance is a core keyword read from the card's characteristics.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Ally) }>,
        statics::plus_pt::<1, 1>,
        "Other Allies you control get +1/+1.",
    )],
    ..oracle::WHITE_LOTUS_REINFORCEMENTS
};
