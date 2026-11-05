use cantrip_api::*;
use cantrip_oracle as oracle;

/// How many times counters have been put on `creature` this turn, as the history stands (CR 603.4's "first time counters have been
/// put on that creature this turn" is asked of it, whatever their kind). Counters removed from it are filed with them
/// (`EventMask::COUNTERS`) and are not counters put on it.
fn counters_put_on_this_turn(q: &dyn Query, creature: ObjectId) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::COUNTERS).this_turn().object(creature))
        .filter(|r| matches!(r.event, Event::CountersPut { .. }))
        .count()
}

/// "Whenever one or more counters are put on a creature you control, if it's the first time counters have been put on that creature
/// this turn" (CR 603.2, 603.4), a creature this ability's controller controls as the counters are put on it, entering with them
/// included (CR 122.6). A trigger sees the history as it was just before the counters were put, so the first time is when none are
/// recorded yet. Counters of several kinds put at once are one time: the ability triggers once for them (CR 603.2c), and once for
/// each creature they are put on. `data[0]` is the serial of the first of them.
fn counters_put_on_a_creature_you_control_first_time(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, .. }
            if q.chars(*object).is(CardType::Creature)
                && q.controller(*object) == me.controller
                && counters_put_on_this_turn(q, *object) == 0 =>
        {
            Some(Fired { subject: Target::Object(*object), data: [q.history().head(), object.0] })
        }
        _ => None,
    }
}

/// "Put a +1/+1 counter on that creature": the intervening "if" is checked again as the ability resolves (CR 603.4). The counters
/// that triggered it are the first recorded on it from `data[0]`, one kind after another with nothing between; counters put on it
/// after those are another time, and the ability does nothing. The counter this ability puts on is such a later time, so it does
/// not trigger it again. A creature that has left the battlefield is not put one (CR 400.7).
fn put_a_plus_one_counter_on_that_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let put_on_it = |r: &Record| matches!(r.event, Event::CountersPut { object, .. } if object == creature);
    let history = g.history();
    let since = history.all().get(item.data[0] as usize..).unwrap_or_default();
    let first = since.iter().position(put_on_it).unwrap_or(since.len());
    let that_time = first + since[first..].iter().take_while(|r| put_on_it(r)).count();
    let another_time = since[that_time..].iter().any(put_on_it);
    if g.is_on_battlefield(creature) && !another_time {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::COUNTERS, counters_put_on_a_creature_you_control_first_time),
        put_a_plus_one_counter_on_that_creature,
        "Whenever one or more counters are put on a creature you control, if it's the first time counters have been put on that creature this turn, put a +1/+1 counter on that creature.",
    )
    .once_per_batch()
    .batch_by_data()],
    ..oracle::STALWART_SUCCESSOR
};
