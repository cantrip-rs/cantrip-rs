use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't attack unless at least two other creatures attack. This creature can't block unless at least two other creatures block." (CR 604.1).
fn creature_cant_attack_unless_least_rule(
    _: &dyn Query,
    asker: &Asker,
    id: ObjectId,
) -> bool {
    id == asker.source
}

/// "This creature can't attack unless at least two other creatures attack. This creature can't block unless at least two other creatures block." (CR 604.1).
fn creature_cant_attack_unless_least_rule_counted(
    q: &dyn Query,
    _: &Asker,
    _: ObjectId,
    other: ObjectId,
) -> bool {
    q.chars(other).is(CardType::Creature)
}

/// "This creature can't attack unless at least two other creatures attack. This creature can't block unless at least two other creatures block." (CR 604.1).
fn creature_cant_attack_unless_least_rule_2(
    _: &dyn Query,
    asker: &Asker,
    id: ObjectId,
) -> bool {
    id == asker.source
}

/// "This creature can't attack unless at least two other creatures attack. This creature can't block unless at least two other creatures block." (CR 604.1).
fn creature_cant_attack_unless_least_rule_counted_2(
    q: &dyn Query,
    _: &Asker,
    _: ObjectId,
    other: ObjectId,
) -> bool {
    q.chars(other).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    combat_groups: &[
        CombatGroupRule {
            role: CombatRole::Attacker,
            applies: creature_cant_attack_unless_least_rule,
            counted: creature_cant_attack_unless_least_rule_counted,
            others_only: true,
            at_least: 2,
            at_most: u32::MAX,
        },
        CombatGroupRule {
            role: CombatRole::Blocker,
            applies: creature_cant_attack_unless_least_rule_2,
            counted: creature_cant_attack_unless_least_rule_counted_2,
            others_only: true,
            at_least: 2,
            at_most: u32::MAX,
        },
    ],
    ..oracle::ORCISH_CONSCRIPTS
};
