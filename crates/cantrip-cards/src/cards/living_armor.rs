use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}, Sacrifice this artifact: Put X +0/+1 counters on target creature, where X is that creature's mana value." (CR 608.2, 122.1).
fn put_x_0_1_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let amount = g.mana_value(creature) as i32;
    g.add_counters(creature, CounterKind::Plus0Plus1, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                SACRIFICE_THIS.and(TAP),
                put_x_0_1_counters,
                "{T}, Sacrifice this artifact: Put X +0/+1 counters on target creature, where X is that creature's mana value.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::LIVING_ARMOR
};
