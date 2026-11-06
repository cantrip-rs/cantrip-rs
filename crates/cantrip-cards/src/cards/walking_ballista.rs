use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// {4}: Put a +1/+1 counter on this creature.
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) && g.is_on_battlefield(item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    abilities: &[
        ability(cost("{4}"), grow, "{4}: Put a +1/+1 counter on this creature."),
        effects::REMOVE_PLUS_ONE_TO_DEAL_ONE,
    ],
    ..oracle::WALKING_BALLISTA
};
