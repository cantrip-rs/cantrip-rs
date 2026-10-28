use cantrip_api::*;
use cantrip_mechanics::{effects, sneak, targets, triggers};
use cantrip_oracle as oracle;

/// "Sneak {1}{B}{B}" (CR 702.190a): cast for {1}{B}{B} and an unblocked attacker you control returned to its owner's hand, in your
/// declare blockers step; "she enters tapped and attacking" the same player, planeswalker or battle as that creature (CR 702.190b),
/// which the core does for a permanent spell cast this way.
/// "When Elektra enters, destroy target creature an opponent controls with power 3 or less." (CR 701.8, 603.6a, 608.2b): the target
/// is chosen as the ability goes on the stack, and its power is judged again as the ability resolves, so a creature that has grown
/// since is no longer a legal target.
pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{1}{B}{B}"), "Sneak {1}{B}{B}")],
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::destroy_first_target,
        "When Elektra enters, destroy target creature an opponent controls with power 3 or less.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS_WITH_POWER_3_OR_LESS])],
    ..oracle::ELEKTRA_DAUGHTER_OF_THE_HAND
};
