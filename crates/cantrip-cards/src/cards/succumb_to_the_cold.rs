use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Tap one or two target creatures an opponent controls. Put a stun counter on each of them." The targets still legal as it
/// resolves are tapped together and each gets its counter (`effects::tap_and_stun_each_target`).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_AN_OPPONENT_CONTROLS.between(1, 2)], effects::tap_and_stun_each_target),
    ..oracle::SUCCUMB_TO_THE_COLD
};
