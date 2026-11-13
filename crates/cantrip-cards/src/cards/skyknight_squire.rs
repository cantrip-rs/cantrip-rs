use cantrip_api::*;
use cantrip_mechanics::triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS;
use cantrip_mechanics::{effects, lands, statics};
use cantrip_oracle as oracle;

/// "As long as this creature has three or more +1/+1 counters on it" (CR 122.1, 613.1): only +1/+1 counters are counted, and it is
/// asked again whenever characteristics are computed, so it follows the counters as they come and go.
fn has_three_or_more_plus_one_counters(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().obj(source).counters(CounterKind::PlusOne) >= 3
}

const STATIC_TEXT: &str = "As long as this creature has three or more +1/+1 counters on it, it has flying and is a Knight in addition to its other types.";

/// The Knight subtype is a layer 4 effect and flying a layer 6 one (CR 613.1d, 613.1f): one ability, so the second continues the first.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        effects::plus_one_counter_on_this,
        "Whenever another creature you control enters, put a +1/+1 counter on this creature.",
    )],
    statics: &[
        static_ability(Layer::Type, has_three_or_more_plus_one_counters, statics::also_a::<{ lands::sub(Subtype::Knight) }>, STATIC_TEXT),
        static_ability(Layer::Ability, has_three_or_more_plus_one_counters, statics::have_flying, STATIC_TEXT).continues(),
    ],
    ..oracle::SKYKNIGHT_SQUIRE
};
