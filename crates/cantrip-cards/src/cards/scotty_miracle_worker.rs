use cantrip_api::*;
use cantrip_mechanics::targets::filters::creature;
use cantrip_oracle as oracle;

/// "During your main phase, each card in your graveyard has beam me up. The beam me up cost is equal to that spell's mana
/// cost. Scotty can't be beamed up to cast a spell this way" (CR 601.3, 118.9, 702.180's wording for "then exile this
/// spell"): the way is the one `cantrip_mechanics::beam_me_up` prints - the card's mana cost and a creature you control
/// returned to its owner's hand, cast from the graveyard and exiled as it leaves the stack - offered only to Scotty's
/// controller, only in their main phase, and only for an instant or sorcery card (the keyword casts nothing else).
fn beam_me_up_in_your_main_phase(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller || q.state().turn.active != player {
        return None;
    }
    if !matches!(q.state().turn.step(), Step::PrecombatMain | Step::PostcombatMain) {
        return None;
    }
    let chars = q.chars(card);
    // GrantedWay::flashback already restricts eligible faces to instant or sorcery spells.
    let returning = PermanentCost { count: 1, filter: creature, action: PermanentAction::BeamUp, tapped: false, excluded: Some(asker.source), together: None };
    Some(Cost::mana(q.card(chars.card).cost).and(Cost { permanents: Some(returning), ..Cost::NONE }))
}

pub static CARD: CardDef = CardDef {
    granted_ways: &[GrantedWay {
        text: "Beam me up",
        cost: beam_me_up_in_your_main_phase,
        from: ZoneKind::Graveyard,
        flashback: true,
        ..GrantedWay::CAST
    }],
    ..oracle::SCOTTY_MIRACLE_WORKER
};
