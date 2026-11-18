use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Whenever a face-down permanent you control is turned face up" (CR 708.2, 708.8, 603.2): the permanent that was turned up,
/// whoever turned it, while this creature's controller controls it; it is the trigger's subject ("it"). This creature itself
/// counts if it was face down (a cloaked or disguised Sumala Sentry has no abilities until it is face up, so it sees only its own
/// turning up).
fn face_down_permanent_you_control_turned_face_up(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::TurnedFaceUp { permanent, .. } if q.controller(permanent) == me.controller => Fired::about(Target::Object(permanent)),
        _ => None,
    }
}

/// "Put a +1/+1 counter on it and a +1/+1 counter on this creature" (CR 122.1): both placed together, on the permanent that was
/// turned up and on the source; either one that has left the battlefield by now is a new object (CR 400.7, 608.2b) and gets
/// none. When the permanent turned up is the source it gets both.
fn counter_on_it_and_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut puts = Vec::new();
    if let Some(permanent) = item.subject.object().filter(|&p| targets::permanent(g, p)) {
        puts.push((permanent, CounterKind::PlusOne, 1));
    }
    if targets::permanent(g, item.object) {
        puts.push((item.object, CounterKind::PlusOne, 1));
    }
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::TURNED_FACE_UP, face_down_permanent_you_control_turned_face_up),
        counter_on_it_and_on_this,
        "Whenever a face-down permanent you control is turned face up, put a +1/+1 counter on it and a +1/+1 counter on this creature.",
    )],
    ..oracle::SUMALA_SENTRY
};
