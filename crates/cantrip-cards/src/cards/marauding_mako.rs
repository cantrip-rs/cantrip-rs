use cantrip_api::*;
use cantrip_mechanics::triggers::cycling;
use cantrip_oracle as oracle;

/// "Put that many +1/+1 counters": the size of the batch the trigger saw, counted even for a card that has since left.
fn that_many_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let many = g.event_objects(item).len() as i16;
    g.add_counters(item.object, CounterKind::PlusOne, i64::from(many));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::YOU_DISCARD, that_many_counters, "Whenever you discard one or more cards, put that many +1/+1 counters on this creature.")
            .once_per_batch(),
        cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
    ],
    ..oracle::MARAUDING_MAKO
};
