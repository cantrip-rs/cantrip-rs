use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{artifacts_you_control, effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::tap_and_stun::<1>,
        "When this creature enters, if you control an artifact, tap target artifact or creature an opponent controls and put a stun counter on it. (If a permanent with a stun counter would become untapped, remove one from it instead.)",
    )
    .condition(|q, asker| artifacts_you_control(q, asker.controller) >= 1)
    .targets(&[targets::ARTIFACT_OR_CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::WAYLAYING_PIRATES
};
