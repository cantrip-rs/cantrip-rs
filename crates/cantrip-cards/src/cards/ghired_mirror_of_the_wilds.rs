use cantrip_api::*;
use cantrip_mechanics::{copy, statics, targets};
use cantrip_oracle as oracle;

/// "Nontoken creatures you control" (CR 611.3a, 613.1f): `your_creatures` that are not tokens, Ghired among them, as the earlier
/// layers made them.
fn nontoken_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    !q.state().obj(affected).token && statics::your_creatures(q, source, affected, so_far)
}

/// "Target token you control that entered this turn" (CR 115.1, 400.7, 603.6a): a token on the battlefield, controlled by the
/// ability's controller, that is the object a move onto the battlefield made this turn.
fn token_you_control_that_entered_this_turn(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::TOKEN_YOU_CONTROL.filter)(q, asker, t) && t.object().is_some_and(|id| q.history().entered_this_turn(id))
}

/// "{T}: Create a token that's a copy of target token you control that entered this turn." (CR 707.2, 111.1): each nontoken
/// creature has its own, activated by its controller (CR 113.6, 602.1); the creature taps for it, so summoning sickness applies
/// (CR 302.6) - Ghired itself has haste.
pub static CARD: CardDef = CardDef {
    delayed: &[ability(
        TAP,
        copy::create_token_copy_of_target,
        "{T}: Create a token that's a copy of target token you control that entered this turn.",
    )
    .targets(&[target(token_you_control_that_entered_this_turn)])],
    statics: &[static_ability(
        Layer::Ability,
        nontoken_creatures_you_control,
        statics::grant_delayed_ability,
        "Nontoken creatures you control have \"{T}: Create a token that's a copy of target token you control that entered this turn.\"",
    )],
    ..oracle::GHIRED_MIRROR_OF_THE_WILDS
};
