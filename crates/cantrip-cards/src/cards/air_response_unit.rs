use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_oracle as oracle;

/// "Flying, vigilance" are the generated keywords; "Crew 1" (CR 702.122a) is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)")],
    ..oracle::AIR_RESPONSE_UNIT
};
