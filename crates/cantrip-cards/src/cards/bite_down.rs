use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control deals damage equal to its power to target creature or planeswalker you don't control." (CR 120.3,
/// 609.7, 608.2b): one-sided, both targets judged as it resolves. "You don't control" is "an opponent controls" in a two-player
/// game, as Hard-Hitting Question words it.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS], fight::first_target_bites_second),
    ..oracle::BITE_DOWN
};
