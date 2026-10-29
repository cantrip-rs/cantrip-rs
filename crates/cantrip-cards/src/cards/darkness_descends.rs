use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Put two -1/-1 counters on each creature" (CR 122.1a): every creature on the battlefield as it resolves, the counters
/// put at once, so a creature that dies of them dies in the same state-based check as the rest.
fn two_counters_on_each_creature(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let puts: Vec<(ObjectId, CounterKind, i16)> =
        g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).map(|id| (id, CounterKind::MinusOne, 2)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], two_counters_on_each_creature), ..oracle::DARKNESS_DESCENDS };
