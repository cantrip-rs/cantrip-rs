use cantrip_api::*;
use cantrip_mechanics::effects::may_draw_that_many_once_each_turn;
use cantrip_mechanics::history::done_this_turn;
use cantrip_oracle as oracle;

/// "Whenever you put one or more +1/+1 counters on a creature you control" (CR 122.6a, 603.2c): the player who put
/// them is the event's, so an opponent's effect putting counters on your creature does not trigger it, and a creature
/// entering under your control with counters does. Not at all once its controller has drawn with it this turn
/// (CR 603.2h). The counters put at once carry as the number to draw.
fn you_put_plus_one_counters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, kind: CounterKind::PlusOne, amount, by, .. }
            if *amount > 0
                && *by == me.controller
                && q.controller(*object) == me.controller
                && q.chars(*object).is(CardType::Creature)
                && !done_this_turn(q, me) =>
        {
            Some(Fired { subject: Target::Object(*object), data: [*amount as u32, 0] })
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when_asked(EventMask::COUNTERS, you_put_plus_one_counters), may_draw_that_many_once_each_turn, "Whenever you put one or more +1/+1 counters on a creature you control, you may draw that many cards. Do this only once each turn.")],
    ..oracle::TERRASYMBIOSIS
};
