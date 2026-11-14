use cantrip_api::*;
use cantrip_mechanics::triggers::{self, sacrificed_by};
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on it. Then if it has three or more +1/+1 counters on it, sacrifice this Aura." (CR 122.1, 701.21a):
/// the counter goes on the creature as it is now (nothing if it has gone), and the count is read after it is on.
fn counter_then_sacrifice_at_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    g.add_counters_together(&[(creature, CounterKind::PlusOne, 1)]);
    if g.state().obj(creature).counters(CounterKind::PlusOne) >= 3 {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "When you sacrifice this Aura" (CR 701.21a, 603.10a): this Aura is the permanent `player` sacrificed.
fn you_sacrifice_this_aura(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let sacrificed = sacrificed_by(q, q.controller(source), event)?;
    if sacrificed == source { Fired::PLAIN } else { None }
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(
            triggers::ATTACHED_CREATURE_ATTACKS_SUBJECT,
            counter_then_sacrifice_at_three,
            "Whenever enchanted creature attacks, put a +1/+1 counter on it. Then if it has three or more +1/+1 counters on it, sacrifice this Aura.",
        ),
        triggered(
            when(EventMask::ZONE_CHANGE, you_sacrifice_this_aura),
            library::search_for_two_basic_lands_onto_the_battlefield_tapped,
            "When you sacrifice this Aura, search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
        )
        .on_sacrifice(),
    ],
    ..oracle::ORDEAL_OF_NYLEA
};
