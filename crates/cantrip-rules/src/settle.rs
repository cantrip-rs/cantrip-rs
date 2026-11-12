//! What happens each time a player would receive priority (CR 117.5): state-based actions until none apply
//! (CR 704), then triggered abilities onto the stack (CR 603.3), and again until neither does anything.

use cantrip_api::*;
use cantrip_state::{PendingTrigger, Span};

use crate::layers::{self, live};
use crate::{Cx, Flow, R, counters, ops};

/// Whether a permanent is a bestowed Aura (CR 702.103b): the Aura face its card has while it is bestowed is what its copy
/// effect says it is, until it ceases to be bestowed (CR 702.103f).
fn bestowed(cx: &Cx, id: ObjectId) -> bool {
    let obj = cx.s().obj(id);
    obj.copy.is_some_and(|c| cx.card(obj.card).bestow.is_some_and(|b| std::ptr::eq(cx.card(c.card), b)))
}

/// The verifier's watch on a settling loop (`CANTRIP_VERIFY`): every round after the first follows one that said it did
/// something, so the position has changed since the round before (`loops::position`, which looks past object identities
/// and timestamps). One that has not would go round for ever - an exiled card returned "until" over and over that could
/// never leave exile (CR 610.3, 303.4g) - and fails at once instead of hanging.
#[derive(Default)]
struct Progress(Option<Vec<u64>>);

impl Progress {
    fn check(&mut self, cx: &Cx) {
        if !cx.g.verify {
            return;
        }
        let now = crate::loops::position(cx.s());
        assert!(self.0.as_ref() != Some(&now), "settling went round again with nothing changed: it would never end");
        self.0 = Some(now);
    }
}

pub fn settle(cx: &mut Cx) -> R<Flow> {
    let mut progress = Progress::default();
    loop {
        progress.check(cx);
        // CR 610.3, 610.4: what was exiled or phased out until a permanent left has come back as it left
        // (`ops::return_until_left`); one whose permanent left where no question could be asked (a concession between runs,
        // a player leaving while their own announcement waited) comes back first here, before anything looks at the board.
        if !cx.s().is_over() && ops::returns_due(cx)? {
            continue;
        }
        // CR 603.8: a state trigger triggers as soon as its state is true, so the board a resolution leaves is asked before
        // the state-based actions change it (CR 704.3): Thespian's Stage that became a Dark Depths with no ice counters has
        // triggered even if the legend rule then puts it into the graveyard.
        state_triggers(cx);
        if state_based_actions(cx)? {
            continue;
        }
        if cx.s().is_over() {
            break;
        }
        if !cx.s().triggers.is_empty() {
            put_triggers_on_stack(cx)?;
            continue;
        }
        break;
    }
    cx.st().clear_unsettled();
    Ok(Flow::Done)
}

/// State-based actions checked and performed until none applies (CR 704.3), and no triggered ability put on the stack: the
/// check of the cleanup step (CR 514.3a) and of a turn being ended (CR 724.1c). A permanent they make leave gives back what
/// was exiled or phased out until it left once the check is done, before the next (CR 610.3, 610.4; `state_based_actions`),
/// and any such return still due is carried out first here, as in `settle`. Whether anything was done.
pub fn state_based_actions_alone(cx: &mut Cx) -> R<bool> {
    let mut acted = false;
    let mut progress = Progress::default();
    loop {
        progress.check(cx);
        if !cx.s().is_over() && ops::returns_due(cx)? {
            acted = true;
            continue;
        }
        if state_based_actions(cx)? {
            acted = true;
            continue;
        }
        break;
    }
    cx.st().clear_unsettled();
    Ok(acted)
}

/// Checks every state-based action at once and performs those that apply, simultaneously (CR 704.3), as one event: what was
/// exiled or phased out until a permanent that leaves in it left comes back once all of it is done, all together, and
/// before the next check (CR 610.3, 610.3d, 610.4, 610.4d) - not between its parts, with a loser still to leave or a
/// creature still to die. Whether any did.
pub(crate) fn state_based_actions(cx: &mut Cx) -> R<bool> {
    // The returns its moves would make at once (`ops::return_until_left`) wait for the end of the check, however it ends (a
    // question stops it; a host that catches a panic goes on with the game).
    let held = cx.s().returning.replace(true);
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_once(&mut *cx)));
    cx.s().returning.set(held);
    let acted = match done {
        Ok(acted) => acted?,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    if acted {
        ops::return_until_left(cx);
    }
    Ok(acted)
}

