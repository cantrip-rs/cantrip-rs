use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Trample" (CR 702.19).
pub static CARD: CardDef = CardDef {
    abilities: &[
        block_allowance(1, "This creature can block an additional creature each combat."),
    ],
    ..oracle::TWO_HEADED_GIANT_OF_FORIYS
};
