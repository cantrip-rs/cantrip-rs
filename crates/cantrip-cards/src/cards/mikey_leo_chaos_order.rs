use cantrip_api::*;
use cantrip_mechanics::effects::draw;
use cantrip_oracle as oracle;

/// "Whenever you put a counter on a creature you control" (CR 122.6, 603.2c): a counter of any kind. The player who put it is the event's,
/// so a counter an opponent puts on your creature does not trigger it, and a counter put on a creature you do not control does not either.
fn you_put_a_counter_on_a_creature_you_control(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, by, .. }
            if *by == q.controller(source) && q.controller(*object) == q.controller(source) && q.chars(*object).is(CardType::Creature) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::COUNTERS, you_put_a_counter_on_a_creature_you_control),
        |g, item| draw(g, item, 1),
        "Whenever you put a counter on a creature you control, draw a card. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::MIKEY_LEO_CHAOS_ORDER
};