/// `state_based_actions`' check, its returns held.
fn check_once(cx: &mut Cx) -> R<bool> {
    verify_control(cx);
    let mut acted = false;
    let mut leavers = Vec::new();
    // CR 611.2b: an effect "for as long as" a condition ends once it is false; then, CR 613.1b: who controls each permanent
    // is what the control-changing effects that apply now say, before anything that asks it (the legend rule, an Aura's
    // legality, "you control").
    ops::settle_lasting_and_control(cx);

    // CR 704.5a, 704.5b, 704.5c: a player at 0 life, who attempted to draw from an empty library since the last check, or
    // with ten or more poison counters loses - unless a static ability says they don't lose that way (CR 104.3: Platinum
    // Angel, Phyrexian Unlife), in which case the game goes on and the draw from an empty library is forgotten with this
    // check.
    // CR 704.6c: in a Commander game, one dealt 21 or more combat damage by one commander loses too.
    let commander_beaten = ops::dealt_21_by_a_commander(cx);
    for p in cx.s().seats().collect::<Vec<_>>() {
        let player = *cx.s().player(p);
        if player.lost {
            continue;
        }
        if let Some(reason) = ops::loses_to_state_based_actions_given(cx, p, &commander_beaten) {
            cx.st().update_player(p, |pl| {
                pl.lost = true;
                pl.drew_from_empty = false;
            });
            leavers.push((p, reason));
            acted = true;
        } else if player.drew_from_empty {
            cx.st().update_player(p, |pl| pl.drew_from_empty = false);
        }
    }
    // The monarch and the initiative pass on once every loser of this check has left, so none of them is passed one on
    // the way (CR 725.4, 726.4, 800.4a).
    if !leavers.is_empty() {
        crate::designations::monarch_left(cx);
        crate::designations::initiative_left(cx);
    }
    // The game they lost is looked back on (CR 603.10f): as the board was before this check, whose actions all happen at
    // once with the losses (CR 704.3) - a permanent dying in it still sees its controller's opponent lose.
    let lost_before = (!leavers.is_empty()).then(|| (cx.s().battlefield().to_vec(), Then::now(cx)));
    // CR 903.9a: a commander that entered graveyard or exile since the previous check may move to command zone now.
    let mut pending = Vec::new();
    if cx.s().commander {
        for zone in cx.s().seats().flat_map(|p| [Zone::graveyard(p), Zone::exile(p)]) {
            pending.extend(cx.s().zone(zone).iter().copied().filter(|&id| cx.s().obj(id).commander_return_pending));
        }
    }
    let mut return_to_command = Vec::new();
    for id in pending {
        let owner = cx.s().obj(id).owner;
        cx.st().update_obj(id, |obj| obj.commander_return_pending = false);
        if !cx.s().player(owner).lost
            && matches!(cx.ask_rule(owner, RequestKind::May { source: id, purpose: Purpose::Optional })?, Some(Answer::Yes))
        {
            return_to_command.push(ops::Moving::to(id, Zone::COMMAND));
        }
    }
    // CR 704.3: the rest of this check is judged on the board as it is now, the losers' objects still on it - an anthem of
    // theirs still counts towards whether another player's creature has lethal damage - and those objects leave with their
    // owners as part of this check (CR 800.4a), below, not by any of the other actions.
    let leaving: Vec<PlayerId> = leavers.iter().map(|&(p, _)| p).collect();
    let stays = |cx: &Cx, id: ObjectId| !leaving.contains(&cx.s().obj(id).owner);

    // A prepare spell's copy ceases to exist out of exile, or once what prepared it is gone or no longer prepared
    // (CR 722.3c); on the stack it is a spell like any other.
    let stale: Vec<(Zone, ObjectId)> = cx
        .s()
        .zones
        .iter()
        .flatten()
        .copied()
        .filter(|&id| {
            let obj = cx.s().obj(id);
            obj.card_copy && !obj.zone.is(ZoneKind::Stack) && {
                let kept = obj.zone.is(ZoneKind::Exile)
                    && obj.prepared_from.is_some_and(|from| {
                        let f = cx.s().obj(from);
                        live(cx.s(), from) && f.zone.is(ZoneKind::Battlefield) && f.prepared
                    });
                !kept
            }
        })
        .map(|id| (cx.s().obj(id).zone, id))
        .collect();
    if !stale.is_empty() {
        ops::cease_together(cx, &stale);
        acted = true;
    }

    // CR 704.5d: a token anywhere but the battlefield ceases to exist; a phased-out one is still on it (CR 702.26d).
    let gone: Vec<(Zone, ObjectId)> = cx
        .s()
        .zones
        .iter()
        .flatten()
        .copied()
        .filter(|&id| cx.s().obj(id).token && !cx.s().obj(id).zone.is(ZoneKind::Battlefield) && !cx.s().obj(id).zone.is(ZoneKind::PhasedOut))
        .map(|id| (cx.s().obj(id).zone, id))
        .collect();
    if !gone.is_empty() {
        ops::cease_together(cx, &gone);
        acted = true;
    }

    // CR 702.131b, 702.195a: the city's blessing and an enduring story, for the rest of the game, by a board that changed
    // with no event (an effect ending, a type changing).
    if crate::designations::keyword_designations(cx) {
        acted = true;
    }
    // CR 704.5aa: a player with no speed who controls a permanent with start your engines! gets a speed of 1.
    if crate::speed::engines_start(cx) {
        acted = true;
    }

    // CR 704.5f, g, h, i: creatures with no toughness or lethal damage, planeswalkers with no loyalty.
    let mut aura_hosts_left = Vec::new();
    let mut to_graveyard = Vec::new();
    let mut destroyed = Vec::new();
    let mut unattach = Vec::new();
    let mut unbestow = Vec::new();
    let mut sagas = Vec::new();
    let mut legends: Vec<(ObjectId, &'static str)> = Vec::new();
    let mut worlds: Vec<(ObjectId, u32)> = Vec::new();
    let mut bound: Vec<crate::day_night::Bound> = Vec::new();
    let mut battles: Vec<(ObjectId, bool)> = Vec::new();
    let mut annihilate: Vec<(ObjectId, i32)> = Vec::new();
    let mut roles: Vec<(ObjectId, ObjectId, PlayerId, u32)> = Vec::new();
    for &id in cx.s().battlefield() {
        if !stays(cx, id) {
            continue;
        }
        // CR 704.5q: a permanent with both +1/+1 and -1/-1 counters loses N of each, N the smaller number. Power and
        // toughness are the same either way, so the other actions of this check are judged alike; removed below.
        let both = cx.s().obj(id).counters(CounterKind::PlusOne).min(cx.s().obj(id).counters(CounterKind::MinusOne));
        if both > 0 {
            annihilate.push((id, both));
        }
        let chars = cx.chars(id);
        if let Some(daybound) = crate::day_night::role(chars.keywords) {
            bound.push(crate::day_night::Bound { id, daybound });
        }
        let obj = cx.s().obj(id);
        if chars.supertypes.world { worlds.push((id, obj.world_since)); }
        // CR 704.5j: not for a player for whom an effect says the legend rule doesn't apply (Hall of Echoes), nor for a
        // permanent a static ability exempts (Mirror Gallery).
        if chars.supertypes.legendary
            && !cx.s().effects.iter().any(|e| e.kind == EffectKind::NoLegendRule(obj.controller))
            && !crate::static_rules::legend_rule_exempt(cx, id)
        {
            legends.push((id, chars.name));
        }
        if chars.has_subtype(Subtype::Saga) {
            sagas.push((id, chars));
        }
        if chars.is(CardType::Battle) {
            // CR 704.5v-w: a battle with no defense counters is put into its owner's graveyard, not destroyed. A
            // Siege is spared while it is the source of an ability that has triggered and not yet left the stack: its
            // own defeat ability (CR 310.12b).
            battles.push((id, chars.has_subtype(Subtype::Siege)));
            let pending = cx.s().triggers.iter().any(|t| t.source == id)
                || cx.s().stack.iter().any(|i| i.kind != StackKind::Spell && i.object == id);
            if cx.s().obj(id).counters(CounterKind::Defense) <= 0 && !(chars.has_subtype(Subtype::Siege) && pending) {
                to_graveyard.push(id);
                continue;
            }
        }
        // CR 704.5m: an Aura attached to an illegal object, or to nothing, goes to its owner's graveyard. CR 704.5n:
        // an Equipment or Fortification attached to an illegal permanent becomes unattached; so does anything else attached to
        // anything (CR 704.5p).
        if let Some(host) = obj.attached_to
            && chars.has_subtype(Subtype::Role)
        {
            roles.push((id, host, obj.controller, obj.timestamp));
        }
        if chars.has_subtype(Subtype::Aura) {
            if !(obj.attached_to.is_some_and(|host| crate::attach::host_allows(&*cx, id, host, false))
                || obj.attached_player.is_some_and(|p| crate::attach::player_allows(&*cx, id, p)))
            {
                // CR 702.103f: a bestowed Aura attached to an illegal object, or to nothing, becomes unattached and
                // ceases to be bestowed, a creature again, instead of going to the graveyard.
                if bestowed(&*cx, id) {
                    unbestow.push(id);
                } else {
                    // CR 400.7f: only this unattached-Aura SBA can follow an earlier departure. An illegal Aura
                    // while its host remains, a later sacrifice, and Equipment never acquire this exception.
                    if let Some(host) = obj.attached_to.filter(|&host| {
                        // The old row retains its departure zone. A card an Aura enchanted in a graveyard is not
                        // an enchanted permanent leaving the battlefield, even if that physical card was once there.
                        cx.s().obj(host).zone.is(ZoneKind::Battlefield) && !live(cx.s(), host)
                    }) {
                        aura_hosts_left.push((id, host));
                    }
                    to_graveyard.push(id);
                }
                continue;
            }
        } else if let Some(host) = obj.attached_to
            && !crate::attach::host_allows(&*cx, id, host, false)
        {
            unattach.push(id);
        }
        if chars.is(CardType::Creature) {
            let toughness = chars.toughness.unwrap_or(0);
            if toughness <= 0 {
                to_graveyard.push(id);
                continue;
            }
            if i64::from(obj.damage) >= i64::from(toughness) || obj.deathtouched {
                destroyed.push(id);
                continue;
            }
        }
        // CR 704.5i, unless a static ability says a planeswalker isn't put there for having 0 loyalty.
        if chars.is(CardType::Planeswalker)
            && obj.counters(CounterKind::Loyalty) <= 0
            && !crate::static_rules::zero_loyalty_stays(cx, id)
        {
            to_graveyard.push(id);
        }
    }
    // CR 702.145d, 702.145g, 702.145c, 702.145f: day and night, as far as the permanents with daybound or nightbound go.
    if crate::day_night::apply(cx, &bound)? {
        acted = true;
    }
    destroyed.retain(|&id| !cx.chars(id).has(Keyword::Indestructible));
    for id in unattach {
        ops::unattach(cx, id);
        acted = true;
    }
    for id in unbestow {
        ops::unattach(cx, id);
        cx.st().update_obj(id, |o| o.copy = None);
        acted = true;
    }

    // CR 714.4, 704.5s: a Saga whose lore counters have reached its final chapter is sacrificed, unless it is the
    // source of a chapter ability that has triggered and not yet left the stack. Judged before the triggers of the
    // counter that reached it go on the stack, so the last chapter always gets to resolve.
    let mut sacrificed = Vec::new();
    for (id, chars) in sagas {
        let last = ops::final_chapter(chars.abilities().map(|a| cx.cards().ability(a)));
        if last == 0 || cx.s().obj(id).counters(CounterKind::Lore) < last as i32 {
            continue;
        }
        let chapter_ability = |ability: AbilityRef| matches!(cx.cards().ability(ability), AbilityDef::Triggered(t) if t.chapter > 0);
        let pending = cx.s().triggers.iter().any(|t| t.source == id && chapter_ability(t.ability))
            || cx.s().stack.iter().any(|i| i.kind == StackKind::Triggered && i.object == id && chapter_ability(i.ability));
        // One that can't be sacrificed stays, its lore counters as they are (CR 101.2, 701.21a).
        if !pending && !crate::static_rules::cant_be_sacrificed(&*cx, cx.cards(), id) {
            sacrificed.push(id);
        }
    }

    // CR 310.11, 704.5x-y: a battle whose protector can no longer protect it, or that has none in the game, has its
    // controller choose another; nobody to choose puts it into its owner's graveyard. A battle being attacked keeps a
    // protector who has left the game until combat ends (704.5x).
    for (id, siege) in battles {
        if to_graveyard.contains(&id) {
            continue;
        }
        let controller = cx.s().obj(id).controller;
        let candidates = ops::protector_candidates(cx, siege, controller);
        let current = cx.s().obj(id).marks.protector;
        if current.is_some_and(|p| candidates.contains(&p)) {
            continue;
        }
        let unprotected = current.is_none_or(|p| cx.s().player(p).lost);
        if unprotected && cx.s().attackers.iter().any(|a| a.target == Target::Object(id)) {
            continue;
        }
        if candidates.is_empty() {
            to_graveyard.push(id);
            continue;
        }
        let protector = if candidates.len() > 1 {
            let ask = RequestKind::ChoosePlayer { source: id, candidates: candidates.clone(), verb: "Protect", amount: 0, subject: None };
            match cx.ask_rule(controller, ask)? {
                Some(Answer::Player(p)) if candidates.contains(&p) => p,
                // A controller who has left has taken the battle with them (CR 800.4a).
                None => continue,
                _ => candidates[0],
            }
        } else {
            candidates[0]
        };
        cx.st().update_obj(id, |o| o.marks.protector = Some(protector));
        // A battle whose protector changes is removed from combat (CR 506.4): the creatures attacking it attack nothing
        // (CR 506.4c).
        ops::remove_from_combat(cx, &[id]);
        acted = true;
    }

    // CR 704.5j, the legend rule: a player who controls two or more legendary permanents with the same name chooses
    // one and puts the rest into their owners' graveyards, not destroyed. Asked in turn order, a name at a time in the
    // order its first permanent entered; decided on the same board as the other actions of this check (CR 704.3),
    // so a legend that is dying anyway may still be the one kept.
    let mut surplus = Vec::new();
    // CR 704.5k: compare continuous world intervals, not layer timestamps. A tie for newest sends EVERY World to
    // the graveyard, including older ones. This is neither destruction nor sacrifice and has no player choice.
    if worlds.len() > 1 {
        let newest = worlds.iter().map(|&(_, since)| since).max().unwrap();
        let tied = worlds.iter().filter(|&&(_, since)| since == newest).count() > 1;
        surplus.extend(worlds.iter().filter_map(|&(id, since)| (tied || since != newest).then_some(id)));
    }
    if legends.len() > 1 {
        for p in cx.s().apnap().collect::<Vec<_>>() {
            let mine: Vec<(ObjectId, &'static str)> =
                legends.iter().copied().filter(|&(id, _)| cx.s().obj(id).controller == p).collect();
            let mut asked: Vec<&'static str> = Vec::new();
            for &(_, name) in &mine {
                if asked.contains(&name) {
                    continue;
                }
                asked.push(name);
                let group: Vec<ObjectId> = mine.iter().filter(|&&(_, n)| n == name).map(|&(id, _)| id).collect();
                if group.len() < 2 {
                    continue;
                }
                let ask = RequestKind::choose_cards(group[0], Purpose::KeepLegend, group.clone(), group.clone(), 1, false, "");
                let keep = match cx.ask_rule(p, ask)? {
                    Some(Answer::Cards(cards)) => cards.first().copied().unwrap_or(group[0]),
                    // A player who has left has taken their legends with them (CR 800.4a).
                    None => break,
                    _ => group[0],
                };
                surplus.extend(group.into_iter().filter(|&id| id != keep));
            }
        }
    }
    // CR 704.5z, 303.7a: of the Roles one player controls attached to one permanent, all but the one with the most
    // recent timestamp go to their owners' graveyards, judged on the same board as the rest (CR 704.3), so a Role leaving
    // for another reason in this check still counts. Roles that became attached at the same moment have their relative
    // timestamps in an order their controller would choose (CR 613.7m); the engine gives them consecutive ones, and only
    // copies of one token (a token doubler's) can arrive together on one permanent, so which is kept is the same game.
    let mut roles_surplus = Vec::new();
    for &(id, host, controller, timestamp) in &roles {
        if roles.iter().any(|&(other, h, c, t)| other != id && h == host && c == controller && t > timestamp) {
            roles_surplus.push(id);
        }
    }
    // A permanent that satisfies several of these changes zones once, and what is not destruction wins over it. A Saga is
    // sacrificed by its controller (CR 714.4, 704.5s: "that Saga's controller sacrifices it"), so "whenever you sacrifice"
    // sees it; the rest are put into graveyards, which is no sacrifice.
    let sagas_sacrificed = sacrificed.clone();
    for id in sacrificed.into_iter().chain(surplus).chain(roles_surplus) {
        if !to_graveyard.contains(&id) {
            to_graveyard.push(id);
        }
    }
    destroyed.retain(|id| !to_graveyard.contains(id));
    let mut sba_moves = return_to_command;
    if !to_graveyard.is_empty() || !destroyed.is_empty() {
        // All at once (CR 704.3): a creature that dies to its toughness and one destroyed by damage leave together.
        for id in to_graveyard.into_iter().chain(destroyed.iter().copied()) {
            // Lethal damage destroys (CR 704.5g-h): a regeneration shield may replace that (CR 701.19a, 614.8), ordered with the
            // other replacement effects (CR 616.1).
            let mut moving = ops::Moving { destroy: destroyed.contains(&id), ..ops::Moving::to(id, Zone::graveyard(cx.s().obj(id).owner)) };
            moving.aura_host_left = aura_hosts_left.iter().find_map(|&(a, h)| (a == id).then_some(h));
            if sagas_sacrificed.contains(&id) {
                moving.reason = Some(ZoneReason::Sacrifice);
            }
            sba_moves.push(moving);
        }
    }
    if !sba_moves.is_empty() {
        ops::move_together(cx, &sba_moves, ZoneReason::StateBased);
        acted = true;
    }
    // CR 704.5q, at the same time as the rest (CR 704.3): a permanent leaving in this check leaves with the counters it had.
    for (id, n) in annihilate {
        if live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
            counters::remove(cx, id, CounterKind::PlusOne, n);
            counters::remove(cx, id, CounterKind::MinusOne, n);
            acted = true;
        }
    }

    // The losers of this check leave the game, all of them, as part of it (CR 704.3, 800.4a): after the other actions of
    // the check were judged with their objects on the battlefield, and performed - so what dies in this check is seen as it
    // last existed with those objects there (CR 603.10a). The game goes on without them, or ends with the one left (CR
    // 104.2a).
    if let Some((battlefield, then)) = &lost_before {
        lost_the_game(cx, &leavers, battlefield, then);
    }
    for (p, _) in leavers {
        ops::leave_game(cx, p);
    }

    // CR 704.5t: a player on the bottommost room of a dungeon with none of its abilities on the stack completes it.
    acted |= crate::designations::complete_dungeons(cx);

    // CR 104.2a: the game ends when one player is left (or none).
    if acted {
        let left: Vec<PlayerId> = cx.s().apnap().collect();
        match left.len() {
            0 => {
                ops::end_game(cx, Outcome::Draw);
                cx.record(Event::GameDrawn { reason: DrawReason::AllLost });
            }
            1 => ops::end_game(cx, Outcome::Won(left[0])),
            _ => {}
        }
    }

    // Effects on objects that are gone apply to nothing (CR 611.2c); dropping them keeps the list short.
    // "For as long as [source] remains on the battlefield" ends with it (CR 611.2b), and "for as long as it has a [kind]
    // counter on it" with the last such counter: both the moment they end (`ops::end_departed_sources` as each event
    // happens, `ops::end_lapsed_durations` as the counter comes off); this is the backstop for any other path.
    let state = cx.s();
    let applies = |e: &cantrip_state::ContinuousEffect| {
        (live(state, e.affected) || e.kind.outlives_affected()) && !ops::source_departed(state, e) && e.duration.holds_for(state.obj(e.affected))
    };
    if state.effects.iter().any(|e| !applies(e)) {
        let alive: Vec<bool> = state.effects.iter().map(applies).collect();
        // By index: `effects_retain` visits the effects from the end, so a counter over them would mismatch.
        cx.st().effects_keep(&alive);
        // A control-changing effect that ended with them gives its permanent back now (CR 613.1b), and the state-based
        // actions are checked again with the new controllers.
        acted |= ops::settle_control(cx);
    }
    // "For as long as you control [this permanent]" permissions end with the permanent's leaving (CR 611.2b, 400.7); a
    // change of control or phasing out ends them as it happens (`settle_control`, `phase_out`).
    if !cx.s().permissions.is_empty() {
        cx.st().end_lapsed_permissions();
    }
    Ok(acted)
}

fn triggered(cx: &Cx, ability: AbilityRef) -> &'static TriggeredDef {
    match cx.cards().ability(ability) {
        AbilityDef::Triggered(t) => t,
        AbilityDef::Activated(_) | AbilityDef::Static(_) | AbilityDef::PlayRule(_) | AbilityDef::RegenerationBan(_) | AbilityDef::BlockAllowance(_) => unreachable!("a triggered ability"),
    }
}

