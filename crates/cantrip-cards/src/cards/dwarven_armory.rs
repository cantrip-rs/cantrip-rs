use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{2}, Sacrifice a land: Put a +2/+2 counter on target creature. Activate only during any upkeep step." (CR 608.2, 122.1).
fn put_2_2_counter_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(target, CounterKind::Plus2Plus2, 1);
    Ok(())
}

/// "{2}, Sacrifice a land: Put a +2/+2 counter on target creature. Activate only during any upkeep step." (CR 602.5).
fn put_2_2_counter_target_if(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::Upkeep
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(sacrifice(1, targets::filters::land)),
                put_2_2_counter_target,
                "{2}, Sacrifice a land: Put a +2/+2 counter on target creature. Activate only during any upkeep step.",
            )
            .targets(&[targets::CREATURE])
            .condition(put_2_2_counter_target_if),
    ],
    ..oracle::DWARVEN_ARMORY
};
