use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "Once during each of your turns, you may cast an instant or sorcery spell from your hand without paying its mana cost" (CR 118.9,
/// 601.3): a way this permanent gives, for no mana (X is 0, CR 107.3b; additional costs are still paid, CR 118.9d). It is used up by
/// a cast by it this turn, which the history records as made by this permanent (`Cast::by`); another Zaffai, or this one come back, is
/// another permission (CR 400.7).
fn cast_free(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller || q.state().turn.active != player {
        return None;
    }
    let c = q.chars(card);
    if !(c.is(CardType::Instant) || c.is(CardType::Sorcery)) {
        return None;
    }
    (!history::permission_used_this_turn(q, asker.source)).then_some(Cost::NONE)
}

pub static CARD: CardDef = CardDef {
    granted_ways: &[GrantedWay {
        text: "Once during each of your turns, you may cast an instant or sorcery spell from your hand without paying its mana cost.",
        cost: cast_free,
        from: ZoneKind::Hand,
        ..GrantedWay::CAST
    }],
    ..oracle::ZAFFAI_AND_THE_TEMPESTS
};
