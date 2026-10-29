use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_effect, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, exile_target_until_leaves_effect, "When this creature enters, exile target tapped creature an opponent controls until this creature leaves the battlefield.")
        .targets(&[targets::TAPPED_CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::HENCHBOTS
};
