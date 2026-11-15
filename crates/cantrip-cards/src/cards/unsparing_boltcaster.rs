use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::damage_first_target::<5>,
        "When this creature enters, it deals 5 damage to target creature an opponent controls that was dealt damage this turn.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS_THAT_WAS_DEALT_DAMAGE_THIS_TURN])],
    ..oracle::UNSPARING_BOLTCASTER
};
