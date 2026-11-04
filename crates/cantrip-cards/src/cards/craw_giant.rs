use cantrip_api::*;
use cantrip_mechanics::rampage;
use cantrip_oracle as oracle;

/// "Trample" (CR 702.1).
pub static CARD: CardDef = CardDef {
    abilities: &[rampage::trigger::<2>("Rampage 2")],
    ..oracle::CRAW_GIANT
};
