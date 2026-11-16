use cantrip_api::*;
use cantrip_mechanics::{saddle, targets, triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER};
use cantrip_oracle as oracle;

/// "Target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana
/// cost. That card gains flashback {0} until end of turn instead if this creature is saddled" (CR 702.34a, 702.171b): its
/// controller may cast it from the graveyard this turn for its mana cost, or for {0} when the Newt is saddled, and it is exiled
/// instead of going anywhere else as it leaves the stack. Saddled is read as the Newt is as the ability resolves, or as it last was
/// if it has left the battlefield. Nothing if the card has left the graveyard (CR 608.2b).
fn gain_flashback(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let instead = g.is_saddled(item.object).then_some(ManaCost::ZERO);
    g.allow_play(card, item.controller, PlayGrant { instead, expiry: Expiry::EndOfTurn, flashback: true, ..PlayGrant::CAST });
    Ok(())
}
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER,
            gain_flashback,
            "Whenever this creature deals combat damage to a player, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. That card gains flashback {0} until end of turn instead if this creature is saddled.",
        )
        .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD]),
        saddle::ability(3, "Saddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::ARCHMAGES_NEWT
};
