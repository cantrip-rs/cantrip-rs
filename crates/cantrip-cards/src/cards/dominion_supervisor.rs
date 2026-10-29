use cantrip_api::*;
use cantrip_mechanics::{effects, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "Creatures you control with counters on them have trample." Counters of every kind count (CR 613.1f, 702.19).
fn your_creatures_with_any_counters(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::your_creatures(q, source, affected, so_far)
        && CounterKind::ALL.into_iter().any(|kind| q.state().obj(affected).counters(kind) > 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::plus_one_counter_on_target,
        "When this creature enters, put a +1/+1 counter on target creature.",
    )
    .targets(&[targets::CREATURE])],
    statics: &[static_ability(
        Layer::Ability,
        your_creatures_with_any_counters,
        statics::have_trample,
        "Creatures you control with counters on them have trample.",
    )],
    ..oracle::DOMINION_SUPERVISOR
};
