use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, target creature you control gains hexproof until end of turn" (CR 702.11b, 611.2a): the target
/// may be the Escort itself. It does nothing if the target is no longer legal (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::first_target_gains_hexproof,
        "When this creature enters, target creature you control gains hexproof until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::PLUMECREED_ESCORT
};
