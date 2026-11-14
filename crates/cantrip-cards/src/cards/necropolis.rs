use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// The paid creature card's first exile incarnation supplies X, even if it has since left exile (CR 608.2h, 202.3e).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The exile that paid the cost, read from the history (a replacement may have sent the card elsewhere, which exiled none).
    let exiled = g.history().ability_cost_objects(item.serial, 0, CostPart::GraveyardExile).next().and_then(|paid| {
        let exile = EventQuery::kind(EventMask::ZONE_CHANGE).object(paid).to(ZoneKind::Exile);
        g.history().select(exile).find_map(|r| if let Event::ZoneChange { new, .. } = r.event { Some(new) } else { None })
    });
    // The card in exile, where X in its cost is zero (CR 202.3e).
    let value = exiled.map_or(0, |card| g.mana_value(card));
    g.add_counters(item.object, CounterKind::Plus0Plus1, value as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        exile_from_graveyard_of(1, filters::creature),
        resolve,
        "Exile a creature card from your graveyard: Put X +0/+1 counters on this creature, where X is the exiled card's mana value.",
    )],
    ..oracle::NECROPOLIS
};
