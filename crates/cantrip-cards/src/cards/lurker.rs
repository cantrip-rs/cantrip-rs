use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't be the target of spells unless it attacked or blocked this turn." (CR 604.1).
fn creature_cant_be_target_spells_rule_on(
    q: &dyn LayerQuery,
    source: ObjectId,
    subject: ObjectId,
) -> bool {
    subject == source
        && !q.history().attacked_this_turn(source)
        && !q.history().this_turn().iter().any(|r| {
            matches!(r.event, Event::Blocks { blocker, .. } if blocker == source)
        })
}

/// "This creature can't be the target of spells unless it attacked or blocked this turn." (CR 604.1).
fn creature_cant_be_target_spells_rule_by(
    _: &dyn LayerQuery,
    _: ObjectId,
    by: &Asker,
    _: &Chars,
) -> bool {
    by.spell
}

pub static CARD: CardDef = CardDef {
    targeting_rules: &[
        TargetingRule {
            kind: TargetingRuleKind::Forbid(creature_cant_be_target_spells_rule_by),
            applies: creature_cant_be_target_spells_rule_on,
        },
    ],
    ..oracle::LURKER
};
