use cantrip_api::*;
use cantrip_oracle as oracle;

/// All landwalk abilities are ignored for blocking, without removing them (CR 609.4, 702.14c).
fn ignore_landwalk(_: &dyn Query, _: &Asker, _: ObjectId, _: ObjectId) -> Keywords {
    Keywords::LANDWALKS
}

pub static CARD: CardDef = CardDef {
    landwalk_exceptions: &[ignore_landwalk],
    ..oracle::STAFF_OF_THE_AGES
};