/// Puts waiting triggered abilities on the stack (CR 603.3b): the active player's first, then each other player's
/// in turn order; a player with more than one chooses their order, and each ability's targets are chosen as it goes
/// on (CR 603.3d).
fn put_triggers_on_stack(cx: &mut Cx) -> R<()> {
    let mut waiting: Vec<PendingTrigger> = cx.s().triggers.clone();
    while !cx.s().triggers.is_empty() {
        let last = cx.s().triggers.len() - 1;
        cx.st().trigger_remove(last);
    }
    // The players still in the game, read afresh after each one's triggers: a player may leave as any of them is asked, and a
    // triggered ability a player who has left would control is not put on the stack (CR 800.4d).
    let mut done: Vec<PlayerId> = Vec::new();
    while let Some(p) = cx.s().apnap().find(|p| !done.contains(p)) {
        done.push(p);
        let mut theirs: Vec<PendingTrigger> = waiting.iter().copied().filter(|t| t.controller == p).collect();
        waiting.retain(|t| t.controller != p);
        if theirs.len() > 1 {
            let ask = RequestKind::OrderTriggers {
                sources: theirs.iter().map(|t| t.source).collect(),
                abilities: theirs.iter().map(|t| t.ability).collect(),
            };
            match cx.ask_rule(p, ask)? {
                None => continue,
                Some(Answer::Order(order)) => theirs = order.iter().map(|&i| theirs[i as usize]).collect(),
                Some(_) => {}
            }
        }
        for trigger in theirs {
            put_trigger(cx, trigger)?;
        }
    }
    Ok(())
}

/// Chooses the targets of a triggered ability's slots as it is put on the stack (CR 603.3d). Whether it can be put
/// there at all: a required slot with no legal target removes it. `all_of_few` takes every candidate of a required
/// slot without asking when there are no more than its fewest, so no choice is left (CR 601.2c).
fn choose_trigger_targets(
    cx: &mut Cx,
    trigger: &PendingTrigger,
    specs: &[TargetSpec],
    all_of_few: bool,
    targets: &mut Vec<Target>,
    of: &mut Vec<u8>,
    first_spec: u8,
) -> R<bool> {
    choose_trigger_targets_after(cx, trigger, specs, all_of_few, targets, of, first_spec, false)
}

#[allow(clippy::too_many_arguments)]
fn choose_trigger_targets_after(
    cx: &mut Cx, trigger: &PendingTrigger, specs: &[TargetSpec], all_of_few: bool,
    targets: &mut Vec<Target>, of: &mut Vec<u8>, first_spec: u8, after_divided: bool,
) -> R<bool> {
    // Asked as the ability it is (CR 607.5): one an effect gave its source reads only its own links.
    let asker = Asker { instance: None, source: trigger.source, controller: trigger.controller, by: layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired), spell: false };
    let start = targets.len();
    let slots = crate::resolve::expand_specs(cx.s(), trigger.controller, specs);
    // "If [it] was kicked, instead choose ..." (CR 702.33a, 601.2c), of the source.
    let kicked = cx.s().obj(trigger.source).kicked;
    // The text it is put on the stack with judges its targets: a reflexive or delayed ability's is the text that made it
    // (`PendingTrigger::words`, CR 603.12, 113.7a).
    let text = trigger.words.map(|w| (trigger.source, w));
    for (spec, slot_player, at) in &slots {
        let spec = &spec.when_kicked(kicked);
        let before = targets.len();
        'slot: {
            let candidates = crate::actions::target_candidates_with_text(
                cx,
                &asker,
                spec,
                if after_divided { &targets[..] } else if slots.len() > 1 { &targets[start..] } else { &[] },
                trigger.subject,
                *slot_player,
                text,
            );
            let counted = u32::from(trigger_counted(cx, trigger));
            let own = crate::resolve::Given { q: &*cx, earlier: &[], subject: Target::None, slot_player: None, gift: false, x: None, text };
            let (candidates, together) = crate::actions::within_together(&own, &asker, spec, counted, candidates);
            let (least, most) = spec.bounds(counted);
            if !crate::actions::enough_together(&own, &asker, &together, &candidates, least) {
                // Fewer legal targets than the slot's minimum: it is removed from the stack (CR 603.3d, 601.2c).
                return Ok(false);
            }
            if candidates.is_empty() {
                if spec.required() {
                    // With no legal target, it is removed from the stack (CR 603.3d).
                    return Ok(false);
                }
                break 'slot;
            }
            // A lone required candidate is no choice, for an ability with one target slot only (the C# engine asks each of several).
            if spec.min > 0 && spec.max == 1 && candidates.len() == 1 && (slots.len() == 1 || all_of_few) {
                targets.push(candidates[0]);
                break 'slot;
            }
            if all_of_few && least > 0 && candidates.len() <= least as usize && together.is_none() {
                targets.extend(candidates);
                break 'slot;
            }
            if most == 0 {
                break 'slot;
            }
            // No more than there are candidates to choose (CR 601.2c: each target is a different object or player).
            let count = most.min(cantrip_state::count_u32(candidates.len()));
            if left_the_game(cx, trigger.controller) {
                return Ok(false);
            }
            let ask = RequestKind::ChooseTarget { replacing: None, label: spec.label, slot_player: *slot_player,
                source: trigger.source,
                purpose: Purpose::Targets,
                candidates,
                optional: least == 0,
                least: u32::from(least).min(count),
                count,
                // "Any number of targets" (CR 115.1d) is said so, not as "up to" the candidates there are.
                any_number: spec.is_any_number(),
                together,
            };
            // The ability's controller chooses its targets (CR 603.3d), unless it says that player does: the one it is about.
            let chooser = match trigger.subject {
                Target::Player(p) if triggered(cx, trigger.ability).subject_chooses_targets && !cx.s().player(p).lost => p,
                _ => trigger.controller,
            };
            if let Answer::Targets(chosen) = cx.ask_core(chooser, ask, false)? {
                targets.extend(chosen);
            }
        }
        // A slot that chose nothing keeps its place (CR 601.2c, 115.1).
        if slots.len() > 1 && spec.max <= 1 && targets.len() == before {
            targets.push(Target::None);
        }
        // Each target with the spec it was chosen for (`StackItem::spec_of`), numbered across a modal ability's modes.
        of.resize(targets.len(), first_spec + at);
    }
    while targets.len() > start && targets.last() == Some(&Target::None) {
        targets.pop();
    }
    of.truncate(targets.len());
    Ok(true)
}

/// Whether `player` has left the game, by a concession made at this point of the run too (`Game::concede`): a triggered
/// ability they would control is not put on the stack (CR 800.4d).
fn left_the_game(cx: &mut Cx, player: PlayerId) -> bool {
    cx.concessions_here();
    cx.s().player(player).lost
}

/// What a triggered ability's target slots' `TargetSpec::COUNTED` read (`StackItem::counted`): the "that many" of a reflexive
/// ability (CR 603.12), the amount of the event that triggered it (`TriggeredDef::counted_by_amount`), a number read from the game
/// as it goes on the stack (`TriggeredDef::counted_by`), or else its source's X, or the
/// number of times the source was kicked (CR 702.33d, 107.3m).
fn trigger_counted(cx: &Cx, trigger: &PendingTrigger) -> u32 {
    // "Up to that many target ...", that many being the event's amount (the life gained): a count of 32 bits.
    if triggered(cx, trigger.ability).counted_by_amount {
        return trigger.data[0];
    }
    // "Up to X target ...", X read from the game now, as the ability goes on the stack (CR 603.3d, 601.2c).
    if let Some(count) = triggered(cx, trigger.ability).counted_by {
        let by = layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired);
        return count(cx, &Asker { instance: None, source: trigger.source, controller: trigger.controller, by, spell: false });
    }
    let source = cx.s().obj(trigger.source);
    trigger.count.unwrap_or(if source.x > 0 { u32::from(source.x) } else { u32::from(source.kicked) })
}

fn put_trigger(cx: &mut Cx, trigger: PendingTrigger) -> R<()> {
    // Asked as the ability it is (CR 607.5) as its modes and targets are chosen: one an effect gave its source reads only
    // its own links.
    let by = layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired);
    put_trigger_as(cx, trigger, by)
}

fn put_trigger_as(cx: &mut Cx, trigger: PendingTrigger, by: Option<Linked>) -> R<()> {
    if let Some(targets) = announce_trigger(cx, trigger, by)? {
        ops::became_targets(cx, targets.iter().copied(), trigger.source, trigger.controller, true);
        // Actual placement on the stack commits a crime (CR 603.3a, 700.13). A capability query does neither.
        ops::commit_crime(cx, trigger.controller, trigger.source, &targets);
    }
    Ok(())
}

/// Shared target/mode announcement for real triggers (CR 603.3c-d) and CR 106.7 advice copies. It does not
/// fabricate a triggering, becoming-targeted event or crime: only actual placement announces those events.
pub(crate) fn announce_trigger(cx: &mut Cx, trigger: PendingTrigger, by: Option<Linked>) -> R<Option<Vec<Target>>> {
    if left_the_game(cx, trigger.controller) {
        return Ok(None);
    }
    let def = triggered(cx, trigger.ability);
    // Its text as it is put on the stack (CR 603.3, 113.7a): its source's words now, or last, if it has left. A reflexive or
    // delayed ability's are those of the text that made it (`PendingTrigger::words`). Worked out once, for the question a
    // divided total asks and for the item alike.
    let words = trigger.words.unwrap_or_else(|| cantrip_api::words(cx.s(), trigger.source));
    // Asked as the ability it is (CR 607.5): one an effect gave its source reads only its own links.
    let asker = Asker { instance: None, source: trigger.source, controller: trigger.controller, by: layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired), spell: false };
    let mut targets = Vec::new();
    let mut of: Vec<u8> = Vec::new();
    let mut shares: Vec<u32> = Vec::new();
    let mut mask = 0u32;
    if let Some(modal) = def.modal {
        // A modal ability's modes are chosen as it is put on the stack (CR 603.3c, 700.2), then each chosen mode's
        // target, in printed order. A mode whose required target has no candidate cannot be chosen (CR 700.2a), and with
        // too few left the ability is removed. As many eligible modes as it chooses is no choice, and is not asked.
        // "Choose up to that many" (`ModalDef::COUNTED`, CR 603.12a): as many as the ability's count, read now.
        let most = modal.most(cx, &asker, trigger_counted(cx, &trigger));
        let eligible = crate::actions::eligible_modes(cx, &asker, modal);
        if eligible.len() < modal.min.max(1) as usize || most == 0 {
            return Ok(None);
        }
        let chosen = if most == modal.min && eligible.len() == modal.min as usize {
            eligible
        } else {
            if left_the_game(cx, trigger.controller) {
                return Ok(None);
            }
            let ask = RequestKind::ChooseModes {
                source: trigger.source,
                modes: (0..modal.modes.len() as u8).map(|i| Label::Mode(trigger.ability, i)).collect(),
                eligible: eligible.clone(),
                count: modal.min,
                at_most: if most > modal.min { most } else { 0 },
                affordable: Vec::new(),
                additional_cost: false,
                repeat: modal.repeat,
            };
            match cx.ask_core(trigger.controller, ask, false)? {
                Answer::Modes(modes) => modes,
                _ => eligible,
            }
        };
        for &n in &chosen {
            mask = modal.with(mask, n as usize);
        }
        // "Choose up to": with no mode chosen the ability is removed from the stack as it is put there (CR 603.3c,
        // 700.2b), so it never stays there for anyone to respond to, copy or count. No mode was chosen, so nothing is
        // recorded as chosen.
        if mask == 0 {
            return Ok(None);
        }
        // Chosen as it goes on the stack: chosen even if the ability is later countered (CR 603.3c, 700.2).
        cx.emit(Event::ModesChosen { player: trigger.controller, source: trigger.source, ability: trigger.ability, modes: mask });
        let mut first_spec = 0u8;
        for (n, mode) in modal.modes.iter().enumerate() {
            for _ in 0..modal.times(mask, n) {
                if !choose_trigger_targets(cx, &trigger, mode.targets, true, &mut targets, &mut of, first_spec)? {
                    return Ok(None);
                }
                first_spec += mode.targets.len() as u8;
            }
        }
    } else if let Some(divide) = def.divide
        && let Some(spec) = def.targets.first()
    {
        // CR 601.2d, 603.3d: a divided slot's targets and shares are chosen as the ability goes on the stack.
        let x = trigger.count.unwrap_or(cx.s().obj(trigger.source).x);
        let total = divide.total_for(&*cx, &Asker { instance: None, source: trigger.source, controller: trigger.controller, by, spell: false }, x);
        if left_the_game(cx, trigger.controller) {
            return Ok(None);
        }
        match crate::actions::ask_divided(cx, trigger.controller, trigger.source, false, spec, total, None, false, by, Some((trigger.ability, words)))? {
            crate::actions::Divided::Chosen(chosen, given) => {
                targets = chosen;
                shares = given;
            }
            crate::actions::Divided::Skipped => {}
            crate::actions::Divided::Impossible => return Ok(None),
        }
        of.resize(targets.len(), 0);
        if !choose_trigger_targets_after(cx, &trigger, &def.targets[1..], false, &mut targets, &mut of, 1, true)? {
            return Ok(None);
        }
    } else if !choose_trigger_targets(cx, &trigger, def.targets, false, &mut targets, &mut of, 0)? {
        return Ok(None);
    }
    let mut item = StackItem { instance: AbilityAcquisition::OWN, event_amount: trigger.event_amount,
        kind: StackKind::Triggered,
        object: trigger.source,
        ability: trigger.ability,
        controller: trigger.controller,
        inline_targets: [Target::None; cantrip_state::INLINE_TARGETS],
        target_count: 0,
        inline_spec_of: [0; cantrip_state::INLINE_TARGETS],
        x: trigger.count.unwrap_or(0),
        kicked: 0,
        counted: trigger_counted(cx, &trigger),
        paid: 0,
        modes: mask,
        way: 0,
        choice: 0,
        subject: trigger.subject,
        data: trigger.data,
        members: trigger.members,
        serial: 0,
        // CR 603.10, 702.174b: what was true of the source as the ability triggered, for a gift triggered ability to
        // give its gift after the permanent has left.
        gift: cx.s().obj(trigger.source).gift,
        shares: StackItem::NO_SHARES,
        spill: 0,
        exile_instead: ExileInstead::Never,
        flashback: false,
        attacking: false,
        // Put on the stack now, or, a delayed triggered ability, created when its `Delayed` was (CR 701.27f).
        since: trigger.since.unwrap_or(cx.g.log.len() as u32),
        tapped_since: cx.g.log.len() as u32,
        copied: false,
        cast_known: u32::MAX,
        acquired: trigger.acquired,
        words,
    };
    // Its controller left as one of its choices was asked (CR 800.4d).
    if cx.s().player(trigger.controller).lost {
        return Ok(None);
    }
    cx.st().set_item_targets(&mut item, &targets, &of, &shares);
    cx.st().stack_push(item);
    Ok(Some(targets))
}

