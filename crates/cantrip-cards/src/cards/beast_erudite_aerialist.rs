use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{effects, statics};
use cantrip_oracle as oracle;

/// "As long as you've put one or more +1/+1 counters on Beast this turn" (CR 611.3a), as a static ability's
/// `StaticDef::affects`: only Beast himself, and only while this turn's history holds counters its controller put on him.
fn itself_after_you_put_a_counter_on_it(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let me = q.state().obj(source).controller;
    source == affected
        && q.history().this_turn().iter().any(|r| matches!(r.event, Event::CountersPut { object, kind: CounterKind::PlusOne, by, .. } if object == source && by == me))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, |g, item| effects::draw(g, item, 1), "Whenever Beast deals combat damage to a player, draw a card.")],
    statics: &[static_ability(
        Layer::Ability,
        itself_after_you_put_a_counter_on_it,
        statics::have_flying,
        "As long as you've put one or more +1/+1 counters on Beast this turn, he has flying.",
    )],
    ..oracle::BEAST_ERUDITE_AERIALIST
};
