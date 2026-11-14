use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Creatures with swampwalk can be blocked as though they didn't have swampwalk." (CR 604.1).
fn creatures_swampwalk_can_be_blocked_rule(
    q: &dyn Query,
    asker: &Asker,
    _: ObjectId,
    attacker: ObjectId,
) -> Keywords {
    if q.chars(attacker).is(CardType::Creature)
        && Keyword::landwalk(q.words(asker.source).subtype(Subtype::Swamp))
            .is_some_and(|walk| q.chars(attacker).keywords.matches(walk))
    {
        Keyword::landwalk(q.words(asker.source).subtype(Subtype::Swamp))
            .map_or(Keywords::NONE, |walk| Keywords::of(&[walk]))
    } else {
        Keywords::NONE
    }
}

pub static CARD: CardDef = CardDef {
    landwalk_exceptions: &[creatures_swampwalk_can_be_blocked_rule],
    ..oracle::UR_DRAGO
};
