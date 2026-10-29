use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Whenever one or more permanent cards are put into your graveyard from anywhere while this creature has a -1/-1
/// counter on it" (CR 603.2c): asked of each card that arrives, and one trigger for the cards that arrive together. A
/// permanent card is a nontoken card of a permanent type. The counters are the ones it has as the cards arrive.
fn permanent_card_to_your_graveyard(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    if *to != Zone::graveyard(q.controller(source)) || q.obj(*new).token || q.obj(source).counters(CounterKind::MinusOne) <= 0 {
        return None;
    }
    if targets::is_permanent_card(&q.chars(*new)) { Fired::about(Target::Object(*new)) } else { None }
}

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::effects::enters_with_minus_one_counters::<6>),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, permanent_card_to_your_graveyard), cantrip_mechanics::effects::remove_a_minus_one_counter_from_this,"Whenever one or more permanent cards are put into your graveyard from anywhere while this creature has a -1/-1 counter on it, remove a -1/-1 counter from this creature.")
        .once_per_batch()],
    ..oracle::MOONSHADOW
};
