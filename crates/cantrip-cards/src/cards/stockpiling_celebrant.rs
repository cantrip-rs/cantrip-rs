use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "You may return another target nonland permanent you control to its owner's hand. If you do, scry 2." The target is
/// mandatory (the ability is removed from the stack without one) and chosen as it goes on the stack; the "may" is asked as it
/// resolves, and only if the target is still legal (CR 608.2b). The scry follows only if the permanent really arrived in a
/// hand (CR 608.2c; a token that is returned ceases to exist after it moved, CR 111.7).
fn may_return_target_then_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    if effects::ask_may(g, item.controller, item.object, Purpose::ReturnToHand)? && !g.return_to_hand(&[permanent]).is_empty() {
        effects::scry(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        may_return_target_then_scry,
        "When this creature enters, you may return another target nonland permanent you control to its owner's hand. If you do, scry 2.",
    )
    .targets(&[target(targets::other_nonland_permanent_you_control_filter)])],
    ..oracle::STOCKPILING_CELEBRANT
};
