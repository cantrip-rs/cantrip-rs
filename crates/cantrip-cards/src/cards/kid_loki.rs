use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::statics::{have_hexproof, your_creatures};
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_oracle as oracle;

/// "Each creature you control that you've put one or more +1/+1 counters on this turn" (CR 122.6, 611.3a): a creature under
/// Kid Loki's controller's control that has had a +1/+1 counter put on it this turn by that player, asked of the event
/// history (the layers ask it again whenever characteristics are computed). A creature that left and came back is a new
/// object (CR 400.7) and has had none put on it.
fn creatures_you_put_plus_one_counters_on_this_turn(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let you = view.state().obj(source).controller;
    your_creatures(view, source, affected, so_far)
        && view.history().select(EventQuery::kind(EventMask::COUNTERS).this_turn().object(affected)).any(|record| {
            matches!(record.event, Event::CountersPut { kind: CounterKind::PlusOne, amount, by, .. } if amount > 0 && by == you)
        })
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        creatures_you_put_plus_one_counters_on_this_turn,
        have_hexproof,
        "Each creature you control that you've put one or more +1/+1 counters on this turn has hexproof.",
    )],
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        plus_one_counter_on_this,
        "Whenever you draw your second card each turn, put a +1/+1 counter on Kid Loki.",
    )],
    ..oracle::KID_LOKI
};
