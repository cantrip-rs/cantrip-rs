use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

const TEXT: &str = "Enchanted creature gets +2/+2 as long as it's a Detective you control. Otherwise, it has base power and toughness 1/1 and can't block Detectives.";

/// "It's a Detective you control" (CR 205.3, 613.1d): the enchanted creature as the layers before the one being applied leave it, and
/// under the Aura's controller's control (the controller is settled in layer 2, so a Detective that changed hands is not one).
fn is_a_detective_you_control(q: &dyn LayerQuery, source: ObjectId, host: ObjectId) -> bool {
    q.controller(host) == q.controller(source) && q.chars(host).has_subtype(q.words(source).subtype(Subtype::Detective))
}

/// "Enchanted creature gets +2/+2 as long as it's a Detective you control" (CR 604.1, 611.3a): asked again whenever characteristics are computed.
fn enchanted_detective_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    attach::attached_object(q, source, affected, so_far) && is_a_detective_you_control(q, source, affected)
}

/// "Otherwise, it has base power and toughness 1/1" (CR 613.4b): the same creature when it is not a Detective you control.
fn enchanted_otherwise(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    attach::attached_object(q, source, affected, so_far) && !is_a_detective_you_control(q, source, affected)
}

/// "Otherwise, ... can't block Detectives" (CR 509.1b) as a `CardDef::block_rules` entry: no "has", so a rule of the Aura's, not an ability
/// of the creature (CR 113.1a). It ends where the creature is a Detective you control, and holds against any attacker that is a Detective.
fn enchanted_otherwise_cant_block_detectives(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    let detective = q.words(source).subtype(Subtype::Detective);
    let a_detective_you_control = q.controller(blocker) == q.controller(source) && q.chars(blocker).has_subtype(detective);
    q.obj(source).attached_to != Some(blocker) || a_detective_you_control || !q.chars(attacker).has_subtype(detective)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::PtModify, enchanted_detective_you_control, statics::plus_pt::<2, 2>, TEXT),
        static_ability(Layer::PtSet, enchanted_otherwise, statics::base_pt::<1, 1>, TEXT),
    ],
    block_rules: &[enchanted_otherwise_cant_block_detectives],
    ..oracle::BURDEN_OF_PROOF
};
