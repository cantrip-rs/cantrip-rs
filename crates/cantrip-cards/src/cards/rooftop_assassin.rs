use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Flash, flying and lifelink are core keywords. "When this creature enters, destroy target creature an opponent controls that
/// was dealt damage this turn." (CR 701.8a, 115.1, 608.2b): the target is judged as it is chosen and again as the ability resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::destroy_first_target,
        "When this creature enters, destroy target creature an opponent controls that was dealt damage this turn.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS_THAT_WAS_DEALT_DAMAGE_THIS_TURN])],
    ..oracle::ROOFTOP_ASSASSIN
};
