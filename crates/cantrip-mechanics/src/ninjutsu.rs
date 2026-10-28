//! Ninjutsu (CR 702.49), the ability of a card in hand that swaps it for an unblocked attacker.

use cantrip_api::*;

use crate::combat::unblocked_attacker_you_control;

/// "Put this card onto the battlefield from your hand tapped and attacking": attacking what the returned creature was
/// (the cost left that in the ability's `subject`), without having been declared as an attacker (CR 508.4). A card that
/// has left the hand since does nothing.
fn put_onto_battlefield_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !cantrip_api::is_live(g, item.object) || !g.obj(item.object).zone.is(ZoneKind::Hand) {
        return Ok(());
    }
    if item.subject == Target::None {
        g.put_onto_battlefield(item.object, item.controller, true)?;
    } else {
        g.put_onto_battlefield_attacking(item.object, item.controller, item.subject, &[])?;
    }
    Ok(())
}

/// Ninjutsu `cost` (CR 702.49a): "{cost}, Return an unblocked attacker you control to its owner's hand: Put this card
/// onto the battlefield from your hand tapped and attacking." Kaito, Bane of Nightmares and Fallen Shinobi print it;
/// `text` is the ability's full text, which a scenario's `Ability:` matches.
pub const fn ninjutsu(cost: ManaCost, text: &'static str) -> AbilityDef {
    ability(Cost::mana(cost).and(return_to_hand(1, unblocked_attacker_you_control)), put_onto_battlefield_attacking, text)
        .from(ZoneKind::Hand)
}
