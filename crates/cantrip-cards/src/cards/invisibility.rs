use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchanted creature can't be blocked except by Walls." (CR 604.1).
fn enchanted_creature_cant_be_blocked_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(q.state().obj(asker.source).attached_to == Some(attacker)
        && !(q.chars(blocker).has_subtype(q.words(asker.source).subtype(Subtype::Wall))))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[enchanted_creature_cant_be_blocked_rule],
    enchant: Some(targets::CREATURE),
    ..oracle::INVISIBILITY
};
