use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// The paid creature card's first exile incarnation supplies X, even if it has since left exile (CR 608.2h, 202.3e).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.history().ability_cost_objects(item.serial, 0, CostPart::GraveyardExile)
        .next()
        .and_then(|paid| g.events().iter().find_map(|record| match record.event {
            Event::ZoneChange { old, new, to, .. } if old == paid && to.is(ZoneKind::Exile) => Some(new),
            _ => None,
        }));
    // This is a physical card's mana value in exile, where X is zero (CR 202.3e), never a melded permanent or
    // stack spell. `physical_off_stack_mana_value_bound` verifies every registered card face fits the counter API.
    let value = exiled.map_or(0, |card| g.mana_value(card));
    let count = value as i32;
    g.add_counters(item.object, CounterKind::Plus0Plus1, count);
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
