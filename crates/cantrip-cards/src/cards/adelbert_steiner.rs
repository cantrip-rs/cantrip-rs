use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        lands::plus_per_subtype_you_control::<{ lands::sub(Subtype::Equipment) }, false, 1, 1>,
        "Adelbert Steiner gets +1/+1 for each Equipment you control.",
    )],
    ..oracle::ADELBERT_STEINER
};
