use cantrip_api::*;
use cantrip_mechanics::{effects, forage, targets, triggers};
use cantrip_oracle as oracle;

/// "You may forage. When you do, return target permanent card from your graveyard to your hand" (CR 701.61a, 603.12): the reflexive
/// trigger is queued once the forage was paid (so the three exiled cards are no longer targetable), its target chosen as it goes on the
/// stack, and with no permanent card left in the graveyard it is removed from the stack (CR 603.3d).
fn may_forage_then_reflexive_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if forage::may_forage(g, item)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), item.subject);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        may_forage_then_reflexive_return,
        "When this creature enters, you may forage. When you do, return target permanent card from your graveyard to your hand.",
    )],
    delayed: &[delayed_trigger(
        effects::return_first_target_to_hand,
        "When you do, return target permanent card from your graveyard to your hand.",
    )
    .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::CURIOUS_FORAGER
};
