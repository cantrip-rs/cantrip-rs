use cantrip_api::*;
use cantrip_mechanics::effects::return_first_target_to_hand_then_plus_one_counter_on_this;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Up to one other target permanent you control" (CR 115.1, 109.1): any permanent its controller controls but the Nurturer
/// itself, a land, an artifact or a creature among them.
fn other_permanent_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_you_control_filter(q, asker, t) && matches!(t, Target::Object(id) if id != asker.source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        return_first_target_to_hand_then_plus_one_counter_on_this,
        "When this creature enters, return up to one other target permanent you control to its owner's hand. If you do, put a +1/+1 counter on this creature.",
    )
    .targets(&[up_to_one(other_permanent_you_control)])],
    ..oracle::MIRKWOOD_NURTURER
};
