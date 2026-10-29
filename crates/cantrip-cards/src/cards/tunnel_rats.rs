use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Return this card from your graveyard to the battlefield tapped" (CR 400.7): the ability's own source, put onto the
/// battlefield tapped under its controller; nothing if it is no longer the object the ability came from (the core skips a
/// card that changed zones).
fn return_this_card_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.put_onto_battlefield(item.object, item.controller, true)?;
    Ok(())
}

/// The return works only from the graveyard (CR 113.6m), where the card is not a creature.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{4}{B}"), return_this_card_tapped, "{4}{B}: Return this card from your graveyard to the battlefield tapped.")
        .from(ZoneKind::Graveyard)],
    ..oracle::TUNNEL_RATS
};