/// Effects that made one triggering trigger additional times (CR 603.2d), identified independently so each counts once
/// even when met again among a batch's events: a static ability's permanent and slot, or a resolved effect's timestamp.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdditionalTrigger {
    Static(ObjectId, u8),
    Resolved(u32),
}
pub(crate) type Again = Vec<AdditionalTrigger>;

type Found = Vec<(PendingTrigger, &'static TriggeredDef, Again)>;

/// The triggered abilities of `source` that `event` triggers, onto `found`: those that look back on it (CR 603.10,
/// `CardTable::looks_back`) or those that don't, as `look_back` says. With `later`, both, the object's characteristics
/// found once: the look-back ones onto `found`, the others onto `later`.
///
/// `permanent`: the source is on the battlefield (or was, for a look-back ability); false for an emblem in the command zone,
/// which is no permanent (CR 114.4), so "a triggered ability of a permanent" does not mean its abilities.
///
/// `then`: the source as it was just before the events, for a look-back pass over the permanents that were there. `board`:
/// the battlefield then, which each of its abilities that looks back on the event is asked of (`BoardThen`, CR 603.10a).
#[allow(clippy::too_many_arguments)]
fn consider(
    cx: &Cx,
    source: ObjectId,
    look_back: bool,
    permanent: bool,
    event: &Event,
    seen: bool,
    mask: EventMask,
    statics: &mut TriggerStatics,
    found: &mut Found,
    mut later: Option<&mut Found>,
    then: Option<&Then>,
    board: Option<&Then>,
) {
    // A look-back pass judges the object as it was just before the event (CR 603.10a): its characteristics then, and who
    // controlled it then, who controls the ability (CR 603.3a).
    // One that could have no look-back ability then (`Then`) looks back with none.
    let (chars, controller) = match then.and_then(|t| t.of(source)) {
        Some(Some((chars, controller))) => (chars, controller),
        Some(None) => return,
        None => (layers::chars(cx.cards(), cx.s(), &cx.g.log, source), cx.s().obj(source).controller),
    };
    // How it acquired each ability (CR 607.5), as it was then where it looks back (`Then`): worked out only for one that
    // triggers.
    let taken = then.and_then(|t| t.acquired(source));
    let acquired_at = |nth: usize| match &taken {
        Some((text, slots)) => slots.as_ref().map_or(*text, |a| a[slot_of(&chars, nth)]),
        None => layers::acquired(cx.cards(), cx.s(), &cx.g.log, source, &chars, slot_of(&chars, nth)),
    };
    // Each ability is asked as the ability it is (CR 607.5): one an effect gave the object reads only its own links. Whether
    // any could have been given is found out once, and only for an ability that is asked.
    let given = std::cell::OnceCell::new();
    // An ability an effect gave the object is the giving text's, and reads its words (`layers::granted_words`, CR 611.2c,
    // 612.1, 612.3): its givers are worked out once, only where one may have been given.
    // Nothing in the game could have given an ability, nearly always: then none is looked for.
    let possible = statics.acquisition_possible(cx);
    let givers = std::cell::OnceCell::new();
    for (nth, ability) in chars.abilities().enumerate() {
        // Only a triggered ability is asked anything here.
        let AbilityDef::Triggered(_) = cx.cards().ability(ability) else { continue };
        let by = || {
            let given = *given.get_or_init(|| taken.is_some() || possible);
            if given { layers::given_link(cx.cards(), cx.s(), source, ability, acquired_at(nth)) } else { None }
        };
        let words = if possible && !layers::surely_its_text(cx.cards(), &chars, slot_of(&chars, nth)) {
            let slot = slot_of(&chars, nth);
            layers::granted_words(cx.s(), source, ability, givers.get_or_init(|| layers::ability_givers(cx.cards(), cx.s(), &cx.g.log, source))[slot])
        } else {
            None
        };
        let given_text = crate::resolve::Given {
            q: cx,
            earlier: &[],
            subject: Target::None,
            slot_player: None,
            gift: false,
            x: None,
            text: words.map(|w| (source, w)),
        };
        // Asked through the game itself where its text is its object's own, which is nearly always.
        let own: &dyn Query = if words.is_some() { &given_text } else { cx };
        // A look-back ability is asked of the game as it was just before the events (CR 603.10a, `BoardThen`): the
        // permanents that stay on the battlefield as they were then, not as the events left them.
        let board_then;
        let own: &dyn Query = match board {
            Some(then) if cx.cards().looks_back(ability, event, seen) => {
                board_then = BoardThen { q: own, then };
                &board_then
            }
            _ => own,
        };
        // A state trigger (CR 603.8) triggers as soon as its state is true, so it is asked after every event, of the board as
        // it is now (never of a look-back pass), as well as each time a player would receive priority (`state_triggers`).
        if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
            && let Some(state) = t.state
        {
            if (later.is_some() || !look_back)
                && state(own, &Asker { instance: None, source, controller, by: by(), spell: false })
                && !state_trigger_waiting(cx, source, ability, found, later.as_deref())
            {
                found.push((state_trigger(source, ability, controller, acquired_at(nth)), t, Again::new()));
            }
            continue;
        }
        if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
            // A permanent's ability functions only if it functions on the battlefield (CR 113.6): "when you cast this spell"
            // doesn't (CR 113.6k). An emblem's function in the command zone (CR 113.6p).
            && (!permanent || t.functions.contains(ZoneKind::Battlefield))
            && (later.is_some() || cx.cards().looks_back(ability, event, seen) == look_back)
            && let Some(fired) = if t.chapter > 0 {
                chapter_reached(cx, source, &chars, t.chapter, event)
            } else if t.attack_option.is_some() {
                paid_to_attack(source, ability, chars.instance(nth), event)
            } else if let Some(option) = t.cast_option {
                paid_to_cast(cx, source, &chars, nth, option, event)
            } else if t.listens.intersects(mask) {
                t.fires(own, &Asker { instance: None, source, controller, by: by(), spell: false }, event)
            } else {
                None
            }
        {
            if let Some(condition) = t.condition
                && !condition(own, &Asker { instance: None, source, controller, by: by(), spell: false })
            {
                continue;
            }
            // "Ward abilities of those creatures don't trigger" (CR 702.21a, Nowhere to Run): asked only of a ward that matched.
            if t.ward && layers::targeting_rule(cx.cards(), cx.s(), &cx.g.log, TargetingRuleKind::SuppressWard, source) {
                continue;
            }
            let looks_back = cx.cards().looks_back(ability, event, seen);
            let triggering = Triggering { source, controller, def: t, permanent, looks_back };
            if statics.stop(cx, event, &triggering) {
                continue;
            }
            let mut again = Again::new();
            statics.additional(cx, event, &triggering, &mut again);
            let into = match later.as_deref_mut() {
                Some(later) if !looks_back => later,
                _ => &mut *found,
            };
            into.push((
                PendingTrigger { event_amount: matched_damage_amount(t, event),
                    source,
                    ability,
                    controller,
                    subject: fired.subject,
                    data: fired.data,
                    members: Span::default(),
                    count: None,
                    since: None,
                    acquired: acquired_at(nth),
                    words,
                },
                t,
                again,
            ));
        }
    }
}

/// One of the players' inherent triggered abilities (`designations::inherent_triggers`) that `event` triggered: from an object of
/// the rules' own in no zone (`designations::queue`), controlled by `controller` and about `subject`, unless something stops it
/// triggering (CR 603.2), and once more for each static ability that makes it trigger again (CR 603.2d). The initiative's "one
/// or more creatures a player controls" triggers once for that player in a batch (CR 603.2c); speed's once each turn, its
/// triggering noted in the history - an ability stopped from triggering has not triggered.
fn inherent(cx: &mut Cx, index: u8, controller: PlayerId, subject: Target, event: &Event, seen: bool, statics: &mut TriggerStatics, found: &mut Found) {
    let card = cx.cards().intrinsic;
    let ability = AbilityRef { card, index };
    let AbilityDef::Triggered(t) = cx.cards().ability(ability) else { return };
    let same = |p: &PendingTrigger| p.ability == ability && p.subject == subject;
    // Once for each player whose creatures dealt the damage in this batch (CR 726.2, 603.2c).
    if index == cantrip_api::intrinsic::INITIATIVE_TAKEN && found.iter().any(|(p, _, _)| same(p)) {
        return;
    }
    if index == cantrip_api::intrinsic::SPEED_UP && found.iter().any(|(p, _, _)| p.ability == ability) {
        return;
    }
    // Its source, in no zone: what stops abilities triggering asks about it (CR 603.2d), and is given it back if it does.
    let source = cx.st().push_obj(Obj::new(card, controller, Zone::GONE));
    let triggering = Triggering { source, controller, def: t, permanent: false, looks_back: cx.cards().looks_back(ability, event, seen) };
    if statics.stop(cx, event, &triggering) {
        cx.st().pop_obj(source);
        return;
    }
    let mut again = Again::new();
    statics.additional(cx, event, &triggering, &mut again);
    if index == cantrip_api::intrinsic::SPEED_UP {
        cx.record(Event::Triggered { source, ability });
    }
    found.push((PendingTrigger { event_amount: None, source, ability, controller, subject, data: [0; 2], members: Span::default(), count: None, since: None, acquired: Acquired::OWN, words: None }, t, again));
}

/// The slot of `chars`'s abilities that the `nth` of `Chars::abilities()` is in.
fn slot_of(chars: &Chars, nth: usize) -> usize {
    chars.abilities.iter().enumerate().filter(|(_, a)| a.is_some()).nth(nth).map_or(0, |(slot, _)| slot)
}

/// A delayed triggered ability as linked abilities know it (CR 607.5), where that differs from an ability of its source's
/// text: of the card whose ability created it, acquired as that ability was (`Delayed::acquired`).
fn delayed_link(cx: &Cx, d: &cantrip_state::Delayed) -> Option<Linked> {
    layers::given_link(cx.cards(), cx.s(), d.source, d.ability, d.acquired)
}

/// The triggering of a state trigger (CR 603.8): about no event, so nothing is its subject.
fn state_trigger(source: ObjectId, ability: AbilityRef, controller: PlayerId, acquired: Acquired) -> PendingTrigger {
    PendingTrigger { event_amount: None, source, ability, controller, subject: Target::None, data: [0; 2], members: Span::default(), count: None, since: None, acquired, words: None }
}

/// Whether a state trigger of `source` has triggered and not yet left the stack (CR 603.8): found for this batch, waiting
/// to be put on the stack, or on it. Until then it doesn't trigger again. A copy of it on the stack (Return the Favor) is
/// not it: it was not triggered (CR 707.10). An object that has changed zones is a new object (CR 400.7), whose ability
/// is another.
fn state_trigger_waiting(cx: &Cx, source: ObjectId, ability: AbilityRef, found: &Found, later: Option<&Found>) -> bool {
    let this = |t: &PendingTrigger| t.source == source && t.ability == ability;
    found.iter().chain(later.into_iter().flatten()).any(|(t, _, _)| this(t))
        || cx.s().triggers.iter().any(this)
        || cx.s().stack.iter().any(|i| i.kind == StackKind::Triggered && !i.copied && i.object == source && i.ability == ability)
}

/// The state triggers whose states are true each time a player would receive priority (CR 603.8, 117.5): of the permanents
/// and of the emblems in the command zone (CR 114.4). A state that became true with no event to ask it after, such as an
/// effect ending, is found here.
fn state_triggers(cx: &mut Cx) {
    let mut found: Found = Vec::new();
    let sources: Vec<ObjectId> = cx.s().battlefield().iter().chain(cx.s().zone(Zone::COMMAND)).copied().collect();
    for source in sources {
        let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, source);
        let controller = cx.s().obj(source).controller;
        let acquired_at = |nth: usize| layers::acquired(cx.cards(), cx.s(), &cx.g.log, source, &chars, slot_of(&chars, nth));
        for (nth, ability) in chars.abilities().enumerate() {
            if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
                && let Some(state) = t.state
                && state(cx, &Asker { instance: None, source, controller, by: cx.link_nth(source, &chars, nth), spell: false })
                && !state_trigger_waiting(cx, source, ability, &found, None)
            {
                found.push((state_trigger(source, ability, controller, acquired_at(nth)), t, Again::new()));
            }
        }
    }
    for (trigger, _, _) in found {
        cx.st().trigger_push(trigger);
    }
}

