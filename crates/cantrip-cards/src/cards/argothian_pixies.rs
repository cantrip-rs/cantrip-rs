use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't be blocked by artifact creatures. Prevent all damage that would be dealt to this creature by artifact creatures." (CR 604.1).
fn creature_cant_be_blocked_by_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(attacker == asker.source
        && (q.chars(blocker).is(CardType::Artifact)
            && q.chars(blocker).is(CardType::Creature)))
}

/// "This creature can't be blocked by artifact creatures. Prevent all damage that would be dealt to this creature by artifact creatures." (CR 615.1).
fn creature_cant_be_blocked_by_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if !q.state().obj(source).zone.is(ZoneKind::Stack)
            && (q.chars(source).is(CardType::Artifact)
                && q.chars(source).is(CardType::Creature))
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
            hint: "prevents all damage that would be dealt to this creature by artifact creatures",
            applies: creature_cant_be_blocked_by_applies,
            amount: PreventionAmount::All,
        },
    ],
    block_rules: &[creature_cant_be_blocked_by_rule],
    ..oracle::ARGOTHIAN_PIXIES
};
