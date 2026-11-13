use cantrip_api::*;
use cantrip_oracle as oracle;

/// "No more than two creatures can attack each combat. No more than two creatures can block each combat." (CR 604.1).
fn no_more_than_two_creatures_rule(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

/// "No more than two creatures can attack each combat. No more than two creatures can block each combat." (CR 604.1).
fn no_more_than_two_creatures_rule_counted(
    _: &dyn Query,
    _: &Asker,
    _: ObjectId,
    _: ObjectId,
) -> bool {
    true
}

/// "No more than two creatures can attack each combat. No more than two creatures can block each combat." (CR 604.1).
fn no_more_than_two_creatures_rule_2(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

/// "No more than two creatures can attack each combat. No more than two creatures can block each combat." (CR 604.1).
fn no_more_than_two_creatures_rule_counted_2(
    _: &dyn Query,
    _: &Asker,
    _: ObjectId,
    _: ObjectId,
) -> bool {
    true
}

pub static CARD: CardDef = CardDef {
    combat_groups: &[
        CombatGroupRule {
            role: CombatRole::Attacker,
            applies: no_more_than_two_creatures_rule,
            counted: no_more_than_two_creatures_rule_counted,
            others_only: false,
            at_least: 0,
            at_most: 2,
        },
        CombatGroupRule {
            role: CombatRole::Blocker,
            applies: no_more_than_two_creatures_rule_2,
            counted: no_more_than_two_creatures_rule_counted_2,
            others_only: false,
            at_least: 0,
            at_most: 2,
        },
    ],
    ..oracle::CAVERNS_OF_DESPAIR
};
