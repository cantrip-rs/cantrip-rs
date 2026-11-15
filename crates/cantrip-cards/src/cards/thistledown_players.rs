use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, untap target nonland permanent." (CR 701.26b): the target may be any nonland permanent, tapped or
/// not, the attacker itself and an opponent's included (CR 115.1); a target that is already untapped is untouched.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, effects::untap_targets, "Whenever this creature attacks, untap target nonland permanent.")
        .targets(&[targets::NONLAND_PERMANENT])],
    ..oracle::THISTLEDOWN_PLAYERS
};
