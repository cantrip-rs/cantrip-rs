use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::itself;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself,
        lands::plus_per_subtype_you_control::<{ lands::sub(Subtype::Island) }, true, 1, 0>,
        "This creature gets +1/+0 for each basic Island you control.",
    )],
    ..oracle::TEMPEST_DJINN
};
