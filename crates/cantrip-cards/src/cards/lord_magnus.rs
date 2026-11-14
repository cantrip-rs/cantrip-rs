use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Creatures with plainswalk can be blocked as though they didn't have plainswalk. Creatures with forestwalk can be blocked as though they didn't have forestwalk." (CR 604.1).
fn creatures_plainswalk_can_be_blocked_rule(
    q: &dyn Query,
    asker: &Asker,
    _: ObjectId,
    attacker: ObjectId,
) -> Keywords {
    if q.chars(attacker).is(CardType::Creature)
        && Keyword::landwalk(q.words(asker.source).subtype(Subtype::Plains))
            .is_some_and(|walk| q.chars(attacker).keywords.matches(walk))
    {
        Keyword::landwalk(q.words(asker.source).subtype(Subtype::Plains))
            .map_or(Keywords::NONE, |walk| Keywords::of(&[walk]))
    } else {
        Keywords::NONE
    }
}

/// "Creatures with plainswalk can be blocked as though they didn't have plainswalk. Creatures with forestwalk can be blocked as though they didn't have forestwalk." (CR 604.1).
fn creatures_plainswalk_can_be_blocked_rule_2(
    q: &dyn Query,
    asker: &Asker,
    _: ObjectId,
    attacker: ObjectId,
) -> Keywords {
    if q.chars(attacker).is(CardType::Creature)
        && Keyword::landwalk(q.words(asker.source).subtype(Subtype::Forest))
            .is_some_and(|walk| q.chars(attacker).keywords.matches(walk))
    {
        Keyword::landwalk(q.words(asker.source).subtype(Subtype::Forest))
            .map_or(Keywords::NONE, |walk| Keywords::of(&[walk]))
    } else {
        Keywords::NONE
    }
}

pub static CARD: CardDef = CardDef {
    landwalk_exceptions: &[
        creatures_plainswalk_can_be_blocked_rule,
        creatures_plainswalk_can_be_blocked_rule_2,
    ],
    ..oracle::LORD_MAGNUS
};
