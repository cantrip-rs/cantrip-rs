use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

/// The same effect grants islandwalk in layer 6 and +1/+1 in layer 7c (CR 613.6); it affects other Merfolk of either player.
const TEXT: &str = "Other Merfolk get +1/+1 and have islandwalk.";

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, statics::other_of_subtype::<{ lands::sub(Subtype::Merfolk) }>, statics::have_landwalk::<{ lands::sub(Subtype::Island) }>, TEXT),
        static_ability(Layer::PtModify, statics::other_of_subtype::<{ lands::sub(Subtype::Merfolk) }>, statics::plus_pt::<1, 1>, TEXT).continues(),
    ],
    ..oracle::LORD_OF_ATLANTIS
};
