use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Enchanted creature can't be blocked by artifact creatures. Prevent all damage that would be dealt to enchanted creature by artifact sources. Enchanted creature can't be the target of abilities from artifact sources." (CR 604.1).
fn enchanted_creature_cant_be_blocked_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(q.state().obj(asker.source).attached_to == Some(attacker)
        && (q.chars(blocker).is(CardType::Artifact)
            && q.chars(blocker).is(CardType::Creature)))
}

/// "Enchanted creature can't be blocked by artifact creatures. Prevent all damage that would be dealt to enchanted creature by artifact sources. Enchanted creature can't be the target of abilities from artifact sources." (CR 615.1).
fn enchanted_creature_cant_be_blocked_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if q.chars(source).is(CardType::Artifact)
            && matches!(
                * target, Target::Object(o) if Some(asker.source).and_then(| o : ObjectId
                | q.state().obj(o).attached_to) == Some(o)
            )
        {
            return true;
        }
    }
    false
}

/// "Enchanted creature can't be blocked by artifact creatures. Prevent all damage that would be dealt to enchanted creature by artifact sources. Enchanted creature can't be the target of abilities from artifact sources." (CR 604.1).
fn enchanted_creature_cant_be_blocked_rule_on(
    q: &dyn LayerQuery,
    source: ObjectId,
    subject: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    q.state().obj(asker.source).attached_to == Some(subject)
}

/// "Enchanted creature can't be blocked by artifact creatures. Prevent all damage that would be dealt to enchanted creature by artifact sources. Enchanted creature can't be the target of abilities from artifact sources." (CR 604.1).
fn enchanted_creature_cant_be_blocked_rule_by(
    _: &dyn LayerQuery,
    _: ObjectId,
    by: &Asker,
    by_source: &Chars,
) -> bool {
    !(by.spell) && (!by.spell && (*by_source).is(CardType::Artifact))
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to enchanted creature by artifact sources",
            applies: enchanted_creature_cant_be_blocked_applies,
            amount: PreventionAmount::All,
        },
    ],
    block_rules: &[enchanted_creature_cant_be_blocked_rule],
    enchant: Some(cantrip_mechanics::targets::CREATURE),
    targeting_rules: &[
        TargetingRule {
            kind: TargetingRuleKind::Forbid(enchanted_creature_cant_be_blocked_rule_by),
            applies: enchanted_creature_cant_be_blocked_rule_on,
        },
    ],
    ..oracle::ARTIFACT_WARD
};
