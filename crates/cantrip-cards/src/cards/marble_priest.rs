use cantrip_api::*;
use cantrip_oracle as oracle;

/// "All Walls able to block this creature do so. Prevent all combat damage that would be dealt to this creature by Walls." (CR 604.1).
fn all_walls_able_block_creature_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    attacker == asker.source
        && q.chars(blocker).has_subtype(q.words(asker.source).subtype(Subtype::Wall))
}

/// "All Walls able to block this creature do so. Prevent all combat damage that would be dealt to this creature by Walls." (CR 615.1).
fn all_walls_able_block_creature_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target, combat) = (e.source, &e.target, e.combat);
        if combat && !q.state().obj(source).zone.is(ZoneKind::Stack)
            && q.chars(source).has_subtype(q.words(asker.source).subtype(Subtype::Wall))
            && matches!(* target, Target::Object(o) if asker.source == o)
        {
            return true;
        }
    }
    false
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all combat damage that would be dealt to this creature by Walls",
            applies: all_walls_able_block_creature_applies,
            amount: PreventionAmount::All,
        },
    ],
    block_requirements: &[all_walls_able_block_creature_rule],
    ..oracle::MARBLE_PRIEST
};