/// The state triggers whose states are true now, in the middle of a resolution, as an effect begins (CR 603.8: a state trigger
/// "triggers as soon as the game state matches the condition"): a change that records no event, after which no event asks the
/// conditions, so a state true only until the next such change would be missed. Only the permanents and emblems that could
/// have one are asked: those whose card as it is now (a face, a copy) prints one or gives one (`CardDef::delayed`), those a
/// copy effect's exception or a resolved effect gives an ability that is one - and every one of them while a static ability
/// that gives objects abilities is on the board, as what it gives is its own to say.
pub(crate) fn state_triggers_now(cx: &mut Cx) {
    let is_state = |cx: &Cx, a: AbilityRef| matches!(cx.cards().ability(a), AbilityDef::Triggered(t) if t.state.is_some());
    let all = layers::ability_grants_present(cx.cards(), cx.s());
    let may_have = |cx: &Cx, id: ObjectId| {
        let obj = cx.s().obj(id);
        all || cx.cards().texts(cx.s(), obj).any(|c| cx.cards().state_triggers[c.0 as usize])
            || obj.copy.is_some_and(|c| c.extra_abilities.iter().any(|e| e.instances > 0 && cx.cards().state_triggers[e.card.0 as usize]))
            || cx.s().effects.iter().any(|e| e.affected == id && matches!(e.kind, EffectKind::AddAbility(a) if is_state(cx, a)))
    };
    let sources: Vec<ObjectId> =
        cx.s().battlefield().iter().chain(cx.s().zone(Zone::COMMAND)).copied().filter(|&id| may_have(cx, id)).collect();
    let mut found: Found = Vec::new();
    // Each state trigger's own text, its source's now, though a resolving ability's instructions are under way (`Cx::own_text`).
    let others = cx.others_text();
    for source in sources {
        let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, source);
        let controller = cx.s().obj(source).controller;
        let acquired_at = |nth: usize| layers::acquired(cx.cards(), cx.s(), &cx.g.log, source, &chars, slot_of(&chars, nth));
        for (nth, ability) in chars.abilities().enumerate() {
            if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
                && let Some(state) = t.state
                && state(cx, &Asker { instance: None, source, controller, by: cx.link_nth(source, &chars, nth), spell: false })
                && !state_trigger_waiting(cx, source, ability, &found, None)
            {
                found.push((state_trigger(source, ability, controller, acquired_at(nth)), t, Again::new()));
            }
        }
    }
    drop(others);
    for (trigger, _, _) in found {
        cx.st().trigger_push(trigger);
    }
    cx.state_judged();
}

/// The "when you do" ability linked to an optional cost to attack (CR 607.2h, 508.1g; `TriggeredDef::attack_option`)
/// triggers when that cost is paid: the payment of this instance of this ability of `source`, and no other (CR 702.154d).
/// About the creature the cost tapped, if it tapped one.
fn paid_to_attack(source: ObjectId, ability: AbilityRef, instance: u8, event: &Event) -> Option<Fired> {
    match *event {
        Event::PaidToAttack { attacker, ability: paid, instance: nth, tapped, .. } if attacker == source && paid == ability && nth == instance => {
            Fired::about(if tapped == ObjectId::NONE { Target::None } else { Target::Object(tapped) })
        }
        _ => None,
    }
}

/// The "when you cast this spell, if its cost was paid" ability linked to an optional cost to cast (CR 607.2h, 601.2b;
/// `TriggeredDef::cast_option`) triggers when that cost was paid: the payment of this instance of it, the `nth` of the spell's
/// abilities, and no other (CR 702.153b). Its bit of the `SpellCast`'s `options` is its place among the abilities that have one.
/// A cost that may be paid any number of times (`option`, CR 702.56a) triggers with how many times this instance's was paid
/// (`Event::PaidRepeatedly`, recorded with the cast) as its first datum: each instance on its own payments (CR 702.56b).
fn paid_to_cast(cx: &Cx, source: ObjectId, chars: &Chars, nth: usize, option: OptionalCost, event: &Event) -> Option<Fired> {
    match *event {
        Event::SpellCast { spell, options, .. } if spell == source => {
            let place = chars.abilities().take(nth).filter(|&a| matches!(cx.cards().ability(a), AbilityDef::Triggered(t) if t.cast_option.is_some())).count();
            if !options.checked_shr(place as u32).is_some_and(|o| o & 1 == 1) {
                return None;
            }
            if !option.repeatable {
                return Fired::PLAIN;
            }
            let times = cx.g.log.iter().rev().find_map(|r| match r.event {
                Event::PaidRepeatedly { spell, place: paid, times } if spell == source && usize::from(paid) == place => Some(times),
                _ => None,
            })?;
            Some(Fired { subject: Target::None, data: [times, 0] })
        }
        _ => None,
    }
}

/// A Saga's chapter ability (CR 714.2b) triggers when lore counters are put on it and their number becomes at least
/// its chapter number, having been less than it. One of a Saga with read ahead (`chars`, its characteristics as the event
/// is judged) can't trigger the turn the Saga entered the battlefield unless it then has exactly the chapter's number of
/// lore counters (CR 702.155a): the chapters a read-ahead Saga skips as it enters with several don't trigger, and nor do
/// those more counters put on it that turn pass over. The history says whether it entered this turn, asked only of a
/// chapter that would otherwise trigger.
fn chapter_reached(cx: &Cx, source: ObjectId, chars: &Chars, chapter: u8, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, kind: CounterKind::Lore, amount, total, .. } if *object == source => {
            let chapter = i32::from(chapter);
            if total - amount >= chapter || *total < chapter {
                return None;
            }
            // The whole history so far: the move that brought it is in the batch being judged, which `Query::history`
            // leaves out while triggers are found (`Game::history_end`).
            if *total != chapter && chars.has(Keyword::ReadAhead) && History::with_receipts(&cx.g.log, cx.s().turn.number, &cx.s().cost_receipts).entered_this_turn(source) {
                return None;
            }
            Fired::PLAIN
        }
        _ => None,
    }
}

/// The permanents with static abilities that stop abilities from triggering (CR 603.2, `CardDef::trigger_rules`) or make them
/// trigger additional times (CR 603.2d, `CardDef::additional_triggers`): found from the board, once per batch and only when an
/// ability has matched, so a batch that triggers nothing pays nothing. An ability that triggers on the board as it is after the
/// events asks those on the battlefield now; one that looks back on its event (CR 603.10, `CardTable::looks_back`), a
/// leaves-the-battlefield ability's, asks those that were there before, the ones leaving with the event included.
pub(crate) struct TriggerStatics<'a> {
    before: Option<&'a [ObjectId]>,
    now: Option<Vec<ObjectId>>,
    then: Option<Vec<ObjectId>>,
    before_effects: Option<&'a [ContinuousEffect]>,
    /// Whether anything could have given an object an ability (`layers::acquisition_possible`), found once for the look.
    acquisition: Option<bool>,
}

impl<'a> TriggerStatics<'a> {
    pub(crate) fn new(before: Option<&'a [ObjectId]>) -> Self {
        TriggerStatics { before, now: None, then: None, before_effects: None, acquisition: None }
    }

    /// `layers::acquisition_possible`, asked once for the look: where nothing could have given an ability, every ability is
    /// its object's own text, and no granted words are looked for.
    fn acquisition_possible(&mut self, cx: &Cx) -> bool {
        *self.acquisition.get_or_insert_with(|| layers::acquisition_possible(cx.cards(), cx.s()))
    }

    /// The permanents whose rules apply to this triggering (CR 603.10).
    fn sources(&mut self, cx: &Cx, triggering: &Triggering) -> &[ObjectId] {
        if self.before.is_some() && triggering.looks_back {
            let before = self.before.unwrap_or_default();
            self.then.get_or_insert_with(|| trigger_rule_sources(cx, before))
        } else {
            self.now.get_or_insert_with(|| trigger_rule_sources(cx, cx.s().battlefield()))
        }
    }

    /// Whether a static ability stops this ability triggering on this event.
    pub(crate) fn stop(&mut self, cx: &Cx, event: &Event, triggering: &Triggering) -> bool {
        self.sources(cx, triggering).iter().any(|&id| {
            let obj = cx.s().obj(id);
            cx.cards().texts_linked(cx.s(), id).any(|(c, by)| {
                let asker = Asker { instance: None, source: id, controller: obj.controller, by, spell: false };
                cx.card(c).trigger_rules.iter().any(|rule| rule(cx, &asker, event, triggering))
            })
        })
    }

    /// The static abilities that make this ability, which this event triggered, trigger an additional time (CR 603.2d): one
    /// more for each, added to `again` unless already there.
    pub(crate) fn additional(&mut self, cx: &Cx, event: &Event, triggering: &Triggering, again: &mut Again) {
        for &id in self.sources(cx, triggering) {
            let obj = cx.s().obj(id);
            // Numbered across the definitions it has rules from (CR 702.140e), each rule its own, asked as its own card's.
            let rules = cx.cards().texts_linked(cx.s(), id).flat_map(|(c, by)| cx.card(c).additional_triggers.iter().map(move |r| (r, by)));
            for (n, (rule, by)) in rules.enumerate() {
                let asker = Asker { instance: None, source: id, controller: obj.controller, by, spell: false };
                if rule(cx, &asker, event, triggering) && !again.contains(&AdditionalTrigger::Static(id, n as u8)) {
                    again.push(AdditionalTrigger::Static(id, n as u8));
                }
            }
        }
        // A resolved effect works after its source loses its abilities (CR 611.2a). Each activation is independent;
        // a look-back trigger uses the continuous effects before the event (CR 603.10a).
        if triggering.permanent {
            let effects = if triggering.looks_back { self.before_effects.unwrap_or(&cx.s().effects) } else { &cx.s().effects };
            for e in effects {
                if e.affected == triggering.source && e.kind == EffectKind::TriggersAgainWhileSourceTapped {
                    let key = AdditionalTrigger::Resolved(e.timestamp);
                    if !again.contains(&key) {
                        again.push(key);
                    }
                }
            }
        }
    }
}

/// Those of `ids` whose card has a rule that stops abilities from triggering or makes them trigger again, and has not lost
/// its abilities (CR 613.1f).
fn trigger_rule_sources(cx: &Cx, ids: &[ObjectId]) -> Vec<ObjectId> {
    ids.iter()
        .copied()
        .filter(|&id| cx.cards().texts(cx.s(), cx.s().obj(id)).any(|c| cx.cards().trigger_rules[c.0 as usize]) && !cx.lost_all_abilities(id))
        .collect()
}

/// The battlefield just before a batch of events (CR 603.10a), and what its permanents that could look back were then:
/// what its look-back abilities are judged on.
#[derive(Clone, Copy)]
pub struct Before<'a> {
    pub battlefield: &'a [ObjectId],
    pub then: &'a Then,
    /// The objects put into a hand or a library that every player could see as they moved (CR 603.10a, `ops::seen_by_all`),
    /// by their old ids: decided as the moves were made, which the objects left behind can't show.
    pub seen_by_all: &'a [ObjectId],
}

/// The permanents that may have a look-back ability, with their characteristics and controller as they are before a batch
/// of events (CR 603.10a), taken before the events happen (`Then::now`): every permanent that could have such an ability
/// then - one its card could have of its own (printed, listed to give, or given by its own static ability; its own card, a
/// copy's or its face-down values), one a copy effect's exception added, one a static ability that gives abilities
/// anywhere or an effect could have given it, one from a keyword counter (CR 122.1b) - and any other has none to look back
/// with. `Then::none` takes nothing, and the look-back pass then judges each permanent as it is now.
///
/// The whole battlefield is taken as well (`board`), and every look-back ability is asked of it (`BoardThen`): a condition
/// that asks about other permanents (Kraven the Hunter's "the greatest power among creatures that player controls",
/// Costume Closet's "modified creature": an Equipment attached then) is judged on the game as it was then, the permanents
/// that stay on the battlefield included. Without it, one that stays could only be judged as it is now, after the events:
/// a creature that a lord leaving with the event had pumped has lost the bonus by then, though the rule asks for the power
/// it had just before. The departed rows (`LastKnown`) are no help there - only an object that left has one - and the
/// history records the events, not every permanent's characteristics between them.
pub struct Then {
    taken: bool,
    /// Resolved trigger multipliers before the event, including ones whose source leaves with it (CR 603.10a).
    trigger_effects: Vec<ContinuousEffect>,
    /// By object, for `of`.
    objects: Vec<(ObjectId, Chars, PlayerId)>,
    /// By object, parallel to `objects`: how it had acquired its abilities then (CR 607.5; `layers::acquisitions`), the text
    /// it had and each slot's where an effect could have given one. Empty where nothing could have (`acquisition_possible`).
    acquired: Vec<Taken>,
    /// Every permanent on the battlefield then, by object, with its characteristics, its controller and what it was
    /// attached to (`BoardThen`).
    board: Vec<(ObjectId, Chars, PlayerId, Option<ObjectId>)>,
    /// The battlefield then, in its order.
    battlefield: Vec<ObjectId>,
}

