use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control deals damage equal to its power to target creature you don't control." (CR 120.3, 609.7, 608.2b):
/// one-sided, both targets judged as it resolves, so with either illegal then no damage is dealt (the 2020-09-25 ruling).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL], fight::first_target_bites_second),
    ..oracle::RABID_BITE
};
