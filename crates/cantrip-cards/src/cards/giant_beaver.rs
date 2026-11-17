use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_target;
use cantrip_mechanics::saddle;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

/// "Target creature that saddled it this turn" (CR 115.1, 702.171c): a creature on the battlefield that was tapped to pay the cost
/// of this Mount's saddle ability this turn, asked of the history - by any saddle activation this turn, whether or not it is the
/// one that made the Mount saddled for this attack.
fn creature_that_saddled_it(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.history().saddled(asker.source).any(|c| c == id))
}

/// "Whenever this creature attacks while saddled, put a +1/+1 counter on target creature that saddled it this turn." (CR 122.1a,
/// 115.1): any creature that tapped to saddle it, its controller's. Vigilance is a core keyword; "Saddle 3" (CR 702.171a) is the
/// shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            plus_one_counter_on_target,
            "Whenever this creature attacks while saddled, put a +1/+1 counter on target creature that saddled it this turn.",
        )
        .targets(&[target(creature_that_saddled_it)]),
        saddle::ability(3, "Saddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::GIANT_BEAVER
};
