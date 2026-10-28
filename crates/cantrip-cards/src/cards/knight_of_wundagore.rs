use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_oracle as oracle;

/// "Whenever you put a +1/+1 counter on another creature" (CR 122.6, 122.6a, 603.2): +1/+1 counters put on a creature other than
/// this one, on the battlefield or as it enters (CR 122.6), by this creature's controller - the player who put them is the event's,
/// so counters an opponent puts on a creature do not trigger it. One trigger for each time counters are put, however many.
fn you_put_a_counter_on_another_creature(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, kind: CounterKind::PlusOne, amount, by, .. }
            if *object != source && *amount > 0 && *by == q.controller(source) && q.is_on_battlefield(*object) && q.chars(*object).is(CardType::Creature) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// Trample is a core keyword. "... put a +1/+1 counter on this creature. This ability triggers only once each turn." (CR 122.1,
/// 603.2h)
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::COUNTERS, you_put_a_counter_on_another_creature),
        plus_one_counter_on_this,
        "Whenever you put a +1/+1 counter on another creature, put a +1/+1 counter on this creature. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::KNIGHT_OF_WUNDAGORE
};
