use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "When this creature enters, return up to one other target creature to its owner's hand. If you controlled it, investigate."
/// (CR 603.6a, 400.3, 701.16a, 608.2h): whether you controlled the creature is read as the ability resolves, before it is returned
/// (`effects::return_first_target_to_hand_was_yours`); with no target, or one no longer legal, there is no Clue (CR 608.2b, the rulings).
fn return_and_investigate_if_yours(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::return_first_target_to_hand_was_yours(g, item)? {
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        return_and_investigate_if_yours,
        "When this creature enters, return up to one other target creature to its owner's hand. If you controlled it, investigate.",
    )
    .targets(&[up_to_one(targets::other_creature_filter)])],
    ..oracle::HOTSHOT_INVESTIGATORS
};
