use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Another target creature you control gets +1/+0 and gains indestructible until end of turn" (CR 611.2a): the ability's first
/// target, one instruction; nothing if it is no longer a legal target as the ability resolves (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::target_creature_gets_and_gains_indestructible::<1, 0>,
        "Whenever this creature attacks, another target creature you control gets +1/+0 and gains indestructible until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::HARDENED_ESCORT
};
