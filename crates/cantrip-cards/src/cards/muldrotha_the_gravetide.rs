use cantrip_api::*;
use cantrip_oracle as oracle;

/// The permanent types, one way each (CR 110.4): index `K` of `TYPES` is the type of the card Muldrotha's way number `K` plays or
/// casts.
const TYPES: [CardType; 6] = [CardType::Artifact, CardType::Battle, CardType::Creature, CardType::Enchantment, CardType::Planeswalker, CardType::Land];

/// Whether Muldrotha's way `grant` has been used this turn: a spell cast by it or a land played by it, as the history records them.
fn used_this_turn(q: &dyn Query, grant: Grant) -> bool {
    q.history().this_turn().iter().any(|r| match r.event {
        Event::Cast(c) => c.granted_by() == Some(grant),
        Event::LandPlayed { granted_by, .. } => granted_by.some() == Some(grant),
        _ => false,
    })
}

/// "During each of your turns, you may play a land and cast a permanent spell of each permanent type from your graveyard. (If a card
/// has multiple permanent types, choose one as you play it.)" One way of the card per permanent type (CR 604, 601.3), each used up
/// by the one play or cast made by it this turn: a card with several permanent types is offered by the way of each, and the way
/// chosen is the type chosen, so only that type's one is spent. A permission (`GrantedWay::permission`) for the card's own mana cost,
/// so an alternative cost may be paid instead; the land way is the land play (CR 305.1), which still uses the land drop (CR 305.2).
fn way<const K: usize>(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller || q.state().turn.active != player {
        return None;
    }
    let c = q.chars(card);
    if !c.is(TYPES[K]) || used_this_turn(q, Grant { object: asker.source, ability: AbilityRef::granted_way(q.chars(asker.source).card, K) }) {
        return None;
    }
    Some(if c.is(CardType::Land) { Cost::NONE } else { Cost::mana(q.card(c.card).cost) })
}

pub static CARD: CardDef = CardDef {
    granted_ways: &[
        GrantedWay { text: "During each of your turns, you may cast an artifact spell from your graveyard.", cost: way::<0>, permission: true, ..GrantedWay::CAST },
        GrantedWay { text: "During each of your turns, you may cast a battle spell from your graveyard.", cost: way::<1>, permission: true, ..GrantedWay::CAST },
        GrantedWay { text: "During each of your turns, you may cast a creature spell from your graveyard.", cost: way::<2>, permission: true, ..GrantedWay::CAST },
        GrantedWay { text: "During each of your turns, you may cast an enchantment spell from your graveyard.", cost: way::<3>, permission: true, ..GrantedWay::CAST },
        GrantedWay { text: "During each of your turns, you may cast a planeswalker spell from your graveyard.", cost: way::<4>, permission: true, ..GrantedWay::CAST },
        GrantedWay { text: "During each of your turns, you may play a land from your graveyard.", cost: way::<5>, lands: true, permission: true, ..GrantedWay::CAST },
    ],
    ..oracle::MULDROTHA_THE_GRAVETIDE
};
