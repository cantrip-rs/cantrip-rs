use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't be blocked by Walls." (CR 604.1).
fn creature_cant_be_blocked_by_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(attacker == asker.source
        && q.chars(blocker).has_subtype(q.words(asker.source).subtype(Subtype::Wall)))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[creature_cant_be_blocked_by_rule],
    ..oracle::BOG_RATS
};
