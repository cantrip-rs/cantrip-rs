use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Tap up to one target creature and put a stun counter on it" (CR 122.1d): one already tapped just gets the counter.
fn tap_and_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tap_and_stun, "When this creature enters, tap up to one target creature and put a stun counter on it.")
        .targets(&[up_to_one(targets::CREATURE.filter)])],
    ..oracle::UTROM_SCIENTISTS
};
