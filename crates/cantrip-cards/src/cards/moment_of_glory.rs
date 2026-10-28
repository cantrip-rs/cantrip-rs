use cantrip_api::*;
use cantrip_mechanics::{flashback, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature you control. If this spell was cast from a graveyard, also put a +1/+1 counter on
/// each other creature you control." The zone it was cast from is the spell's own (CR 601.2a), by flashback or any other
/// permission; a copy was never cast (CR 707.10). Nothing happens to the target if it is no longer legal (CR 608.2b); the
/// other creatures are those its controller controls as the spell resolves, and each gets its counter (CR 122.6).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = targets::object(g, item, 0);
    if let Some(creature) = target {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    g.end_sentence();
    if g.obj(item.object).cast_from == Some(ZoneKind::Graveyard) {
        let others: Vec<(ObjectId, CounterKind, i16)> = g
            .permanents_of(item.controller)
            .into_iter()
            .filter(|&id| Some(id) != target && g.chars(id).is(CardType::Creature))
            .map(|id| (id, CounterKind::PlusOne, 1))
            .collect();
        g.add_counters_together(&others);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effect),
    ways: &[flashback::way(cost("{4}{W}"), "Flashback {4}{W}")],
    ..oracle::MOMENT_OF_GLORY
};
