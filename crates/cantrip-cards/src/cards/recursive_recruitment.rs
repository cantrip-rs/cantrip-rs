use cantrip_api::*;
use cantrip_mechanics::{flashback, tokens};
use cantrip_oracle as oracle;

/// "Create two 2/2 colorless Wizard Soldier creature tokens named Cadet. If this spell was cast from a graveyard, put a
/// +1/+1 counter on each of them for every three cards in your graveyard." The zone it was cast from is the spell's own
/// (CR 601.2a); the graveyard is counted as the spell resolves (CR 608.2h), the spell itself still on the stack.
fn recruit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cadets = tokens::create(g, item.controller, &tokens::CADET, 2)?;
    if g.obj(item.object).cast_from == Some(ZoneKind::Graveyard) {
        let counters = (g.state().zone(Zone::graveyard(item.controller)).len() / 3) as i16;
        let puts: Vec<(ObjectId, CounterKind, i64)> = cadets.into_iter().map(|cadet| (cadet, CounterKind::PlusOne, i64::from(counters))).collect();
        g.add_counters_together(&puts);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], recruit),
    ways: &[flashback::way(cost("{6}{U}{B}"), "Flashback {6}{U}{B}")],
    ..oracle::RECURSIVE_RECRUITMENT
};

pub static EXTRA: &[&CardDef] = &[&tokens::CADET];
