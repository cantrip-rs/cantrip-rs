//! What happens each time a player would receive priority (CR 117.5): state-based actions until none apply
//! (CR 704), then triggered abilities onto the stack (CR 603.3), and again until neither does anything.

use cantrip_api::*;
use cantrip_state::{PendingTrigger, Span};

use crate::actions::target_candidates_given;
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
    for p in cx.s().seats().collect::<Vec<_>>() {
        let player = *cx.s().player(p);
        if player.lost {
            continue;
        }
        if let Some(reason) = ops::loses_to_state_based_actions(cx, p) {
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
    let mut to_graveyard = Vec::new();
    let mut destroyed = Vec::new();
    let mut unattach = Vec::new();
    let mut unbestow = Vec::new();
    let mut sagas = Vec::new();
    let mut legends: Vec<(ObjectId, &'static str)> = Vec::new();
    let mut bound: Vec<crate::day_night::Bound> = Vec::new();
    let mut battles: Vec<(ObjectId, bool)> = Vec::new();
    let mut annihilate: Vec<(ObjectId, i16)> = Vec::new();
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
        // an Equipment attached to an illegal permanent becomes unattached; so does anything else attached to
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
                    to_graveyard.push(id);
                }
                continue;
            }
        } else if let Some(host) = obj.attached_to
            && !(chars.has_subtype(Subtype::Equipment) && crate::attach::host_allows(&*cx, id, host, false))
        {
            unattach.push(id);
        }
        if chars.is(CardType::Creature) {
            let toughness = chars.toughness.unwrap_or(0);
            if toughness <= 0 {
                to_graveyard.push(id);
                continue;
            }
            if i32::from(obj.damage) >= i32::from(toughness) || obj.deathtouched {
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
        let last = chars
            .abilities()
            .filter_map(|a| match cx.cards().ability(a) {
                AbilityDef::Triggered(t) if t.chapter > 0 => Some((a, t.chapter)),
                _ => None,
            })
            .fold(0, |most, (_, chapter)| most.max(chapter));
        if last == 0 || cx.s().obj(id).counters(CounterKind::Lore) < last as i16 {
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
    if !to_graveyard.is_empty() || !destroyed.is_empty() {
        // All at once (CR 704.3): a creature that dies to its toughness and one destroyed by damage leave together.
        let mut moves: Vec<ops::Moving> = Vec::new();
        for id in to_graveyard.into_iter().chain(destroyed.iter().copied()) {
            // Lethal damage destroys (CR 704.5g-h): a regeneration shield may replace that (CR 701.19a, 614.8), ordered with the
            // other replacement effects (CR 616.1).
            let mut moving = ops::Moving { destroy: destroyed.contains(&id), ..ops::Moving::to(id, Zone::graveyard(cx.s().obj(id).owner)) };
            if sagas_sacrificed.contains(&id) {
                moving.reason = Some(ZoneReason::Sacrifice);
            }
            moves.push(moving);
        }
        ops::move_together(cx, &moves, ZoneReason::StateBased);
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
    for (p, reason) in leavers {
        cx.record(Event::GameLost { player: p, reason });
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
        AbilityDef::Activated(_) | AbilityDef::Static(_) => unreachable!("a triggered ability"),
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
    // Asked as the ability it is (CR 607.5): one an effect gave its source reads only its own links.
    let asker = Asker { source: trigger.source, controller: trigger.controller, by: layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired), spell: false };
    let start = targets.len();
    let slots = crate::resolve::expand_specs(cx.s(), trigger.controller, specs);
    // "If [it] was kicked, instead choose ..." (CR 702.33a, 601.2c), of the source.
    let kicked = cx.s().obj(trigger.source).kicked;
    for (spec, slot_player, at) in &slots {
        let spec = &spec.when_kicked(kicked);
        let before = targets.len();
        'slot: {
            let candidates = target_candidates_given(
                cx,
                &asker,
                spec,
                if slots.len() > 1 { &targets[start..] } else { &[] },
                trigger.subject,
                *slot_player,
                None,
            );
            let counted = u32::from(trigger_counted(cx, trigger));
            let (candidates, together) = crate::actions::within_together(cx, &asker, spec, counted, candidates);
            let (least, most) = spec.bounds(counted);
            if !crate::actions::enough_together(cx, &asker, &together, &candidates, least) {
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
            let ask = RequestKind::ChooseTarget { label: spec.label, slot_player: *slot_player,
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
fn trigger_counted(cx: &Cx, trigger: &PendingTrigger) -> u16 {
    // "Up to that many target ...", that many being the event's amount (the life gained): no creature count comes near a u16.
    if triggered(cx, trigger.ability).counted_by_amount {
        return u16::try_from(trigger.data[0]).unwrap_or(u16::MAX);
    }
    // "Up to X target ...", X read from the game now, as the ability goes on the stack (CR 603.3d, 601.2c).
    if let Some(count) = triggered(cx, trigger.ability).counted_by {
        let by = layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired);
        return count(cx, &Asker { source: trigger.source, controller: trigger.controller, by, spell: false });
    }
    let source = cx.s().obj(trigger.source);
    trigger.count.unwrap_or(if source.x > 0 { source.x } else { u16::from(source.kicked) })
}

fn put_trigger(cx: &mut Cx, trigger: PendingTrigger) -> R<()> {
    // Asked as the ability it is (CR 607.5) as its modes and targets are chosen: one an effect gave its source reads only
    // its own links.
    let by = layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired);
    put_trigger_as(cx, trigger, by)
}

fn put_trigger_as(cx: &mut Cx, trigger: PendingTrigger, by: Option<Linked>) -> R<()> {
    if left_the_game(cx, trigger.controller) {
        return Ok(());
    }
    let def = triggered(cx, trigger.ability);
    // Asked as the ability it is (CR 607.5): one an effect gave its source reads only its own links.
    let asker = Asker { source: trigger.source, controller: trigger.controller, by: layers::given_link(cx.cards(), cx.s(), trigger.source, trigger.ability, trigger.acquired), spell: false };
    let mut targets = Vec::new();
    let mut of: Vec<u8> = Vec::new();
    let mut shares: Vec<u32> = Vec::new();
    let mut mask = 0u32;
    if let Some(modal) = def.modal {
        // A modal ability's modes are chosen as it is put on the stack (CR 603.3c, 700.2), then each chosen mode's
        // target, in printed order. A mode whose required target has no candidate cannot be chosen (CR 700.2a), and with
        // too few left the ability is removed. As many eligible modes as it chooses is no choice, and is not asked.
        // "Choose up to that many" (`ModalDef::COUNTED`, CR 603.12a): as many as the ability's count, read now.
        let most = modal.most(trigger_counted(cx, &trigger));
        let eligible = crate::actions::eligible_modes(cx, &asker, modal);
        if eligible.len() < modal.min.max(1) as usize || most == 0 {
            return Ok(());
        }
        let chosen = if most == modal.min && eligible.len() == modal.min as usize {
            eligible
        } else {
            if left_the_game(cx, trigger.controller) {
                return Ok(());
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
            return Ok(());
        }
        // Chosen as it goes on the stack: chosen even if the ability is later countered (CR 603.3c, 700.2).
        cx.emit(Event::ModesChosen { player: trigger.controller, source: trigger.source, ability: trigger.ability, modes: mask });
        let mut first_spec = 0u8;
        for (n, mode) in modal.modes.iter().enumerate() {
            for _ in 0..modal.times(mask, n) {
                if !choose_trigger_targets(cx, &trigger, mode.targets, true, &mut targets, &mut of, first_spec)? {
                    return Ok(());
                }
                first_spec += mode.targets.len() as u8;
            }
        }
    } else if let Some(divide) = def.divide
        && let Some(spec) = def.targets.first()
    {
        // CR 601.2d, 603.3d: a divided slot's targets and shares are chosen as the ability goes on the stack.
        let x = trigger.count.unwrap_or(cx.s().obj(trigger.source).x);
        let total = match divide.of_x {
            Some(of_x) => u32::from(of_x(x)),
            None if divide.total == TargetSpec::COUNTED => u32::from(x),
            None => divide.total,
        };
        if left_the_game(cx, trigger.controller) {
            return Ok(());
        }
        match crate::actions::ask_divided(cx, trigger.controller, trigger.source, false, spec, total, None, false, by)? {
            crate::actions::Divided::Chosen(chosen, given) => {
                targets = chosen;
                shares = given;
            }
            crate::actions::Divided::Skipped => {}
            crate::actions::Divided::Impossible => return Ok(()),
        }
    } else if !choose_trigger_targets(cx, &trigger, def.targets, false, &mut targets, &mut of, 0)? {
        return Ok(());
    }
    let mut item = StackItem {
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
        copied: false,
        cast_known: u32::MAX,
        acquired: trigger.acquired,
    };
    // Its controller left as one of its choices was asked (CR 800.4d).
    if cx.s().player(trigger.controller).lost {
        return Ok(());
    }
    cx.st().set_item_targets(&mut item, &targets, &of, &shares);
    cx.st().stack_push(item);
    ops::became_targets(cx, targets.iter().copied(), trigger.source, trigger.controller, true);
    // The controller of a triggered ability puts it on the stack (CR 603.3a) and is who commits its crime (CR 700.13).
    ops::commit_crime(cx, trigger.controller, trigger.source, &targets);
    Ok(())
}

/// The static abilities that made one triggering trigger additional times (CR 603.2d): each the permanent and the index of its
/// `CardDef::additional_triggers` entry, so the same one met again among a batch's events adds nothing more.
pub(crate) type Again = Vec<(ObjectId, u8)>;

type Found = Vec<(PendingTrigger, &'static TriggeredDef, Again)>;

/// The triggered abilities of `source` that `event` triggers, onto `found`: those that look back on it (CR 603.10,
/// `CardTable::looks_back`) or those that don't, as `look_back` says. With `later`, both, the object's characteristics
/// found once: the look-back ones onto `found`, the others onto `later`.
///
/// `permanent`: the source is on the battlefield (or was, for a look-back ability); false for an emblem in the command zone,
/// which is no permanent (CR 114.4), so "a triggered ability of a permanent" does not mean its abilities.
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
    for (nth, ability) in chars.abilities().enumerate() {
        let by = || {
            let given = *given.get_or_init(|| taken.is_some() || layers::acquisition_possible(cx.cards(), cx.s()));
            if given { layers::given_link(cx.cards(), cx.s(), source, ability, acquired_at(nth)) } else { None }
        };
        // A state trigger (CR 603.8) triggers as soon as its state is true, so it is asked after every event, of the board as
        // it is now (never of a look-back pass), as well as each time a player would receive priority (`state_triggers`).
        if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
            && let Some(state) = t.state
        {
            if (later.is_some() || !look_back)
                && state(cx, &Asker { source, controller, by: by(), spell: false })
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
                chapter_reached(source, t.chapter, event)
            } else if t.attack_option.is_some() {
                paid_to_attack(source, ability, chars.instance(nth), event)
            } else if t.listens.intersects(mask) {
                t.fires(cx, &Asker { source, controller, by: by(), spell: false }, event)
            } else {
                None
            }
        {
            if let Some(condition) = t.condition
                && !condition(cx, &Asker { source, controller, by: by(), spell: false })
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
                PendingTrigger {
                    source,
                    ability,
                    controller,
                    subject: fired.subject,
                    data: fired.data,
                    members: Span::default(),
                    count: None,
                    since: None,
                    acquired: acquired_at(nth),
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
    found.push((PendingTrigger { source, ability, controller, subject, data: [0; 2], members: Span::default(), count: None, since: None, acquired: Acquired::OWN }, t, again));
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
    PendingTrigger { source, ability, controller, subject: Target::None, data: [0; 2], members: Span::default(), count: None, since: None, acquired }
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
                && state(cx, &Asker { source, controller, by: cx.link_nth(source, &chars, nth), spell: false })
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
        all || cx.cards().state_triggers[cx.cards().current(obj).0 as usize]
            || obj.copy.is_some_and(|c| c.extra_abilities.iter().any(|e| e.instances > 0 && cx.cards().state_triggers[e.card.0 as usize]))
            || cx.s().effects.iter().any(|e| e.affected == id && matches!(e.kind, EffectKind::AddAbility(a) if is_state(cx, a)))
    };
    let sources: Vec<ObjectId> =
        cx.s().battlefield().iter().chain(cx.s().zone(Zone::COMMAND)).copied().filter(|&id| may_have(cx, id)).collect();
    let mut found: Found = Vec::new();
    for source in sources {
        let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, source);
        let controller = cx.s().obj(source).controller;
        let acquired_at = |nth: usize| layers::acquired(cx.cards(), cx.s(), &cx.g.log, source, &chars, slot_of(&chars, nth));
        for (nth, ability) in chars.abilities().enumerate() {
            if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
                && let Some(state) = t.state
                && state(cx, &Asker { source, controller, by: cx.link_nth(source, &chars, nth), spell: false })
                && !state_trigger_waiting(cx, source, ability, &found, None)
            {
                found.push((state_trigger(source, ability, controller, acquired_at(nth)), t, Again::new()));
            }
        }
    }
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

/// A Saga's chapter ability (CR 714.2b) triggers when lore counters are put on it and their number becomes at least
/// its chapter number, having been less than it.
fn chapter_reached(source: ObjectId, chapter: u8, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, kind: CounterKind::Lore, amount, total, .. } if *object == source => {
            if total - amount < chapter as i16 && *total >= chapter as i16 { Fired::PLAIN } else { None }
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
}

impl<'a> TriggerStatics<'a> {
    pub(crate) fn new(before: Option<&'a [ObjectId]>) -> Self {
        TriggerStatics { before, now: None, then: None }
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
            let asker = Asker { source: id, controller: obj.controller, by: None, spell: false };
            cx.card(cx.cards().current(obj)).trigger_rules.iter().any(|rule| rule(cx, &asker, event, triggering))
        })
    }

    /// The static abilities that make this ability, which this event triggered, trigger an additional time (CR 603.2d): one
    /// more for each, added to `again` unless already there.
    pub(crate) fn additional(&mut self, cx: &Cx, event: &Event, triggering: &Triggering, again: &mut Again) {
        for &id in self.sources(cx, triggering) {
            let obj = cx.s().obj(id);
            let asker = Asker { source: id, controller: obj.controller, by: None, spell: false };
            for (n, rule) in cx.card(cx.cards().current(obj)).additional_triggers.iter().enumerate() {
                if rule(cx, &asker, event, triggering) && !again.contains(&(id, n as u8)) {
                    again.push((id, n as u8));
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
        .filter(|&id| cx.cards().trigger_rules[cx.cards().current(cx.s().obj(id)).0 as usize] && !cx.lost_all_abilities(id))
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
pub struct Then {
    taken: bool,
    /// By object, for `of`.
    objects: Vec<(ObjectId, Chars, PlayerId)>,
    /// By object, parallel to `objects`: how it had acquired its abilities then (CR 607.5; `layers::acquisitions`), the text
    /// it had and each slot's where an effect could have given one. Empty where nothing could have (`acquisition_possible`).
    acquired: Vec<Taken>,
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

    /// Nothing taken: the look-back pass judges the permanents as they are now.
    pub(crate) fn none() -> Then {
        Then { taken: false, objects: Vec::new(), acquired: Vec::new() }
    }

    /// The permanents on the battlefield now that could look back, as they are now: called just before the events.
    pub(crate) fn now(cx: &Cx) -> Then {
        let cards = cx.cards();
        let s = cx.s();
        let could = could_have(cx, &cards.look_back, TriggeredDef::may_look_back);
        let mut objects: Vec<(ObjectId, Chars, PlayerId)> = s
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| could(id))
            .map(|id| (id, layers::chars(cards, s, &cx.g.log, id), s.obj(id).controller))
            .collect();
        objects.sort_unstable_by_key(|&(id, _, _)| id);
        // With `CANTRIP_VERIFY`: a permanent left out has no look-back ability now - a way of giving one that the set above
        // misses is found here, not as a trigger that silently never happens.
        if layers::verifying() {
            for &id in s.battlefield().iter().filter(|&&id| !could(id)) {
                let chars = layers::chars(cards, s, &cx.g.log, id);
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
        Then { taken: true, objects, acquired }
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
            || table[cards.current(o).0 as usize]
            || o.copy.is_some_and(|c| c.extra_instances().any(is_kind))
            || (effects && s.effects.iter().any(|e| e.affected == id && matches!(e.kind, EffectKind::AddAbility(a) if is_kind(a))))
    }
}

/// Every triggered ability this event triggers (CR 603.2).
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
    // The events are recorded already (from `start`), so the continuous effects that read the history see them as the
    // triggers are looked for (CR 603.10: "continuous effects that exist at that time are used to determine what the trigger
    // conditions are"); each trigger's own question about the past is of the history as it was before them, as it always
    // has been: "the first time each turn" is not this event.
    // The look-back abilities are judged on the game just before the events (CR 603.10a), the history then included.
    let then = before.map(|b| b.then);
    let seen_by_all = before.map_or(&[][..], |b| b.seen_by_all);
    // The cards a discard put into a hidden zone unrevealed: their characteristics, their abilities among them, are undefined
    // (CR 701.9c, 109.3), so none of their own abilities is asked of these events.
    let undefined: Vec<ObjectId> =
        events.iter().filter_map(|e| if let Event::Discarded { card, undefined: true, .. } = e { Some(*card) } else { None }).collect();
    let before = before.map(|b| b.battlefield);
    let outer = cx.g.history_end;
    cx.g.history_end = Some(start);
    // The cut is lifted however the look ends: a host that catches a panic goes on with the game (`cantrip_host::host`).
    let looked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut found: Found = Vec::new();
        let mut statics = TriggerStatics::new(before);
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
                    consider(cx, id, true, true, event, seen, mask, &mut statics, &mut found, None, then);
                }
                for &id in &now {
                    consider(cx, id, false, true, event, seen, mask, &mut statics, &mut found, None, None);
                }
            } else {
                // The same permanents for both: each one's characteristics are found once, and the abilities that don't look
                // back come after every one that does, as they would from two passes.
                let mut later: Found = Vec::new();
                for &id in &now {
                    consider(cx, id, true, true, event, seen, mask, &mut statics, &mut found, Some(&mut later), None);
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
                    consider(cx, id, true, false, event, seen, mask, &mut statics, &mut found, Some(&mut later), None);
                }
                found.append(&mut later);
            }
            // Delayed abilities of "whenever [event] this turn" (CR 603.7b), each of which triggers on every match.
            if cx.s().delayed.iter().any(|d| d.when.is_event()) {
                for d in cx.s().delayed.clone() {
                    // "You" in its trigger event is its own controller, the player who controlled the spell or ability that
                    // created it (CR 603.7d-e), whoever controls its source now and wherever the source has gone. So is "you"
                    // in an intervening "if" (CR 603.4).
                    let asker = Asker { source: d.source, controller: d.controller, by: delayed_link(cx, &d), spell: false };
                    if d.when.is_event()
                        && let AbilityDef::Triggered(t) = cx.cards().ability(d.ability)
                        // A delayed triggered mana ability never goes on the stack: it resolves as the mana ability that
                        // triggered it does (CR 605.1b, 605.4a), in `tapped_for_mana`.
                        && !t.mana_ability
                        && t.listens.intersects(mask)
                        && let Some(fired) = t.fires(cx, &asker, event)
                        && t.condition.is_none_or(|condition| condition(cx, &asker))
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
                        found.push((
                            PendingTrigger {
                                source: d.source,
                                ability: d.ability,
                                controller: d.controller,
                                subject: if d.when == DelayedWhen::UntilEvent { d.subject } else { fired.subject },
                                data: fired.data,
                                members: Span::default(),
                                count: None,
                                // A delayed ability goes on the stack with the moment it was created (CR 701.27f).
                                since: Some(d.since),
                                acquired: d.acquired,
                            },
                            t,
                            Again::new(),
                        ));
                        // "Until" is used up by the event that ends the wait (CR 603.7b), and "when you next" by the event it
                        // triggers on (CR 603.7b).
                        if matches!(d.when, DelayedWhen::UntilEvent | DelayedWhen::NextThisTurn)
                            && let Some(at) = cx.s().delayed.iter().position(|x| *x == d)
                        {
                            cx.st().delayed_remove(at);
                        }
                    }
                }
            }
            // Abilities that function in other zones (`TriggeredDef::functions`, CR 113.6): each asked only in the zones it
            // functions in, and only of cards that have one there. A spell on the stack is among them: "when you cast this
            // spell" (storm, replicate; CR 702.40a, 702.56a) functions from the stack (CR 113.6k), and so is a library: a card
            // discarded or cycled into one triggers from there (CR 113.6k, 702.29c) when it was revealed as it went (Wheel of
            // Sun and Moon). One put into a hidden zone unrevealed (Library of Leng) has its characteristics, its abilities
            // among them, undefined (CR 701.9c, 109.3), so nothing of its own triggers (`Event::Discarded::undefined`).
            // A spell may also be given such an ability by a static ability (Thrumming Stone's ripple, CR 613.1f), which only
            // a game that has had static abilities in play can have done.
            let cards = cx.cards();
            let granted = mask.intersects(EventMask::SPELL_CAST) && cx.s().statics_seen;
            let elsewhere: Vec<ObjectId> = cx
                .s()
                .seats()
                .flat_map(|p| [Zone::hand(p), Zone::graveyard(p), Zone::exile(p), Zone::library(p)])
                .chain(std::iter::once(Zone::STACK))
                .flat_map(|z| cx.s().zone(z).iter().copied())
                .filter(|&id| {
                    let obj = cx.s().obj(id);
                    (!undefined.contains(&id) && cards.functions_elsewhere[obj.card.0 as usize].contains(obj.zone.kind()))
                        || (granted && obj.zone.is(ZoneKind::Stack))
                })
                .collect();
            for id in elsewhere {
                let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, id);
                let zone = cx.s().obj(id).zone.kind();
                for (nth, ability) in chars.abilities().enumerate() {
                    let by = || cx.link_nth(id, &chars, nth);
                    if let AbilityDef::Triggered(t) = cx.cards().ability(ability)
                        && t.functions.contains(zone)
                        && t.listens.intersects(mask)
                        && let Some(fired) = t.fires(cx, &Asker { source: id, controller: cx.s().obj(id).controller, by: by(), spell: false }, event)
                    {
                        // A spell is controlled by whoever cast it; a card in another zone answers to its owner.
                        let controller = if cx.s().obj(id).zone.is(ZoneKind::Stack) {
                            cx.s().obj(id).controller
                        } else {
                            cx.s().obj(id).owner
                        };
                        if let Some(condition) = t.condition
                            && !condition(cx, &Asker { source: id, controller, by: by(), spell: false })
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
                            PendingTrigger {
                                source: id,
                                ability,
                                controller,
                                subject: fired.subject,
                                data: fired.data,
                                members: Span::default(),
                                count: None,
                                since: None,
                                acquired: layers::acquired(cx.cards(), cx.s(), &cx.g.log, id, &chars, slot_of(&chars, nth)),
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
            existing.0.data[0] += trigger.data[0];
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
                let objects = members_of(&*cx, trigger.subject);
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

/// A step has begun: its "at the beginning of" abilities trigger, and so do the delayed abilities waiting for it,
/// which are used up (CR 603.7c) unless they wait for each such step for the rest of the game.
pub fn step_began(cx: &mut Cx, step: Step) {
    let active = cx.s().turn.active;
    // CR 500.4: a duration of "until your next end step" ends as the step begins, before its abilities trigger; a
    // permission granted during that step is for the next one.
    if step == Step::End && cx.s().permissions.iter().any(|p| p.grant.expiry == Expiry::UntilYourNextEndStep) {
        cx.st().permissions_retain(|p| !(p.grant.expiry == Expiry::UntilYourNextEndStep && p.player == active));
    }
    let mut at = 0;
    while at < cx.s().delayed.len() {
        let d = cx.s().delayed[at];
        let (due, used_up) = match d.when {
            DelayedWhen::Step { step: s, on_turn_of } => (s == step && on_turn_of.is_none_or(|p| p == active), true),
            DelayedWhen::EachStep { step: s, on_turn_of } => (s == step && on_turn_of.is_none_or(|p| p == active), false),
            DelayedWhen::ExtraTurnStep { step: s, turn } => (s == step && cx.s().turn.extra == turn, true),
            DelayedWhen::ThisTurn | DelayedWhen::UntilEvent | DelayedWhen::UntilNextTurnOf(_) | DelayedWhen::NextThisTurn => (false, false),
        };
        if due {
            if used_up {
                cx.st().delayed_remove(at);
            } else {
                at += 1;
            }
            cx.st().trigger_push(PendingTrigger { source: d.source, ability: d.ability, controller: d.controller, subject: d.subject, data: [0; 2], members: d.members, count: None, since: Some(d.since), acquired: d.acquired });
        } else {
            at += 1;
        }
    }
    // The monarch's and the initiative's step triggers are found from the event (`designations::inherent_triggers`).
    cx.emit(Event::StepBegan { step });
    if step == Step::Upkeep {
        suspended_cards(cx, active);
    }
}

/// "At the beginning of your upkeep, if this card is suspended, remove a time counter from it" (CR 702.62a): the rules'
/// own ability for each suspended card its owner has in exile. A card is looked at only if it has time counters.
fn suspended_cards(cx: &mut Cx, active: PlayerId) {
    let intrinsic = cx.cards().intrinsic;
    let suspended: Vec<ObjectId> = cx
        .s()
        .zone(Zone::exile(active))
        .iter()
        .copied()
        .filter(|&id| cx.s().obj(id).counters(CounterKind::Time) > 0 && cx.card(cx.s().obj(id).card).suspend.is_some())
        .collect();
    for source in suspended {
        cx.st().trigger_push(PendingTrigger {
            source,
            ability: AbilityRef { card: intrinsic, index: cantrip_api::intrinsic::SUSPEND_UPKEEP },
            controller: active,
            subject: Target::None,
            data: [0; 2],
            members: Span::default(),
            count: None,
            since: None,
            acquired: Acquired::OWN,
        });
    }
}

/// Whether a triggered ability's intervening "if" still holds as it resolves (CR 603.4).
pub fn still_holds(cx: &Cx, item: &StackItem) -> bool {
    match cx.cards().ability(item.ability) {
        AbilityDef::Triggered(TriggeredDef { condition: Some(condition), .. }) => {
            condition(cx, &Asker { source: item.object, controller: item.controller, by: cx.link_of(item), spell: false })
        }
        _ => true,
    }
}
