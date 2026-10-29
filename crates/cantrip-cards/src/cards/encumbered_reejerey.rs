use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::this_becomes_tapped;
use cantrip_oracle as oracle;

/// "Whenever this creature becomes tapped while it has a -1/-1 counter on it" (CR 603.4): asked as it becomes tapped, with the
/// counters it has then; the effect checks again as it resolves.
fn becomes_tapped_with_a_counter(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    if q.obj(source).counters(CounterKind::MinusOne) <= 0 {
        return None;
    }
    this_becomes_tapped(q, source, event)
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<3>),
    abilities: &[triggered(
        when(EventMask::TAPPED, becomes_tapped_with_a_counter),
        effects::remove_a_minus_one_counter_from_this,
        "Whenever this creature becomes tapped while it has a -1/-1 counter on it, remove a -1/-1 counter from it.",
    )],
    ..oracle::ENCUMBERED_REEJEREY
};