/// How an object had acquired its abilities as a `Then` was taken: by the text it had, and each slot's where an effect could
/// have given one (`layers::acquisitions`).
type Taken = (Acquired, Option<Box<[Acquired; cantrip_state::MAX_ABILITIES]>>);

impl Then {
    /// Whether anything was taken (`now`), rather than `none`.
    pub(crate) fn taken(&self) -> bool {
        self.taken
    }

    /// What was taken of `id`: `None` if nothing was taken (judge it now), `Some(None)` if it could look back with nothing.
    fn of(&self, id: ObjectId) -> Option<Option<(Chars, PlayerId)>> {
        if !self.taken {
            return None;
        }
        Some(self.objects.binary_search_by_key(&id, |&(o, _, _)| o).ok().map(|at| (self.objects[at].1, self.objects[at].2)))
    }

    /// How `id` had acquired its abilities, if it was taken: all its own where nothing could have given one then.
    fn acquired(&self, id: ObjectId) -> Option<Taken> {
        let at = self.objects.binary_search_by_key(&id, |&(o, _, _)| o).ok()?;
        Some(self.acquired.get(at).cloned().unwrap_or((Acquired::OWN, None)))
    }

    /// The game as it was then, for a look-back ability to be asked of (`BoardThen`): `None` where nothing was taken.
    fn board(&self) -> Option<&Then> {
        self.taken.then_some(self)
    }

    /// `id` as it was on the battlefield then, if it was there.
    fn on_board(&self, id: ObjectId) -> Option<&(ObjectId, Chars, PlayerId, Option<ObjectId>)> {
        self.board.binary_search_by_key(&id, |r| r.0).ok().map(|at| &self.board[at])
    }

    /// Nothing taken: the look-back pass judges the permanents as they are now.
    pub(crate) fn none() -> Then {
        Then { taken: false, trigger_effects: Vec::new(), objects: Vec::new(), acquired: Vec::new(), board: Vec::new(), battlefield: Vec::new() }
    }

    /// The whole battlefield as it is now, and which of its permanents could look back: called just before the events.
    pub(crate) fn now(cx: &Cx) -> Then {
        let cards = cx.cards();
        let s = cx.s();
        let could = could_have(cx, &cards.look_back, TriggeredDef::may_look_back);
        let battlefield = s.battlefield().to_vec();
        let mut board: Vec<(ObjectId, Chars, PlayerId, Option<ObjectId>)> =
            battlefield.iter().map(|&id| (id, layers::chars(cards, s, &cx.g.log, id), s.obj(id).controller, s.obj(id).attached_to)).collect();
        board.sort_unstable_by_key(|r| r.0);
        let objects: Vec<(ObjectId, Chars, PlayerId)> = board.iter().filter(|r| could(r.0)).map(|&(id, chars, controller, _)| (id, chars, controller)).collect();
        // With `CANTRIP_VERIFY`: a permanent left out has no look-back ability now - a way of giving one that the set above
        // misses is found here, not as a trigger that silently never happens.
        if layers::verifying() {
            for &(id, chars, _, _) in board.iter().filter(|r| !could(r.0)) {
                assert!(
                    !chars.abilities().any(|a| matches!(cards.ability(a), AbilityDef::Triggered(t) if t.may_look_back())),
                    "{id:?} has a look-back ability that nothing counted as able to give it (CR 603.10a)"
                );
            }
        }
        // How each had acquired its abilities (CR 607.5), where anything could have given one: else all are its own.
        let acquired = if layers::acquisition_possible(cards, s) {
            objects.iter().map(|(id, chars, _)| (s.copy_acquired(*id), layers::acquisitions(cards, s, &cx.g.log, *id, chars))).collect()
        } else {
            Vec::new()
        };
        let trigger_effects = s.effects.iter().filter(|e| e.kind == EffectKind::TriggersAgainWhileSourceTapped).copied().collect();
        Then { taken: true, trigger_effects, objects, acquired, board, battlefield }
    }
}

/// The game as a look-back ability asks it (CR 603.10a: "the game state before the event"): the permanents on the
/// battlefield just before the events (`Then::board`) - what is on it, with each one's characteristics, controller and
/// attachment then - and everything else as `q` has it.
struct BoardThen<'a> {
    q: &'a dyn Query,
    then: &'a Then,
}

impl Query for BoardThen<'_> {
    fn producible_types(&self, permanent: ObjectId) -> (ColorSet, bool) {
        self.q.producible_types(permanent)
    }

    fn state(&self) -> &GameState {
        self.q.state()
    }

    fn linked_card(&self, card: CardId) -> CardId {
        self.q.linked_card(card)
    }

    fn names_its_source(&self, waiting: Waiting) -> bool {
        self.q.names_its_source(waiting)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.q.text_linked(source)
    }

    fn events(&self) -> &[Record] {
        self.q.events()
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.q.card_id(def)
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.q.card(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        self.then.on_board(id).map_or_else(|| self.q.chars(id), |r| r.1)
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        self.q.token_chars(card, copy)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        self.q.lost_all_abilities(id)
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        self.q.can_target(asker, target)
    }

    fn can_attach_to(&self, attachment: ObjectId, host: ObjectId) -> bool {
        self.q.can_attach_to(attachment, host)
    }

    fn enchanting_forbidden(&self, host: ObjectId, aura: ObjectId) -> bool {
        self.q.enchanting_forbidden(host, aura)
    }

    fn can_untap(&self, permanent: ObjectId) -> bool {
        self.q.can_untap(permanent)
    }

    fn goaders(&self, creature: ObjectId) -> Vec<PlayerId> {
        self.q.goaders(creature)
    }

    fn words(&self, id: ObjectId) -> Words {
        self.q.words(id)
    }

    fn obj(&self, id: ObjectId) -> &Obj {
        self.q.obj(id)
    }

    fn chosen_targets(&self) -> &[Target] {
        self.q.chosen_targets()
    }

    fn event_subject(&self) -> Target {
        self.q.event_subject()
    }

    fn slot_opponent(&self) -> Option<PlayerId> {
        self.q.slot_opponent()
    }

    fn gift_assumed(&self) -> bool {
        self.q.gift_assumed()
    }

    fn announced_x(&self) -> Option<u32> {
        self.q.announced_x()
    }

    /// Whether `id` was on the battlefield then.
    fn is_on_battlefield(&self, id: ObjectId) -> bool {
        self.then.on_board(id).is_some()
    }

    fn controller(&self, id: ObjectId) -> PlayerId {
        self.then.on_board(id).map_or_else(|| self.q.controller(id), |r| r.2)
    }

    /// The permanents `player` controlled then, in battlefield order.
    fn permanents_of(&self, player: PlayerId) -> Vec<ObjectId> {
        self.then.battlefield.iter().copied().filter(|&id| self.then.on_board(id).is_some_and(|r| r.2 == player)).collect()
    }

    /// What it was attached to then, both being on the battlefield.
    fn attached_to(&self, id: ObjectId) -> Option<ObjectId> {
        self.then.on_board(id)?.3.filter(|&host| self.then.on_board(host).is_some())
    }
}

/// Whether anything in the game could trigger on an ability's activation (CR 603.2): a delayed ability waiting for one, a card
/// in a hand, graveyard, exile, library or on the stack that has such a triggered ability functioning in that zone (CR 113.6;
/// the ones `collect_batch` asks, `CardTable::activation_triggers_elsewhere`), or a permanent or emblem that could have one
/// however it was given (`could_have`). Where none could, the activation of a mana ability - the most frequent event - is only
/// recorded in the history, not looked at for triggers: nothing it could trigger is left out.
pub(crate) fn activation_watched(cx: &Cx) -> bool {
    let cards = cx.cards();
    let s = cx.s();
    let listens = |t: &TriggeredDef| t.listens.intersects(EventMask::ABILITY_ACTIVATED);
    let watched = s.delayed.iter().any(|d| d.when.is_event() && matches!(cards.ability(d.ability), AbilityDef::Triggered(t) if listens(t)))
        || s.seats().any(|p| {
            [Zone::hand(p), Zone::graveyard(p), Zone::exile(p), Zone::library(p)]
                .into_iter()
                .any(|z| s.zone(z).iter().any(|&id| cards.activation_triggers_elsewhere[s.obj(id).card.0 as usize]))
        })
        || s.zone(Zone::STACK).iter().any(|&id| cards.activation_triggers_elsewhere[s.obj(id).card.0 as usize])
        || {
            let could = could_have(cx, &cards.activation_triggers, listens);
            s.battlefield().iter().chain(s.zone(Zone::COMMAND)).any(|&id| could(id))
        };
    // With `CANTRIP_VERIFY`: nothing the guard left out has such an ability now - a way of giving one that the guard misses is
    // found here, not as a trigger that silently never comes. A permanent or emblem, and a card in the zones `collect_batch`
    // reads for abilities that function there (the stack's spells granted one are only for casting: not an activation).
    if !watched && layers::verifying() {
        let has = |id: ObjectId| cx.chars(id).abilities().any(|a| matches!(cards.ability(a), AbilityDef::Triggered(t) if listens(t) && t.functions.contains(s.obj(id).zone.kind())));
        let elsewhere = s.seats().flat_map(|p| [Zone::hand(p), Zone::graveyard(p), Zone::exile(p), Zone::library(p)]).chain(std::iter::once(Zone::STACK));
        for id in s.battlefield().iter().chain(s.zone(Zone::COMMAND)).chain(elsewhere.flat_map(|z| s.zone(z).iter())) {
            assert!(!has(*id), "{id:?} has an ability that triggers on an activation that nothing counted as able to give it (CR 603.2)");
        }
    }
    watched
}

/// Which permanents could have a triggered ability of a kind now (`kind`), `table` saying which cards could of their own
/// (`CardTable::look_back`, `CardTable::mana_triggers`): a superset of those that have one, which the caller then looks at
/// through the layers. Every way the layers give an object an ability counts (CR 613.1): its copiable values (layer 1: its
/// own card or the face that shows, a copy effect's card or its face-down values - `CardTable::current` - and a copy
/// effect's exceptions), a Room's unlocked doors (in its card's entry), a static ability (its own, in its card's entry;
/// another object's, wherever it functions, which may give anything), a resolved effect that gives it such an ability, and
/// the abilities the rules give for a keyword or a subtype (`intrinsic`) - the only way a keyword, granted or a counter's
/// (CR 122.1b), is an ability an object has. Layer 1 has no static abilities here: every copy effect, whatever made it, is
/// `Obj::copy`.
pub(crate) fn could_have<'a>(cx: &'a Cx, table: &'a [bool], kind: fn(&TriggeredDef) -> bool) -> impl Fn(ObjectId) -> bool + 'a {
    let cards = cx.cards();
    let s = cx.s();
    let is_kind = move |a: AbilityRef| matches!(cards.ability(a), AbilityDef::Triggered(t) if kind(t));
    let granting = table[cards.intrinsic.0 as usize] || layers::ability_granting_present(cards, s);
    let effects = s.effects.iter().any(|e| matches!(e.kind, EffectKind::AddAbility(a) if is_kind(a)));
    move |id: ObjectId| {
        let o = s.obj(id);
        granting
            || cards.texts(s, o).any(|c| table[c.0 as usize])
            || o.copy.is_some_and(|c| c.extra_instances().any(is_kind))
            || (effects && s.effects.iter().any(|e| e.affected == id && matches!(e.kind, EffectKind::AddAbility(a) if is_kind(a))))
    }
}

/// `losses`, players who lost the game at once and are about to leave it (CR 104.3, 800.4a): recorded, and the abilities
/// that trigger on them looked for (CR 603.2), looking back on `battlefield` as it was just before (CR 603.10f), as `then`
/// took it. Their own abilities that trigger go with them as they leave (CR 800.4a).
pub(crate) fn lost_the_game(cx: &mut Cx, losses: &[(PlayerId, LossReason)], battlefield: &[ObjectId], then: &Then) {
    let events: Vec<Event> = losses.iter().map(|&(player, reason)| Event::GameLost { player, reason }).collect();
    let start = cx.g.log.len();
    for &event in &events {
        cx.record(event);
    }
    collect_batch(cx, &events, Some(Before { battlefield, then, seen_by_all: &[] }), start);
}

/// How many mana that a "when that mana is spent" delayed ability rides (`DelayedWhen::ThatMana`) were spent to cast the
/// spell `event` says was cast (CR 603.7a, 106.3, 601.2h): the `ManaSpent` records of its casting. Each is a trigger (Path of
/// Ancestry's ruling: two of its mana spent on one creature spell are two). Zero for any other event.
fn that_mana_spent(cx: &Cx, rider: u32, event: &Event) -> u32 {
    let Event::SpellCast { spell, .. } = *event else { return 0 };
    cx.g.log.iter().rev()
        .take_while(|r| r.turn == cx.s().turn.number)
        .filter_map(|r| match r.event {
            Event::ManaSpent { on, cast: true, mana, rider: r, .. } if on == spell && r == rider => Some(mana.total()),
            _ => None,
        })
        .sum()
}

