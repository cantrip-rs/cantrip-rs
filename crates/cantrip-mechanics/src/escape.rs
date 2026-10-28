//! Escape--[cost], Exile N other cards from your graveyard (CR 702.138a): "You may cast this card from your graveyard
//! by paying [cost] rather than paying its mana cost" (CR 118.9). Bloodbraid Challenger, From the Catacombs, Phlage,
//! Titan of Fire's Fury and Uro, Titan of Nature's Wrath print it; Underworld Breach grants it to every nonland card in
//! its controller's graveyard (CR 113.6).
//!
//! A card with escape lists `way(cost, exile, text)` among its ways, and a permanent that gives escape lists `granted(cost)`
//! among its granted ways: both are marked `WayKind::Escape`. "Escaped" (CR 702.138b) asks the spell's `Cast` record in the
//! history which way it was cast by, its own or a granted one (`escaped`), which `sacrifice_unless_escaped` does for
//! Kroxa, Phlage and Uro.

use crate::triggers::THIS_ENTERS;
use cantrip_api::*;

/// The escape cost: `cost` and the exiling of `exile` other cards from the graveyard (CR 702.138a).
pub const fn cost(cost: ManaCost, exile: u8) -> Cost {
    Cost { mana: cost, exile_from_graveyard: exile, ..Cost::NONE }
}

/// The alternative cost of escape: cast from the graveyard for `cost`, exiling `exile` other cards from it. `text` is the
/// way's full text.
pub const fn way(mana: ManaCost, exile: u8, text: &'static str) -> WayDef {
    cantrip_api::way(cost(mana, exile), ZoneKind::Graveyard, text).kind(WayKind::Escape)
}

/// Escape whose cost also exiles a permanent you control that passes `filter` (CR 702.138a, 118.3): "Escape--{4}{G}{U},
/// Exile a land you control, Exile five other cards from your graveyard" (Lunar Hatchling). The permanent is chosen as the
/// cost is chosen and exiled as it is paid (CR 601.2b, 601.2h). `text` is the way's full text.
pub const fn way_exiling_permanent(mana: ManaCost, exile: u8, filter: ObjectFilter, text: &'static str) -> WayDef {
    let permanents = Some(PermanentCost { count: 1, filter, action: PermanentAction::Exile, tapped: false });
    cantrip_api::way(Cost { permanents, ..cost(mana, exile) }, ZoneKind::Graveyard, text).kind(WayKind::Escape)
}

/// Escape a permanent gives cards in its controller's graveyard (CR 702.138a, 113.6, 604): "each nonland card in your
/// graveyard has escape" (Underworld Breach). `cost` is the escape cost of each card, or none for one without escape.
pub const fn granted(cost: GrantedCost) -> GrantedWay {
    GrantedWay { text: "Escape", cost, kind: WayKind::Escape, ..GrantedWay::CAST }
}

/// Escape whose exile is "any number of other cards from your graveyard with `types` or more card types among them"
/// (CR 702.138a): the cards exiled are chosen as the cost is paid. `text` is the way's full text. Nethergoyf prints it.
pub const fn way_by_types(mana: ManaCost, types: u8, text: &'static str) -> WayDef {
    cantrip_api::way(Cost { mana, variable: Some(VariableCost::ExileFromGraveyardWithTypes(types)), ..Cost::NONE }, ZoneKind::Graveyard, text).kind(WayKind::Escape)
}

/// Whether `id` escaped (CR 702.138b): it is a spell, or the permanent a spell became as it resolved (CR 400.7, 608.3), that
/// was cast from a graveyard with an escape ability - the card's own, or one a permanent gave it (Underworld Breach), never
/// another permission (the rulings: "if it was cast using any permission other than an escape ability"). Read from the
/// spell's `Cast` record: a copy of a spell was not cast (CR 707.10), and a permanent put onto the battlefield was never a
/// spell, so neither escaped.
pub fn escaped(q: &dyn Query, id: ObjectId) -> bool {
    let cast = q.history().all().iter().rev().find_map(|r| match &r.event {
        Event::Cast(c) if c.spell == id || q.obj(c.spell).became == Some(id) => Some(*c),
        _ => None,
    });
    let Some(cast) = cast.filter(|c| c.from == ZoneKind::Graveyard) else { return false };
    match cast.granted_by() {
        // A way a permanent gave casts the face for its cost; the card's own ways are not open under another permission.
        Some(grant) => grant
            .ability
            .as_granted_way()
            .and_then(|k| q.card(grant.ability.card).granted_ways.get(k))
            .is_some_and(|w| w.kind == WayKind::Escape),
        None => cast.way > 0 && q.card(q.obj(cast.spell).card).ways.get(cast.way as usize - 1).is_some_and(|w| w.kind == WayKind::Escape),
    }
}

/// "Sacrifice it unless it escaped": always triggers, and is judged as it resolves (CR 702.138b, 701.21a).
fn sacrifice_unless_escaped_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !escaped(g, item.object) && g.obj(item.object).became.is_none() {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "When this enters, sacrifice it unless it escaped": `text` names the card.
pub const fn sacrifice_unless_escaped(text: &'static str) -> AbilityDef {
    triggered(THIS_ENTERS, sacrifice_unless_escaped_effect, text)
}
