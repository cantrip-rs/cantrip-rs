use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        statics::plus_per_other_subtype_you_control::<{ lands::sub(Subtype::Goblin) }, 2, 0>,
        "This creature gets +2/+0 for each other Goblin you control.",
    )],
    ..oracle::BONECLUB_BERSERKER
};
