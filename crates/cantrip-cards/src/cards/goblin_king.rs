use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

const TEXT: &str = "Other Goblins get +1/+1 and have mountainwalk.";

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, statics::other_of_subtype::<{ lands::sub(Subtype::Goblin) }>, statics::have_landwalk::<{ lands::sub(Subtype::Mountain) }>, TEXT),
        static_ability(Layer::PtModify, statics::other_of_subtype::<{ lands::sub(Subtype::Goblin) }>, statics::plus_pt_saturating::<1, 1>, TEXT).continues(),
    ],
    ..oracle::GOBLIN_KING
};
