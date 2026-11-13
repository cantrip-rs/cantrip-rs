use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +4/+4 and gains trample until end of turn" (CR 611.2a, 702.19): one instruction, to any creature.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_gains_trample::<4, 4>),
    ..oracle::BESTOW_GREATNESS
};
