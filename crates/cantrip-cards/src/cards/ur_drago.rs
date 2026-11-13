use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "Creatures with swampwalk can be blocked as though they didn't have swampwalk." (CR 702.14, 509.1b)
    landwalk_exceptions: &[lands::named_landwalk_exception::<{ lands::sub(Subtype::Swamp) }>],
    ..oracle::UR_DRAGO
};
