//! Speed (CR 702.179): a value a player can have, which start your engines! gives and the player's own inherent
//! triggered ability raises, up to the max speed of 4 that max speed abilities ask for (CR 702.178a, 702.179e).

use cantrip_api::intrinsic::{MAX_SPEED as MAX, SPEED_UP};
use cantrip_api::*;

use crate::Cx;

/// `player`'s speed increases by `amount` (CR 702.179c-d): from none it becomes `amount`, and never above 4. Nothing
/// happens to a player who has left the game, or to one already at 4.
pub fn increase(cx: &mut Cx, player: PlayerId, amount: u8) {
    let p = *cx.s().player(player);
    if p.lost || amount == 0 {
        return;
    }
    let speed = p.speed.map_or(amount, |s| s.saturating_add(amount)).min(MAX);
    if p.speed == Some(speed) {
        return;
    }
    set(cx, player, speed);
}

/// `player`'s speed is reduced by `amount`, but not below `minimum` ("This effect can't reduce their speed below 1", Spikeshell
/// Harrier). A player with no speed has none to reduce (CR 702.179b, f: it is 0 only for an effect that reads it), and a
/// player whose speed is already at or below `minimum` is unchanged, with no change to announce.
pub fn reduce(cx: &mut Cx, player: PlayerId, amount: u8, minimum: u8) {
    let p = *cx.s().player(player);
    let Some(current) = p.speed else { return };
    if p.lost || amount == 0 {
        return;
    }
    let speed = current.saturating_sub(amount).max(minimum).min(current);
    if speed != current {
        set(cx, player, speed);
    }
}

fn set(cx: &mut Cx, player: PlayerId, speed: u8) {
    cx.st().update_player(player, |pl| pl.speed = Some(speed));
    cx.emit(Event::SpeedChanged { player, speed });
}

/// State-based action (CR 704.5aa, 702.179a): a player who controls a permanent with start your engines! and has no
/// speed gets a speed of 1. A permanent that has lost its abilities has no start your engines! (CR 613.1f), and a
/// face-down one has none of its card's (CR 708.2). Only the battlefield's permanents of cards that have the keyword are
/// looked at, and only for a player with no speed. Whether anyone's speed was set.
pub fn engines_start(cx: &mut Cx) -> bool {
    let mut starting = 0u32;
    for &id in cx.s().battlefield() {
        let obj = cx.s().obj(id);
        if !cx.cards().texts(cx.s(), obj).any(|c| cx.cards().engines[c.0 as usize]) {
            continue;
        }
        let p = obj.controller;
        let player = cx.s().player(p);
        if player.lost || player.speed.is_some() || cx.lost_all_abilities(id) {
            continue;
        }
        starting |= 1 << p.index();
    }
    if starting == 0 {
        return false;
    }
    for p in cx.s().seats().collect::<Vec<_>>() {
        if starting & (1 << p.index()) != 0 {
            set(cx, p, 1);
        }
    }
    true
}

/// `player` lost life (CR 119.3). "Whenever one or more opponents lose life during your turn, if your speed is less than
/// 4, your speed increases by 1. This ability triggers only once each turn." (CR 702.179d): the inherent ability of the
/// player whose turn it is, which they have only while they have speed. It has no source and is controlled by that
/// player; it triggers once for losses that happen together (CR 603.2c) and once in the turn at all, which the history
/// is asked: it records each triggering. The intervening "if" is asked here and again as it resolves (CR 603.4).
pub(crate) fn life_lost(cx: &Cx, player: PlayerId) -> Option<(u8, PlayerId, Target)> {
    let active = cx.s().turn.active;
    let you = cx.s().player(active);
    if !cx.s().opponents(active).any(|o| o == player) || you.lost || !you.speed.is_some_and(|s| s < MAX) {
        return None;
    }
    let ability = AbilityRef { card: cx.cards().intrinsic, index: SPEED_UP };
    if crate::loops::settled_read(cx, |h| h.this_turn().iter().any(|r| matches!(r.event, Event::Triggered { ability: a, .. } if a == ability)), |&once| once) {
        return None;
    }
    Some((SPEED_UP, active, Target::None))
}
