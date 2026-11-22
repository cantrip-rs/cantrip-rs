//! Mayhem [cost] (CR 702.187a): "You may cast this card from your graveyard if you discarded it this turn. Timing rules
//! still apply." Carnage, Crimson Chaos prints it; Norman Osborn's back face (Green Goblin) gives it to every nonland
//! card in its controller's graveyard (CR 113.6).
//!
//! A card with mayhem lists `way(cost, text)` among its `ways`. A card that grants it lists `GRANTED` among its
//! `granted_ways`, the cost being each card's own mana cost. Mayhem without a cost (CR 702.187c), a land's (Oscorp
//! Industries), is `PLAY` among its `exile_casts`.

use cantrip_api::*;

use crate::cast_from_your_graveyard;

/// Whether the card was discarded this turn: asked of the event history.
pub fn discarded_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().discarded_this_turn(asker.source)
}

/// The alternative cost of mayhem: cast from the graveyard for `cost`, if it was discarded this turn.
pub const fn way(cost: ManaCost, text: &'static str) -> WayDef {
    cantrip_api::way(Cost::mana(cost), ZoneKind::Graveyard, text)
        .condition(discarded_this_turn)
        .kind(WayKind::Mayhem)
}

/// "Each nonland card in your graveyard has mayhem. The mayhem cost is equal to its mana cost": for the cards in the
/// granter's controller's graveyard that have a mana cost, cast by that controller. Each has it, discarded this turn or not
/// (CR 113.6); it is used only for one that was (`granted_discarded_this_turn`).
fn granted_cost(q: &dyn Query, asker: &Asker, card: ObjectId, caster: PlayerId) -> Option<Cost> {
    let obj = q.obj(card);
    let def = q.card(q.chars(card).card);
    if caster != asker.controller || obj.owner != asker.controller || def.is(CardType::Land) || !def.has_cost {
        return None;
    }
    Some(Cost::mana(def.cost))
}

/// "You may cast a card from your graveyard for its mayhem cost if you discarded it this turn" (CR 702.187a): asked of the
/// event history.
fn granted_discarded_this_turn(q: &dyn Query, _: &Asker, card: ObjectId, _: PlayerId) -> bool {
    q.history().discarded_this_turn(card)
}

pub const GRANTED: GrantedWay =
    GrantedWay { text: "Mayhem", cost: granted_cost, condition: Some(granted_discarded_this_turn), kind: WayKind::Mayhem, ..GrantedWay::CAST };

/// Whether `id` was cast for its mayhem cost (CR 702.187a-b): it is a spell, or the permanent a spell became as it resolved
/// (CR 400.7, 608.3), that was cast from a graveyard by a mayhem way - the card's own, or the one Green Goblin gives every
/// nonland card in its controller's graveyard, which is a mayhem cost as much as the printed one (CR 702.187b). Another
/// permission is not (a card cast by Underworld Breach's escape was discarded this turn all the same). Read from the spell's
/// `Cast` record: a copy of a spell was not cast (CR 707.10) and paid nothing.
pub fn paid(q: &dyn Query, id: ObjectId) -> bool {
    crate::history::cast_way_kind(q, id) == Some(WayKind::Mayhem)
}

/// Mayhem without a cost (CR 702.187c): "you may play this card from your graveyard if you discarded it this turn" - the card,
/// by its owner, discarded this turn as the event history says.
fn played_discarded_this_turn(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    cast_from_your_graveyard::this_card_by_its_owner(q, asker, card, player) && discarded_this_turn(q, asker)
}

/// Mayhem without a cost (CR 702.187c), as a land prints it (Oscorp Industries): the card's own permission to play it from its
/// graveyard if it was discarded this turn (CR 113.6e). A land is played and not cast, so no cost is paid: a land play, within
/// the turn's land plays and at the times a land may be played (CR 305.1).
pub const PLAY: ExileCast = ExileCast { text: "Mayhem", allows: played_discarded_this_turn, lands: true, ..cast_from_your_graveyard::PERMISSION };
