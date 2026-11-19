use cantrip_api::*;
use cantrip_mechanics::double;
use cantrip_oracle as oracle;

/// "Whenever you untap one or more permanents during your untap step" (CR 502.3, 701.26b, 603.2c): each permanent of the
/// Calendar's controller that becomes untapped in the untap step of a turn of theirs is an event of the one simultaneous
/// batch (the turn-based action untaps them all at once), and the trigger is about each of them. An opponent's permanent that
/// untaps in that step by its own effect (Unwinding Clock) is untapped by its controller, not by you, and is not counted.
fn you_untap_a_permanent_during_your_untap_step(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Untapped { permanent } = event else { return None };
    let turn = q.state().turn;
    if turn.active != me.controller || turn.step() != Step::Untap || q.controller(*permanent) != me.controller {
        return None;
    }
    Fired::about(Target::Object(*permanent))
}

/// "... put that many time counters on The Millennium Calendar": one for each permanent of the batch (CR 603.2c), counted
/// even for one that has since left the battlefield.
fn that_many_time_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let many = g.event_objects(item).len() as i32;
    g.add_counters(item.object, CounterKind::Time, many);
    Ok(())
}

/// "Double the number of time counters on The Millennium Calendar" (CR 701.10e).
fn double_the_time_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    double::counters(g, &[item.object], CounterKind::Time);
    Ok(())
}

/// "When there are 1,000 or more time counters on The Millennium Calendar": a state trigger (CR 603.8), true of the Calendar itself.
fn a_thousand_time_counters(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::Time) >= 1000
}

/// "... sacrifice it and each opponent loses 1,000 life." Only the Calendar that triggered it can be sacrificed: once it has
/// left the battlefield it is a new object (CR 400.7); each opponent loses the life all the same.
fn sacrifice_it_and_each_opponent_loses_1000_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(item.controller, &[item.object]);
    g.each_opponent_loses_life(item.controller, 1000);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::UNTAPPED, you_untap_a_permanent_during_your_untap_step),
            that_many_time_counters,
            "Whenever you untap one or more permanents during your untap step, put that many time counters on The Millennium Calendar.",
        )
        .once_per_batch(),
        ability(cost("{2}").and(TAP), double_the_time_counters, "{2}, {T}: Double the number of time counters on The Millennium Calendar."),
        state_triggered(
            a_thousand_time_counters,
            sacrifice_it_and_each_opponent_loses_1000_life,
            "When there are 1,000 or more time counters on The Millennium Calendar, sacrifice it and each opponent loses 1,000 life.",
        ),
    ],
    ..oracle::THE_MILLENNIUM_CALENDAR
};
