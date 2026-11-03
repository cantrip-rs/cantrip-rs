use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Tetsuo Umezawa can't be the target of Aura spells. {U}{B}{B}{R}, {T}: Destroy target tapped or blocking creature." (CR 604.1).
fn destroy_target_tapped_blocking_creature_rule_on(
    q: &dyn LayerQuery,
    source: ObjectId,
    subject: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    subject == asker.source
}

/// "Tetsuo Umezawa can't be the target of Aura spells. {U}{B}{B}{R}, {T}: Destroy target tapped or blocking creature." (CR 604.1).
fn destroy_target_tapped_blocking_creature_rule_by(
    q: &dyn LayerQuery,
    source: ObjectId,
    by: &Asker,
    by_source: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    by.spell && (*by_source).has_subtype(q.words(asker.source).subtype(Subtype::Aura))
}

/// "{U}{B}{B}{R}, {T}: Destroy target tapped or blocking creature." (CR 115.1).
fn destroy_target_tapped_blocking_creature_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (((q
        .state().obj(id).tapped || q.is_blocking(id)) &&
        q.chars(id).is(CardType::Creature)))
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{U}{B}{B}{R}").and(TAP),
                effects::destroy_first_target,
                "{U}{B}{B}{R}, {T}: Destroy target tapped or blocking creature.",
            )
            .targets(&[target(destroy_target_tapped_blocking_creature_target_0)]),
    ],
    targeting_rules: &[
        TargetingRule {
            kind: TargetingRuleKind::Forbid(
                destroy_target_tapped_blocking_creature_rule_by,
            ),
            applies: destroy_target_tapped_blocking_creature_rule_on,
        },
    ],
    ..oracle::TETSUO_UMEZAWA
};
