//! Continuous library-top visibility (CR 401.5, 401.6, 701.20a, 701.20d).
//! Benjamin Sisko, Besieged and Garruk's Horde share the reveal capability.
//! The API rule grants visibility; the core owns its timing and object identity.

use cantrip_api::*;
use crate::{actions, layers::live, ops, Cx, Game};

pub(crate) fn reveals(game: &Game, player: PlayerId) -> bool {
    let mut revealed = false;
    actions::each_play_rule(game, game.cards, player, false, |_, rule| {
        revealed |= matches!(rule, PlayRule::RevealLibraryTop);
    });
    revealed
}

/// CR 401.5: an unchanged top remains visible while announcing; a new top waits.
pub(crate) fn top(game: &Game, player: PlayerId) -> Option<ObjectId> {
    let top = game.state.zone(Zone::library(player)).first().copied()?;
    for frame in &game.state.agenda {
        if let Some(tops) = &frame.library_tops {
            if tops.get(player.index()).copied().flatten() != Some(top) {
                return None;
            }
        }
    }
    Some(top)
}

pub(crate) fn hold(cx: &mut Cx) -> bool {
    // Outside a frame there is no announcement that can suspend.
    if cx.s().agenda.get(cx.frame).is_some_and(|f| f.library_tops.is_none()) {
        let tops: Box<[_]> = cx.s().seats().map(|p| cx.s().zone(Zone::library(p)).first().copied()).collect();
        let frame = cx.frame;
        cx.st().update_frame(frame, |f| f.library_tops = Some(tops));
        return true;
    }
    false
}

pub(crate) fn release(cx: &mut Cx) {
    let frame = cx.frame;
    cx.st().update_frame(frame, |f| f.library_tops = None);
    sync(cx);
}

/// Maintain current continuous visibility at instruction and procedure boundaries.
/// CR 401.6 makes a card a new object when its continuous reveal is interrupted.
/// This is no zone change: no zone-change event or replacement applies.
pub(crate) fn sync(cx: &mut Cx) {
    // Moves into a library and their final ordering form one instruction.
    // Never publish the temporary insertion order inside that instruction.
    if cx.s().settle_held.get() > 0 {
        return;
    }
    // Guard on the objects and effects in this game, never on the full card registry.
    fn has_reveal(rules: &[PlayRule]) -> bool {
        rules.iter().any(|r| match r {
            PlayRule::RevealLibraryTop => true,
            PlayRule::While(_, inner) => has_reveal(inner),
            _ => false,
        })
    }
    if !cx.s().players.iter().any(|p| p.revealed_library_top.is_some())
        && !cx.s().battlefield().iter().any(|&id| cx.cards().texts(cx.s(), cx.s().obj(id)).any(|c| has_reveal(cx.cards().get(c).play_rules)))
        && !crate::layers::play_rules_given_possible(cx.cards(), cx.s())
        && !cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::PlayerPlayRule(_)))
    {
        return;
    }
    let players: Vec<_> = cx.s().seats().collect();
    for player in players {
        let old = cx.s().player(player).revealed_library_top;
        let new = if reveals(cx.g, player) { top(cx.g, player) } else { None };
        if old == new {
            continue;
        }
        set_top(cx, player, new);
    }
}

/// Shuffling or reordering a revealed card interrupts its reveal even if it ends on top (CR 701.20d).
pub(crate) fn conceal(cx: &mut Cx, player: PlayerId) {
    set_top(cx, player, None);
}

fn set_top(cx: &mut Cx, player: PlayerId, mut new: Option<ObjectId>) {
    if let Some(old) = cx.s().player(player).revealed_library_top {
        if let Some(id) = renew(cx, player, old) {
            if new == Some(old) { new = Some(id); }
        }
    }
    cx.st().update_player(player, |p| p.revealed_library_top = new);
    if let Some(card) = new {
        // The static rule reveals this card, not the spell currently resolving.
        // In particular this is not an instruction to reveal-and-move it.
        // No generic reveal trigger exists (ops::triggered_by_revealing).
        let turn = cx.s().turn;
        let serial = cx.g.log.len() as u32;
        cx.g.log.push(cantrip_api::history::Record {
            serial, turn: turn.number, step: turn.step(), active: turn.active,
            cause: cantrip_api::history::Cause::Rules, event: Event::Revealed { player, card },
        });
    }
}

/// A revealed library card reordered or concealed becomes a new object without changing zones (CR 701.20d, 401.6).
pub(crate) fn renew(cx: &mut Cx, player: PlayerId, old: ObjectId) -> Option<ObjectId> {
    let zone = Zone::library(player);
    if !live(cx.s(), old) || cx.s().obj(old).zone != zone { return None; }
    let last = ops::last_known_of(cx, old);
    let row = cx.st().last_known_push(last);
    let at = cx.s().zone(zone).iter().position(|&id| id == old).expect("a live library card");
    let obj = *cx.s().obj(old);
    let mut renewed = Obj::new(obj.card, obj.owner, zone);
    renewed.token = obj.token;
    renewed.card_copy = obj.card_copy;
    renewed.timestamp = cx.st().timestamp();
    let id = cx.st().push_obj(renewed);
    cx.st().zone_remove(zone, old);
    cx.st().zone_insert(zone, at, id);
    cx.st().update_obj(old, |o| { o.zone = Zone::GONE; o.last_known = Some(row); });
    Some(id)
}
