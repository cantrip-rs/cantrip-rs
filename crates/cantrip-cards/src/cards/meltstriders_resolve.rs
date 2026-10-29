use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, fight, statics::plus_pt, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Enchanted creature ... can't be blocked by more than one creature" (CR 509.1b): no "has", so a rule of the Aura's, not
/// an ability the creature gains (CR 113.1a); a creature that loses all its abilities is still under it (CR 613.1f).
fn one_blocker_at_most(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> Option<usize> {
    (q.obj(source).attached_to == Some(attacker)).then_some(1)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_YOU_CONTROL),
    abilities: &[triggered(
        THIS_ENTERS,
        fight::attached_creature_fights_first_target,
        "When this Aura enters, enchanted creature fights up to one target creature an opponent controls. (Each deals damage equal to its power to the other.)",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS.optional()])],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<0, 2>, "Enchanted creature gets +0/+2."),
    ],
    blocker_limits: &[one_blocker_at_most],
    ..oracle::MELTSTRIDERS_RESOLVE
};
