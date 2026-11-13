use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    landwalk_exceptions: &[lands::named_landwalk_exception::<{ lands::sub(Subtype::Mountain) }>],
    ..oracle::CREVASSE
};
