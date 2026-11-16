use cantrip_api::*;
use cantrip_mechanics::effects::may_draw_that_many_once_each_turn;
use cantrip_mechanics::history::done_this_turn;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever you put one or more +1/+1 counters on Chekov and McCoy" (CR 122.6, 603.2c): the shared trigger, which carries
/// the number put as the number to draw. Not at all once its controller has drawn with it this turn (CR 603.2h).
fn you_put_plus_one_counters_on_this_unless_done(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if done_this_turn(q, me) {
        return None;
    }
    triggers::you_put_plus_one_counters_on_this(q, me, event)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::COUNTERS, you_put_plus_one_counters_on_this_unless_done),
        may_draw_that_many_once_each_turn,
        "Whenever you put one or more +1/+1 counters on Chekov and McCoy, you may draw that many cards. Do this only once each turn.",
    )],
    ..oracle::CHEKOV_AND_MCCOY
};
