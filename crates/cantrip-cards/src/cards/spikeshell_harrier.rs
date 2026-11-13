use cantrip_api::*;
use cantrip_mechanics::{start_your_engines, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, return target creature or Vehicle an opponent controls to its owner's hand. If that opponent's
/// speed is greater than each other player's speed, reduce that opponent's speed by 1. This effect can't reduce their speed below
/// 1." (CR 400.3, 702.179): "that opponent" is the target's controller, read before it leaves the battlefield (a permanent in the
/// hand has no controller); a player with no speed counts as 0 (CR 702.179f), and ties reduce nothing.
fn bounce_and_slow_its_controller(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let opponent = g.controller(permanent);
    g.return_to_hand(&[permanent]);
    g.end_sentence();
    let state = g.state();
    let theirs = start_your_engines::speed(state, opponent);
    if state.apnap().filter(|&p| p != opponent).all(|p| start_your_engines::speed(state, p) < theirs) {
        g.reduce_speed(opponent, 1, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        bounce_and_slow_its_controller,
        "When this creature enters, return target creature or Vehicle an opponent controls to its owner's hand. If that opponent's speed is greater than each other player's speed, reduce that opponent's speed by 1. This effect can't reduce their speed below 1.",
    )
    .targets(&[targets::CREATURE_OR_VEHICLE_AN_OPPONENT_CONTROLS])],
    ..oracle::SPIKESHELL_HARRIER
};
