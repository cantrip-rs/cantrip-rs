use cantrip_api::*;
use cantrip_mechanics::{effects, increment};
use cantrip_oracle as oracle;

/// "Whenever one or more +1/+1 counters are put on this creature" (CR 121.1, 603.2c): one trigger per event, however many
/// counters it put.
fn plus_one_counters_put(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let Event::CountersPut { object, kind: CounterKind::PlusOne, .. } = event else { return None };
    if *object == source { Fired::PLAIN } else { None }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        increment::TRIGGER,
        triggered(when(EventMask::COUNTERS, plus_one_counters_put), |g, item| effects::draw(g, item, 1), "Whenever one or more +1/+1 counters are put on this creature, draw a card."),
    ],
    ..oracle::PENSIVE_PROFESSOR
};
