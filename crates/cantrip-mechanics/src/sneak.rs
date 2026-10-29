//! Sneak [cost] (CR 702.190a): "Any time you could cast an instant during your declare blockers step, you may cast this
//! spell by paying [cost] and returning an unblocked creature you control to its owner's hand rather than paying this
//! spell's mana cost." Shredder's Technique, The Last Ronin's Technique and Leonardo, Cutting Edge print it.
//!
//! A card with it lists `way(cost, text)` among its `ways`: the alternative cost is the mana and the return of an unblocked
//! attacker (a cost chosen with the other costs, CR 601.2b, h), offered from the hand in the declare blockers step of its
//! controller's turn, at instant speed whatever the card's type (`WayDef::instant_speed`).
//!
//! "A permanent spell whose sneak cost was paid enters the battlefield tapped and attacking", attacking what the returned
//! creature was attacking (CR 702.190b): the core keeps that on the spell as the cost is paid and puts the permanent into
//! the combat as it resolves (`WayDef::enters_attacking`), unless the rules keep it out (CR 506.3, 508.4a). It was never
//! declared as an attacker (CR 508.4). Leonardo, Cutting Edge prints it on a creature.
//!
//! "If this spell's sneak cost was paid" is `paid` of the resolving spell (The Last Ronin's Technique, Shredder's
//! Technique prints sneak).

use cantrip_api::*;

use crate::combat::unblocked_attacker_you_control;

/// "During your declare blockers step" (CR 702.190a): the declare blockers step of a turn of `player`.
fn in_declare_blockers_step_of(q: &dyn Query, player: PlayerId) -> bool {
    let turn = &q.state().turn;
    turn.active == player && turn.step() == Step::DeclareBlockers
}

fn your_declare_blockers_step(q: &dyn Query, asker: &Asker) -> bool {
    in_declare_blockers_step_of(q, asker.controller)
}

/// The alternative cost of sneak: `cost` and an unblocked attacker you control returned to its owner's hand. `text` is the
/// way's text, which a scenario's `Ability:` matches.
pub const fn way(cost: ManaCost, text: &'static str) -> WayDef {
    cantrip_api::way(Cost::mana(cost).and(return_to_hand(1, unblocked_attacker_you_control)), ZoneKind::Hand, text)
        .condition(your_declare_blockers_step)
        .instant_speed()
        .enters_attacking()
        .kind(WayKind::Sneak)
}

/// The cost of a sneak a permanent gives (CR 702.190a, 113.6, 604): `mana` and an unblocked attacker `player` controls returned
/// to its owner's hand, or `None` outside the declare blockers step of their turn, when it can't be cast that way.
pub fn granted_cost(q: &dyn Query, player: PlayerId, mana: ManaCost) -> Option<Cost> {
    in_declare_blockers_step_of(q, player).then(|| Cost::mana(mana).and(return_to_hand(1, unblocked_attacker_you_control)))
}

/// The sneak cost of the card itself, for a permanent that lets it be cast from `from` using the sneak ability it has (CR
/// 702.190a, 601.3): "you may cast creature spells from your graveyard using their sneak abilities" (Ninja Teen). `None`
/// outside the declare blockers step of `player`'s turn and for a card with no sneak.
pub fn own_cost(q: &dyn Query, player: PlayerId, card: ObjectId) -> Option<Cost> {
    let way = q.card(q.chars(card).card).ways.iter().find(|w| w.kind == WayKind::Sneak)?;
    in_declare_blockers_step_of(q, player).then_some(way.cost)
}

/// Sneak a permanent gives to the cards in `from` (CR 702.190a, 113.6): "creature cards in your graveyard have sneak {3}{B}"
/// (Ninja Teen). `cost` is each card's sneak cost, `granted_cost` of its mana, or none for a card without it. Like a card's own
/// sneak it is cast whenever an instant could be, and a permanent spell cast so enters tapped and attacking (CR 702.190b).
pub const fn granted(from: ZoneKind, cost: GrantedCost) -> GrantedWay {
    GrantedWay { text: "Sneak", cost, from, kind: WayKind::Sneak, instant_speed: true, enters_attacking: true, ..GrantedWay::CAST }
}

/// "If this spell's sneak cost was paid": it was cast for its first alternative cost, which a card with sneak lists first
/// and, so far, alone. A copy of the spell was not cast, but it copies the alternative cost the original was cast for (CR
/// 707.10): its sneak cost was paid too.
pub fn paid(_: &dyn Query, item: &StackItem) -> bool {
    item.way == 1
}

/// "If its sneak cost was paid" of a permanent (CR 702.190a): it was cast through the way a card lists as sneak
/// (`WayKind::Sneak`, set by `way` and `granted`), the card's own or one a permanent gave it (Ninja Teen). Another way, a copy
/// and a permanent that was not cast were not. Leonardo, Leader in Blue and Turncoat Kunoichi ask it of the creature whose
/// enters ability is on the stack. Read from the spell's `Cast` record, as `escape::escaped` is.
pub fn was_cast_for_sneak(q: &dyn Query, id: ObjectId) -> bool {
    let cast = q.history().all().iter().rev().find_map(|r| match &r.event {
        Event::Cast(c) if c.spell == id || q.obj(c.spell).became == Some(id) => Some(*c),
        _ => None,
    });
    let Some(cast) = cast else { return false };
    match cast.granted_by() {
        Some(grant) => grant
            .ability
            .as_granted_way()
            .and_then(|k| q.card(grant.ability.card).granted_ways.get(k))
            .is_some_and(|w| w.kind == WayKind::Sneak),
        None => cast.way > 0 && q.card(q.obj(cast.spell).card).ways.get(cast.way as usize - 1).is_some_and(|w| w.kind == WayKind::Sneak),
    }
}
