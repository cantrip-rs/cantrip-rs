use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of combat on your turn, target creature you control with power 2 or less gets +1/+0 and gains deathtouch
/// until end of turn" (CR 603.2, 611.2a): the target's power is checked as the ability is put on the stack and again as it
/// resolves (CR 608.2b), when it may have grown.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        effects::target_creature_gets_and_gains_deathtouch::<1, 0>,
        "At the beginning of combat on your turn, target creature you control with power 2 or less gets +1/+0 and gains deathtouch until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL_WITH_POWER_2_OR_LESS])],
    ..oracle::SLIMY_DUALLEECH
};