/// Every triggered ability this event triggers (CR 603.2).
/// Ask the event of the watched object, but ask the creating ability's conditions of its actual source.
pub(crate) fn delayed_fires(q: &dyn Query, d: &cantrip_state::Delayed, t: &TriggeredDef, asker: &Asker, event: &Event) -> Option<Fired> {
    let own = crate::resolve::Given { q, earlier: &[], subject: Target::None, slot_player: None, gift: false, x: None,
        text: d.words.map(|w| (d.watched(), w)) };
    if d.when == DelayedWhen::WatchingThisTurn {
        let source_asker = Asker { source: d.source, ..*asker };
        let source_text = crate::resolve::Given { text: d.words.map(|w| (d.source, w)), ..own };
        if t.only_if.is_some_and(|c| !c(&source_text, &source_asker))
            || t.condition.is_some_and(|c| !c(&source_text, &source_asker)) { return None; }
        if let Some(asked) = t.asked { asked(&own, asker, event) } else { (t.matches)(&own, asker.source, event) }
    } else {
        let fired = t.fires(&own, asker, event)?;
        t.condition.is_none_or(|c| c(&own, asker)).then_some(fired)
    }
}

pub fn collect_triggers(cx: &mut Cx, event: &Event, before: Option<Before>, start: usize) {
    collect_batch(cx, std::slice::from_ref(event), before, start);
}

/// Every triggered ability a batch of simultaneous events triggers (CR 603.2): leaves-the-battlefield abilities among
/// the permanents that were on the battlefield before it (`before`), the rest among those there now, and abilities
/// that function everywhere wherever they are. A "one or more" ability (CR 603.2c) triggers once for the batch, its
/// members the subjects of the events it matched; a limited one ("triggers only N times each turn", no numbered rule; CR 603.2h is "do this only once each turn") stops triggering once it has used up its
/// turn, a triggering that happens being noted in the history.
/// With `CANTRIP_VERIFY`: every permanent's stored controller is what layer 2 says (`layers::control_order`), outside an
/// instruction that holds settling - a write path that skipped a settle is found here. Checked where triggers are looked
/// for, where state-based actions are checked, and before each question put to a player.
pub(crate) fn verify_control(cx: &Cx) {
    if crate::layers::verifying() && cx.s().settle_held.get() == 0 && crate::ops::control_may_change(cx) {
        for (id, controller, _) in crate::layers::control_order(cx.cards(), cx.s(), &cx.g.log) {
            if cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
                assert_eq!(cx.s().obj(id).controller, controller, "{id:?}'s stored controller is not what layer 2 says");
            }
        }
    }
}

