use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Put a +1/+2 counter on target creature." (CR 608.2b, 122.1a).
fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::Plus1Plus2, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(SACRIFICE_THIS), put_counter,
        "{T}, Sacrifice this creature: Put a +1/+2 counter on target creature.")
        .targets(&[targets::CREATURE])],
    ..oracle::ARMOR_THRULL
};
