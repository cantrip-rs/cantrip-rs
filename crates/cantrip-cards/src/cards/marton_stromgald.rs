use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Other creatures currently in the relevant combat role each get +1/+1 for each of them (CR 608.2, 611.2a).
fn pump_others(g: &mut dyn Game, item: &StackItem, attacking: bool) {
    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| {
        id != item.object && g.chars(id).is(CardType::Creature)
            && if attacking { g.is_attacking(id) } else { g.is_blocking(id) }
    }).collect();
    let count = creatures.len() as i32;
    instruction(g, |g| {
        for &id in &creatures {
            g.add_effect(item.object, id, EffectKind::Pump(count, count), Duration::EndOfTurn);
        }
    });
}

fn attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_others(g, item, true);
    Ok(())
}

fn block(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_others(g, item, false);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, attack, "Whenever Márton Stromgald attacks, other attacking creatures get +1/+1 until end of turn for each attacking creature other than Márton Stromgald."),
        triggered(triggers::THIS_BLOCKS, block, "Whenever Márton Stromgald blocks, other blocking creatures get +1/+1 until end of turn for each blocking creature other than Márton Stromgald."),
    ],
    ..oracle::M_RTON_STROMGALD
};