pub fn collect_batch(cx: &mut Cx, events: &[Event], before: Option<Before>, start: usize) {
    verify_control(cx);
    // A designation the events gave is had before anything they trigger is looked for, and the continuous effects that
    // read it apply to that look (CR 702.131d, 702.195c): Bifur, Melodic Rider entering for the third storied permanent
    // makes its own enters ability trigger an additional time.
    crate::designations::keyword_designations(cx);
    // The events that have occurred are recorded before their triggers, so continuous effects that read history see
    // them (CR 603.10, 611.3a). At CR 120.4b this includes all simultaneous dealt damage, but none of its later results.
    // Both damage and result matchers use the same `start`, excluding the entire instruction from their question about
    // the past, while layer queries see all events that have occurred by the phase being judged.
    // The look-back abilities are judged on the game just before the events (CR 603.10a), the history then included.
    let then = before.map(|b| b.then);
    // Every look-back ability is asked of the game as it was then, the battlefield included (`BoardThen`): a permanent's, an
    // emblem's, a delayed one's or a card's in another zone.
    let board = then.and_then(Then::board);
    let seen_by_all = before.map_or(&[][..], |b| b.seen_by_all);
    // The cards a discard put into a hidden zone unrevealed: their characteristics, their abilities among them, are undefined
    // (CR 701.9c, 109.3), so none of their own abilities is asked of these events.
    let undefined: Vec<ObjectId> =
        events.iter().filter_map(|e| if let Event::Discarded { card, undefined: true, .. } = e { Some(*card) } else { None }).collect();
    let before = before.map(|b| b.battlefield);
    let outer = cx.g.history_end;
    cx.g.history_end = Some(start);
    // What triggers is asked of each ability's own text, its source's now, though a resolving ability's instructions made
    // the events (`Cx::own_text`).
    let own_text = cx.g.own_text.replace(false);
    // The cut is lifted however the look ends: a host that catches a panic goes on with the game (`cantrip_host::host`).
    let looked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut found: Found = Vec::new();
        let mut statics = TriggerStatics::new(before);
        statics.before_effects = then.filter(|t| t.taken).map(|t| t.trigger_effects.as_slice());
        for event in events {
            let mask = EventMask::of(event);
            if mask.0 == 0 {
                continue;
            }
            // Whether every player could see the object of a zone change as it moved (`looks_back`).
            let seen = matches!(event, Event::ZoneChange { old, .. } if seen_by_all.contains(old));
            // The players' inherent triggered abilities (CR 725.2, 726.2, 702.179d), which no object has.
            for (index, controller, subject) in crate::designations::inherent_triggers(cx, event) {
                inherent(cx, index, controller, subject, event, seen, &mut statics, &mut found);
            }
            let now: Vec<ObjectId> = cx.s().battlefield().to_vec();
            if let Some(before) = before {
                for &id in before {
                    consider(cx, id, true, true, event, seen, mask, &mut statics, &mut found, None, then, board);
                }
                for &id in &now {
                    consider(cx, id, false, true, event, seen, mask, &mut statics, &mut found, None, None, None);
                }
            } else {
                // The same permanents for both: each one's characteristics are found once, and the abilities that don't look
                // back come after every one that does, as they would from two passes.
                let mut later: Found = Vec::new();
                for &id in &now {
                    consider(cx, id, true, true, event, seen, mask, &mut statics, &mut found, Some(&mut later), None, None);
                }
                found.append(&mut later);
            }
            // An emblem's triggered abilities function in the command zone (CR 114.4), which it never leaves, so it is there
            // both before the event and after it: its look-back abilities (a "whenever a creature dies"; CR 603.10a) and the
            // others alike are asked of it, each once, the look-back ones judged by what stopped or multiplied triggering
            // before the event (`TriggerStatics`), as any other's are.
            if !cx.s().zone(Zone::COMMAND).is_empty() {
                let mut later: Found = Vec::new();
                for id in cx.s().zone(Zone::COMMAND).to_vec() {
                    consider(cx, id, true, false, event, seen, mask, &mut statics, &mut found, Some(&mut later), None, board);
                }
                found.append(&mut later);
            }
            // Delayed abilities waiting for an event (CR 603.7): "whenever [event] this turn" (CR 603.7b), each of which
            // triggers on every match, and "when [event]", "when you next" and "until", each used up by the first.
            if cx.s().delayed.iter().any(|d| d.when.is_event()) {
                for d in cx.s().delayed.clone() {
                    // "You" in its trigger event is its own controller, the player who controlled the spell or ability that
                    // created it (CR 603.7d-e), whoever controls its source now and wherever the source has gone. So is "you"
                    // in an intervening "if" (CR 603.4). Its event is asked of the object it watches (`Delayed::watched`).
                    let asker = Asker { instance: None, source: d.watched(), controller: d.controller, by: delayed_link(cx, &d), spell: false };
                    // Its text is the one that created it (`Delayed::words`, CR 603.7, 113.7a), its source's now notwithstanding.
                    let given = crate::resolve::Given {
                        q: &*cx,
                        earlier: &[],
                        subject: Target::None,
                        slot_player: None,
                        gift: false,
                        x: None,
                        text: d.words.map(|w| (d.source, w)),
                    };
                    // One that looks back is asked of the game as it was just before the events (CR 603.10a, `BoardThen`).
                    let board_then;
                    let own: &dyn Query = match board {
                        Some(then) if cx.cards().looks_back(d.ability, event, seen) => {
                            board_then = BoardThen { q: &given, then };
                            &board_then
                        }
                        _ => &given,
                    };
                    // "When that mana is spent" triggers once for each mana of it spent to cast this spell (`DelayedWhen::ThatMana`),
                    // on nothing but a spell being cast; every other one once.
                    let times = match d.when {
                        DelayedWhen::ThatMana(rider) => that_mana_spent(cx, rider, event),
                        _ => 1,
                    };
                    if d.when.is_event()
                        && times > 0
                        && let AbilityDef::Triggered(t) = cx.cards().ability(d.ability)
                        // A delayed triggered mana ability never goes on the stack: it resolves as the mana ability that
                        // triggered it does (CR 605.1b, 605.4a), in `tapped_for_mana`.
                        && !t.mana_ability
                        && t.listens.intersects(mask)
                        && let Some(fired) = delayed_fires(own, &d, t, &asker, event)
                        // The end of an "until" exile is no triggered ability but the second one-shot effect the rules create
                        // immediately after the event (CR 610.3), so nothing that stops abilities triggering stops it, and the
                        // event that ends the wait always uses it up below. A "this turn" delayed ability is a triggered ability
                        // (CR 603.7b) and is stopped like any other. It is no ability of a permanent, and nothing makes it trigger an
                        // additional time (CR 603.2d).
                        && (d.when == DelayedWhen::UntilEvent
                            || !statics.stop(
                                cx,
                                event,
                                &Triggering {
                                    source: d.source,
                                    controller: d.controller,
                                    def: t,
                                    permanent: false,
                                    looks_back: cx.cards().looks_back(d.ability, event, seen),
                                },
                            ))
                    {
                        for _ in 1..times {
                            found.push((
                                PendingTrigger { event_amount: None, source: d.source, ability: d.ability, controller: d.controller,
                                    subject: fired.subject, data: fired.data, members: d.members, count: None, since: Some(d.since),
                                    acquired: d.acquired, words: d.words },
                                t,
                                Again::new(),
                            ));
                        }
                        found.push((
                            PendingTrigger { event_amount: matched_damage_amount(t, event),
                                source: d.source,
                                ability: d.ability,
                                controller: d.controller,
                                // "Until" and "when [event]" are about the object they were made for (CR 603.7c), and the objects it
                                // carries (`delayed_trigger_about`) go with it; so is any delayed ability made about one: a step
                                // beginning is about no object, so a "next combat" ability keeps the creature it was made for.
                                // One watching an object for the turn is about what its event is about, else that object.
                                subject: if d.when == DelayedWhen::WatchingThisTurn {
                                    if fired.subject == Target::None { Target::Object(d.watched()) } else { fired.subject }
                                } else if matches!(d.when, DelayedWhen::UntilEvent | DelayedWhen::WhenEvent) || d.subject != Target::None {
                                    d.subject
                                } else {
                                    fired.subject
                                },
                                data: fired.data,
                                members: d.members,
                                count: None,
                                // A delayed ability goes on the stack with the moment it was created (CR 701.27f).
                                since: Some(d.since),
                                acquired: d.acquired,
                                words: d.words,
                            },
                            t,
                            Again::new(),
                        ));
                        // "Until" is used up by the event that ends the wait (CR 603.7b), and "when you next" and "when
                        // [event]" by the event they trigger on (CR 603.7b).
                        if d.when.once()
                            && let Some(at) = cx.s().delayed.iter().position(|x| *x == d)
                        {
                            cx.st().delayed_remove(at);
                        }
                    }
                }
                // A spell cast has spent what it spent: a "when that mana is spent" none of whose mana is left is gone.
                if matches!(event, Event::SpellCast { .. }) {
                    cx.st().drop_spent_mana_riders();
                }
            }
            // Abilities that function in other zones (`TriggeredDef::functions`, CR 113.6): each asked only in the zones it
            // functions in, and only of cards that have one there. A spell on the stack is among them: "when you cast this
            // spell" (storm, replicate; CR 702.40a, 702.56a) functions from the stack (CR 113.6k), and so is a library: a card
            // discarded or cycled into one triggers from there (CR 113.6k, 702.29c) when it was revealed as it went (Wheel of
            // Sun and Moon). One put into a hidden zone unrevealed (Library of Leng) has its characteristics, its abilities
            // among them, undefined (CR 701.9c, 109.3), so nothing of its own triggers (`Event::Discarded::undefined`).
            // A spell may also be given such an ability by a static ability (Thrumming Stone's ripple, CR 613.1f), which only
            // a game that has had static abilities in play can have done. And the layers give suspend's two abilities, which
            // function in exile (CR 702.62a), to any object that has suspend, printed or gained (Delay, Epochrasite): they
            // trigger only from a card in exile that has a time counter (the upkeep one's "if this card is suspended", CR
            // 702.62b) or that this event took its last one from, so those are the cards in exile looked at for them, for the
            // events they listen for (a step beginning, counters).
            let cards = cx.cards();
            let granted = mask.intersects(EventMask::SPELL_CAST) && cx.s().statics_seen;
            let suspend_listens = mask.intersects(EventMask(EventMask::STEP_BEGAN.0 | EventMask::COUNTERS.0));
            let timed = |id: ObjectId, obj: &Obj| {
                suspend_listens
                    && (obj.counters(CounterKind::Time) > 0
                        || matches!(*event, Event::CountersRemoved { object, kind: CounterKind::Time, .. } if object == id))
            };
            let elsewhere: Vec<ObjectId> = cx
                .s()
                .seats()
                .flat_map(|p| [Zone::hand(p), Zone::graveyard(p), Zone::exile(p), Zone::library(p)])
                .chain(std::iter::once(Zone::STACK))
                .flat_map(|z| cx.s().zone(z).iter().copied())
                .filter(|&id| {
                    let obj = cx.s().obj(id);
                    (!undefined.contains(&id)
                        && (cards.functions_elsewhere[obj.card.0 as usize].contains(obj.zone.kind()) || (obj.zone.is(ZoneKind::Exile) && timed(id, obj))))
                        || (granted && obj.zone.is(ZoneKind::Stack))
                })
                .collect();
            for id in elsewhere {
                let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, id);
                let zone = cx.s().obj(id).zone.kind();
                for (nth, ability) in chars.abilities().enumerate() {
                    let by = || cx.link_nth(id, &chars, nth);
                    // One that looks back is asked of the game as it was just before the events (CR 603.10a, `BoardThen`).
                    let board_then;
                    let own: &dyn Query = match board {
                        Some(then) if cx.cards().looks_back(ability, event, seen) => {
                            board_then = BoardThen { q: &*cx, then };
                            &board_then
                        }
                        _ => &*cx,
                    };
                    if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
                        && t.functions.contains(zone)
                        && t.listens.intersects(mask)
                        // A look-back ability triggers only from an object that was in its zone just before the events
                        // (CR 603.10a). One these events put there is a new object (CR 400.7) that didn't exist then:
                        // Furious Forebear dying with another creature, or alone, doesn't trigger from the graveyard it
                        // goes to. An ability that isn't looked back on (a "put into a graveyard from anywhere"; CR 113.6k)
                        // still triggers from the zone it was put into.
                        && !(cx.cards().looks_back(ability, event, seen)
                            && events.iter().any(|e| matches!(e, Event::ZoneChange { new, .. } if *new == id)))
                        && let Some(fired) = if let Some(option) = t.cast_option {
                            paid_to_cast(cx, id, &chars, nth, option, event)
                        } else {
                            t.fires(own, &Asker { instance: None, source: id, controller: cx.s().obj(id).controller, by: by(), spell: false }, event)
                        }
                    {
                        // A spell is controlled by whoever cast it; a card in another zone answers to its owner.
                        let controller = if cx.s().obj(id).zone.is(ZoneKind::Stack) {
                            cx.s().obj(id).controller
                        } else {
                            cx.s().obj(id).owner
                        };
                        if let Some(condition) = t.condition
                            && !condition(own, &Asker { instance: None, source: id, controller, by: by(), spell: false })
                        {
                            continue;
                        }
                        let looks_back = cx.cards().looks_back(ability, event, seen);
                        let triggering = Triggering { source: id, controller, def: t, permanent: false, looks_back };
                        if statics.stop(cx, event, &triggering) {
                            continue;
                        }
                        let mut again = Again::new();
                        statics.additional(cx, event, &triggering, &mut again);
                        found.push((
                            PendingTrigger { event_amount: matched_damage_amount(t, event),
                                source: id,
                                ability,
                                controller,
                                subject: fired.subject,
                                data: fired.data,
                                members: Span::default(),
                                count: None,
                                since: None,
                                acquired: layers::acquired(cx.cards(), cx.s(), &cx.g.log, id, &chars, slot_of(&chars, nth)),
                                words: None,
                            },
                            t,
                            again,
                        ));
                    }
                }
            }
        }
        found
    }));
    // The triggerings are counted in the history as it now is (`Event::Triggered`).
    cx.g.history_end = outer;
    cx.g.own_text.set(own_text);
    let found = match looked {
        Ok(found) => found,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    // An event any ability listens for has had the state triggers judge the game as it is (`consider`).
    if events.iter().any(|e| EventMask::of(e).0 != 0) {
        cx.state_judged();
    }
    if found.is_empty() {
        return;
    }

    // One trigger per "one or more" ability and source for the batch, keeping the first; the rest add their subjects, and the
    // static abilities that make it trigger again for their events (CR 603.2d).
    let mut kept: Vec<(PendingTrigger, &'static TriggeredDef, Vec<ObjectId>, Again)> = Vec::with_capacity(found.len());
    for (trigger, def, again) in found {
        // Quantity was captured while matching, never reconstructed from the
        // later board or an approximate interval of event history.

        if def.once_per_batch
            && let Some(existing) =
                kept.iter_mut().find(|(k, d, _, _)| {
                    d.once_per_batch
                        && k.source == trigger.source
                        && k.ability == trigger.ability
                        && (!d.batch_by_data || k.data[1] == trigger.data[1])
                })
        {
            if let Target::Object(member) = trigger.subject
                && !existing.2.contains(&member)
            {
                existing.2.push(member);
            }
            add_again(&mut existing.3, again);
            continue;
        }
        if def.sums_per_subject
            && let Some(existing) = kept.iter_mut().find(|(k, d, _, _)| {
                d.sums_per_subject && k.source == trigger.source && k.ability == trigger.ability && k.subject == trigger.subject
            })
        {
            if def.damage_amount {
                let amount = trigger.event_amount.unwrap_or(0);
                let previous = existing.0.event_amount.unwrap_or(0);
                existing.0.event_amount = Some(previous + amount);
            } else {
                existing.0.data[0] += trigger.data[0];
            }
            add_again(&mut existing.3, again);
            continue;
        }
        let mut members = Vec::new();
        if def.once_per_batch
            && let Target::Object(member) = trigger.subject
        {
            members.push(member);
        }
        kept.push((trigger, def, members, again));
    }

    for (mut trigger, def, members, again) in kept {
        // "Deals N or more damage": judged on the batch's total, once its events are added up (CR 603.2c).
        let amount = if def.damage_amount {
            trigger.event_amount.unwrap_or(0)
        } else {
            trigger.data[0]
        };
        if amount < u32::from(def.sums_at_least) {
            continue;
        }
        // It triggers once, and once more for each static ability that says so (CR 603.2d): each a triggering of its own,
        // counted against "this ability triggers only N times each turn" (no numbered rule; CR 603.2h is "do this only once each turn").
        for n in 0..=again.len() {
            if def.triggers_each_turn > 0 {
                let limit = def.triggers_each_turn as usize;
                if crate::loops::settled_read(cx, |h| h.triggerings(trigger.source, trigger.ability) >= limit, |&reached| reached) {
                    break;
                }
                cx.record(Event::Triggered { source: trigger.source, ability: trigger.ability });
            }
            if n == 0 && !members.is_empty() {
                trigger.members = cx.st().members_push(&members);
            }
            if let Some(members_of) = def.members_of {
                let objects = {
                    let _others = cx.others_text();
                    members_of(&*cx, trigger.subject)
                };
                trigger.members = cx.st().members_push(&objects);
            }
            cx.st().trigger_push(trigger);
        }
    }
}

/// The static abilities that make a "one or more" or summed ability trigger again for one more of its batch's events, added
/// to those already found for it: each counts once for the batch (CR 603.2c, 603.2d).
fn add_again(into: &mut Again, again: Again) {
    for a in again {
        if !into.contains(&a) {
            into.push(a);
        }
    }
}

/// The turn a delayed instruction selected, found from the actual turn beginnings after it was created.
/// Extra steps/phases do not begin a turn; a skipped turn never records a beginning (CR 500.8, 614.10a).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DelayedTurn { Waiting, Current, Passed }

pub(crate) fn delayed_turn(cx: &Cx, since: u32, of: Option<PlayerId>) -> DelayedTurn {
    let mut turns = cx.g.log[since as usize..].iter().filter_map(|record| match record.event {
        Event::TurnBegan { player, .. } => Some(player),
        _ => None,
    });
    if turns.find(|&player| of.is_none_or(|of| of == player)).is_none() {
        DelayedTurn::Waiting
    } else if turns.next().is_some() {
        DelayedTurn::Passed
    } else {
        DelayedTurn::Current
    }
}

/// CR 500.4 deadlines, also run for the untap step before its turn-based actions. Separate from step triggers so
/// the untap step gains no new event or priority window.
pub(crate) fn expire_step_deadlines(cx: &mut Cx, step: Step) {
    let active = cx.s().turn.active;
    // CR 500.4: deadlines expire before beginning-of-step triggers. A grant/effect made during this step waits
    // for a later matching beginning; a skipped step never calls this procedure.
    let begins = |duration: Duration| match duration {
        Duration::UntilNextStepOf { step: s, player } | Duration::UntilEndOfNextStepOf { step: s, player } => s == step && player.is_none_or(|p| p == active),
        _ => false,
    };
    let changed = cx.s().effects.iter().any(|e| begins(e.duration));
    if changed {
        cx.st().effects_retain(|e| !matches!(e.duration, Duration::UntilNextStepOf { .. }) || !begins(e.duration));
        for at in 0..cx.s().effects.len() {
            let e = cx.s().effects[at];
            if matches!(e.duration, Duration::UntilEndOfNextStepOf { .. }) && begins(e.duration) {
                cx.st().effect_set(at, ContinuousEffect { duration: Duration::EndOfStep, ..e });
            }
        }
    }
    let expires = |p: &cantrip_state::PlayPermission| p.player == active && match p.grant.expiry {
        Expiry::UntilYourNextEndStep => step == Step::End,
        Expiry::UntilYourNextStep(s) => s == step,
        _ => false,
    };
    if cx.s().permissions.iter().any(expires) { cx.st().permissions_retain(|p| !expires(p)); }
    if changed { ops::settle_lasting_and_control(cx); }
}

/// A step has begun: its "at the beginning of" abilities trigger, and so do the delayed abilities waiting for it,
/// which are used up (CR 603.7c) unless they wait for each such step for the rest of the game.
pub fn step_began(cx: &mut Cx, step: Step) {
    let active = cx.s().turn.active;
    expire_step_deadlines(cx, step);
    let mut at = 0;
    while at < cx.s().delayed.len() {
        let d = cx.s().delayed[at];
        let (due, used_up) = match d.when {
            DelayedWhen::Step { step: s, on_turn_of } => (s == step && on_turn_of.is_none_or(|p| p == active), true),
            DelayedWhen::StepOfNextTurn { step: s, of } => match delayed_turn(cx, d.since, of) {
                DelayedTurn::Waiting => (false, true),
                DelayedTurn::Current => (s == step, true),
                DelayedTurn::Passed => { cx.st().delayed_remove(at); continue; }
            },
            DelayedWhen::EachStep { step: s, on_turn_of } => (s == step && on_turn_of.is_none_or(|p| p == active), false),
            DelayedWhen::ExtraTurnStep { step: s, turn } => (s == step && cx.s().turn.extra == turn, true),
            DelayedWhen::PhaseStep { step: s, phase } => (s == step && cx.s().turn.phase_id == phase, true),
            DelayedWhen::NextTurnStep { step: s, player, after } => (s == step && player == active && cx.s().turn.number > after, true),
            DelayedWhen::ThisTurn
            | DelayedWhen::WatchingThisTurn
            | DelayedWhen::UntilEvent
            | DelayedWhen::UntilNextTurnOf(_)
            | DelayedWhen::NextThisTurn
            | DelayedWhen::WhenEvent
            | DelayedWhen::ThatMana(_) => {
                (false, false)
            }
        };
        if due {
            if used_up {
                cx.st().delayed_remove(at);
            } else {
                at += 1;
            }
            cx.st().trigger_push(PendingTrigger { event_amount: None, source: d.source, ability: d.ability, controller: d.controller, subject: d.subject, data: [0; 2], members: d.members, count: None, since: Some(d.since), acquired: d.acquired, words: d.words });
        } else {
            at += 1;
        }
    }
    // The monarch's and the initiative's step triggers are found from the event (`designations::inherent_triggers`).
    // Suspend's upkeep ability (CR 702.62a) is an ability of each card that has suspend, found from the event in exile.
    cx.emit(Event::StepBegan { step });
}

/// Whether a triggered ability's intervening "if" still holds as it resolves (CR 603.4).
pub fn still_holds(cx: &Cx, item: &StackItem) -> bool {
    match cx.cards().ability(item.ability) {
        AbilityDef::Triggered(TriggeredDef { condition: Some(condition), .. }) => {
            condition(cx, &Asker { instance: None, source: item.object, controller: item.controller, by: cx.link_of(item), spell: false })
        }
        _ => true,
    }
}

/// Called only after the ability's matcher accepted this event.
fn matched_damage_amount(def: &TriggeredDef, event: &Event) -> Option<u32> {
    // What a "that much damage" trigger captures is the damage of the event it matched; an event that is no damage dealt
    // has none to capture.
    match *event {
        Event::Damage { amount, .. } if def.damage_amount => Some(amount),
        _ => None,
    }
}
