//! The turn (CR 500-514): steps and their turn-based actions, combat, cleanup, and the priority round of each step.

use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

use crate::layers::{self, live, summoning_sick};
use crate::{Cx, Flow, Game, R, Stop, actions, ops};

/// What a card that may begin the game on the battlefield asks (CR 103.6a): the Leylines, Gemstone Caverns.
const BEGIN_ON_BATTLEFIELD: Question = Question::new(
    "BeginOnBattlefield",
    "Begin the game with {source} on the battlefield?",
    "If you don't, it stays in your opening hand",
    "Put it onto the battlefield",
    "Keep it in hand",
);

/// Shuffles a zone with the state's own generator (Fisher-Yates).
pub fn shuffle(state: &mut GameState, zone: Zone) {
    let mut cards: Vec<ObjectId> = state.zone(zone).to_vec();
    permute(state, &mut cards);
    for &id in &cards {
        state.zone_remove(zone, id);
    }
    for id in cards {
        state.zone_push(zone, id);
    }
}

/// Shuffles a player's library and records it (`Event::Shuffled`): whatever anyone knew of its order is gone.
pub fn shuffle_library(cx: &mut Cx, player: PlayerId) {
    crate::library_visibility::conceal(cx, player);
    shuffle_in(cx, Zone::library(player));
    cx.record(Event::Shuffled { player });
    crate::library_visibility::sync(cx);
}

/// A resolving instruction shuffles through its effect context. CR 106.7 advice tries all orders;
/// actual play uses the unchanged generator procedure.
pub(crate) fn shuffle_in(cx: &mut Cx, zone: Zone) {
    if cx.g.hypothetical_chance.is_none() {
        shuffle(cx.st(), zone);
        return;
    }
    let mut cards = cx.s().zone(zone).to_vec();
    for i in (1..cards.len()).rev() {
        let j = cx.random_below(i as u32 + 1) as usize;
        cards.swap(i, j);
    }
    for &id in &cards { cx.st().zone_remove(zone, id); }
    for id in cards { cx.st().zone_push(zone, id); }
}

/// Puts a list in random order with the state's own generator (Fisher-Yates), drawing as `shuffle` does.
pub fn permute<T>(state: &mut GameState, items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let j = state.random(i as u32 + 1) as usize;
        items.swap(i, j);
    }
}

/// Moves the game on to its next step, or its first turn, and puts that step's work on the agenda.
pub fn next_step(game: &mut Game) {
    let turn = game.state.turn;
    let ending = turn.step();
    // The combat phase ends with its end of combat step, or with whatever step of it an effect ended the turn or the
    // combat phase in (CR 724.1d, 724.2d), or with the only step of a scheduled combat phase.
    let combat_ends = ending == Step::EndOfCombat
        || ((turn.ending || turn.combat_ending || turn.phase_only_step.is_some())
            && matches!(ending, Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers | Step::CombatDamage));

    // Mana empties as each step ends (CR 500.5), except the mana that lasts until the end of combat, which is lost as
    // the combat phase ends, and the mana that lasts until end of turn, lost as the cleanup step does (CR 106.4, 500.5a).
    if turn.number > 0 {
        let ends = if ending == Step::Cleanup {
            ManaLasts::Turn
        } else if combat_ends {
            ManaLasts::Combat
        } else {
            ManaLasts::Step
        };
        // Except what a static ability keeps (CR 500.5: Electro, Assaulting Battery), asked as the step ends.
        let kept = {
            let cx = Cx { g: &mut *game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
            crate::static_rules::mana_kept(&cx)
        };
        let s = &mut game.state;
        for p in s.seats().collect::<Vec<_>>() {
            let keep = kept.iter().find(|&&(q, _)| q == p).map_or([false; 6], |&(_, k)| k);
            s.empty_mana_keeping(p, ends, keep);
        }
        // "When that mana is spent" waits only while some of that mana is left (`DelayedWhen::ThatMana`).
        s.drop_spent_mana_riders();
    }
    let main_seen = turn.scheduled && game.log.iter().rev().take_while(|r| r.turn == turn.number).any(|r| matches!(r.event, Event::StepBegan { step } if step.is_main()));
    // A duration through this step ends at its boundary, even when the step is ended early. Settle control before
    // the new step's beginning triggers inspect its permanents (CR 500.4, 611.2a).
    if game.state.effects.iter().any(|e| e.duration == Duration::EndOfStep) {
        game.state.effects_retain(|e| e.duration != Duration::EndOfStep);
        let mut cx = Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        ops::settle_lasting_and_control(&mut cx);
    }
    let s = &mut game.state;
    // CR 724.1d, 724.2d: a turn or combat phase ended during combat removes every creature and planeswalker from combat as
    // the step ends, as the end of the combat phase does, and "until end of combat" effects expire.
    if turn.number > 0 && combat_ends {
        // "This combat" effects end with the combat phase (CR 511.3), before an additional one begins.
        if s.effects.iter().any(|e| e.duration == Duration::EndOfCombat) {
            s.effects_retain(|e| e.duration != Duration::EndOfCombat);
        }
        s.set_attackers(Vec::new());
        s.set_blocks(Vec::new());
        s.set_blockers(Vec::new());
        s.update_turn(|t| {
            t.attacked = false;
            t.damage_round = 0;
        });
    }

    // CR 724.2d-e: an effect ended the combat phase, so the game skips straight to the phase after it, as though its end of
    // combat step had just ended: the end of combat step itself is skipped, and nothing triggers "at end of combat". The
    // next phase is the postcombat main phase, or an additional combat phase that follows this one (CR 500.8).
    let ending = if turn.combat_ending && !turn.ending { Step::EndOfCombat } else { ending };
    let (mut number, mut active, mut step) = if turn.number == 0 {
        (1, turn.starting, Step::Untap)
    } else if turn.ending || (ending == Step::Cleanup && turn.cleanup_again) {
        // CR 724.1d: the turn ended, so the game skips straight to the cleanup step, a new one if it was ended during
        // one; and a cleanup step in which players had priority is followed by another (CR 514.3a).
        (turn.number, turn.active, Step::Cleanup)
    } else {
        match ending.next() {
            // With no attackers declared, the declare blockers and combat damage steps are skipped (CR 508.8): an
            // attacker removed from combat since does not count.
            Some(Step::DeclareBlockers) if !turn.attacked => (turn.number, turn.active, Step::EndOfCombat),
            // After a first-strike damage step comes the regular one (CR 510.4).
            Some(Step::EndOfCombat) if ending == Step::CombatDamage && turn.damage_round == 1 => {
                (turn.number, turn.active, Step::CombatDamage)
            }
            // An additional end step follows the end step that ends, alone: the cleanup step comes after the last of them
            // (Y'shtola Rhul's ruling).
            Some(Step::Cleanup) if ending == Step::End && turn.extra_end_steps > 0 => (turn.number, turn.active, Step::End),
            Some(next) => (turn.number, turn.active, next),
            None if turn.scheduled => (turn.number, turn.active, Step::Cleanup),
            None => (turn.number + 1, next_turn_taker(s, turn.active), Step::Untap),
        }
    };
    // CR 500.8, 500.10: leave an individual phase only after its last unskipped step. Skipped combat steps
    // and first-strike damage remain within that phase; repeat cleanup/end steps remain within the ending phase.
    let boundary = turn.phase_only_step.is_some()
        || ending.next().is_none_or(|next| next.phase() != ending.phase());
    if turn.scheduled && !turn.ending && !turn.cleanup_again && !(ending == Step::End && turn.extra_end_steps > 0) && boundary {
        let mut schedule = s.phase_schedule.clone();
        if !schedule.is_empty() { schedule.remove(0); }
        if let Some(next) = schedule.first().copied() {
            step = first_step(next.spec, main_seen);
            number = turn.number;
            active = turn.active;
            s.update_turn(|t| {
                t.phase_id = next.id;
                t.phase_additional = next.additional;
                t.phase_only_step = match next.spec { PhaseSpec::Only(step) => Some(step), _ => None };
            });
        }
        if schedule.is_empty() {
            number = turn.number + 1;
            active = next_turn_taker(s, turn.active);
            step = Step::Untap;
        }
        s.set_phase_schedule(schedule);
    }
    let previous = turn.active;
    s.update_turn(|t| {
        if number != turn.number {
            t.previous = previous;
        }
        t.number = number;
        t.active = active;
        if number != turn.number {
            t.scheduled = false;
            t.phase_id = 0;
            t.phase_additional = false;
            t.phase_only_step = None;
            t.extra_end_steps = 0;
        } else if ending == Step::End && step == Step::End && turn.extra_end_steps > 0 {
            t.extra_end_steps -= 1;
        }
        if turn.ending {
            t.scheduled = false;
            t.phase_id = 0;
            t.phase_additional = false;
            t.phase_only_step = None;
            t.extra_end_steps = 0;
        }
        t.set_step(step);
        t.priority = active;
        t.passes = 0;
        t.ending = false;
        t.combat_ending = false;
        t.cleanup_again = false;
    });
    if (turn.ending || number != turn.number) && !s.phase_schedule.is_empty() {
        s.set_phase_schedule(Vec::new());
    }
    s.frame_push(Frame::new(FrameKind::BeginStep));
}

/// CR 500.8 and 500.10: materialise the remaining ordinary phases only when an effect needs a schedule.
/// Own the Day inserts after the current main; World at War anchors the second main and names the added combat.
pub fn add_phases(cx: &mut Cx, after: PhaseAnchor, phases: &[PhaseSpec]) -> Vec<u32> {
    if phases.is_empty() || cx.s().turn.number == 0 || cx.s().turn.ending { return Vec::new(); }
    let turn = cx.s().turn;
    let mut schedule = cx.s().phase_schedule.clone();
    if !turn.scheduled {
        let future: &[Phase] = match turn.step() {
            Step::Untap | Step::Upkeep | Step::Draw => &[Phase::Main, Phase::Combat, Phase::Main, Phase::Ending],
            Step::PrecombatMain => &[Phase::Combat, Phase::Main, Phase::Ending],
            Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers | Step::CombatDamage | Step::EndOfCombat => &[Phase::Main, Phase::Ending],
            Step::PostcombatMain => &[Phase::Ending],
            Step::End | Step::Cleanup => &[],
        };
        for phase in std::iter::once(turn.step().phase()).chain(future.iter().copied()) {
            let id = cx.st().timestamp();
            schedule.push(cantrip_state::ScheduledPhase { id, spec: PhaseSpec::Whole(phase), additional: false });
        }
    }
    let anchor = match after {
        PhaseAnchor::Current => Some(0),
        PhaseAnchor::Id(id) => schedule.iter().position(|p| p.id == id),
        PhaseAnchor::Main(n) => {
            let occurred = cx.history().this_turn().iter().filter(|r| matches!(r.event, Event::StepBegan { step } if step.is_main())).count();
            let mut ordinal = occurred.saturating_sub(usize::from(turn.step().is_main()));
            schedule.iter().position(|p| {
                if p.spec.phase() == Phase::Main { ordinal += 1; ordinal == n as usize } else { false }
            })
        }
    };
    let Some(anchor) = anchor else { return Vec::new() };
    let added: Vec<_> = phases.iter().map(|&spec| cantrip_state::ScheduledPhase { id: cx.st().timestamp(), spec, additional: true }).collect();
    let ids = added.iter().map(|p| p.id).collect();
    schedule.splice(anchor + 1..anchor + 1, added);
    let current = schedule[0];
    cx.st().update_turn(|t| {
        t.scheduled = true;
        t.phase_id = current.id;
    });
    cx.st().set_phase_schedule(schedule);
    ids
}

fn first_step(spec: PhaseSpec, main_seen: bool) -> Step {
    match spec {
        PhaseSpec::Only(step) => step,
        PhaseSpec::Whole(Phase::Beginning) => Step::Untap,
        PhaseSpec::Whole(Phase::Main) => if main_seen { Step::PostcombatMain } else { Step::PrecombatMain },
        PhaseSpec::Whole(Phase::Combat) => Step::BeginCombat,
        PhaseSpec::Whole(Phase::Ending) => Step::End,
    }
}

/// Who takes the next turn: the most recently created extra turn first (CR 500.7), skipping a player who has left the
/// game (CR 800.4k), else the next player in turn order.
fn next_turn_taker(s: &mut GameState, after: PlayerId) -> PlayerId {
    let mut queue = s.extra_turns.clone();
    let mut taker = None;
    while let Some(t) = queue.pop() {
        if !s.player(t.player).lost {
            taker = Some(t);
            break;
        }
        // The extra turn of a player who has left would have begun now (CR 800.4k-m).
        turn_would_have_begun(s, t.player);
    }
    if queue.len() != s.extra_turns.len() {
        s.set_extra_turns(queue);
    }
    let rotation = s.turn.rotation;
    let (player, extra) = match taker {
        // An extra turn: the rotation stays where it was, at the player whose ordinary turn this one follows.
        Some(t) => {
            if rotation.is_none() {
                s.update_turn(|t| t.rotation = Some(after));
            }
            (t.player, t.id)
        }
        // The rotation goes on from the last ordinary turn, not from an extra turn taken since (CR 500.7).
        None => {
            if rotation.is_some() {
                s.update_turn(|t| t.rotation = None);
            }
            let from = rotation.unwrap_or(after);
            let next = next_in_game(s, from);
            // Each player passed over has left: their turn would have begun now (CR 800.4k-m).
            let mut skipped = s.next_player(from);
            while skipped != next && skipped != from {
                turn_would_have_begun(s, skipped);
                skipped = s.next_player(skipped);
            }
            (next, 0)
        }
    };
    if s.turn.extra != extra {
        s.update_turn(|t| t.extra = extra);
    }
    // "At the beginning of that turn's [step]" waits for one extra turn (CR 603.7): once that turn has ended without the
    // step, or will never be taken, the ability never triggers.
    // And a "when [event]" ability about an object that has since changed zones can never trigger again: that object is gone,
    // and what it became is a new object the ability doesn't refer to (CR 400.7, 603.7c). The event that moved it was looked
    // at for triggers as it happened, so dropping it now loses nothing.
    let stale = |d: &cantrip_state::Delayed, s: &GameState| match d.when {
        DelayedWhen::PhaseStep { .. } => true,
        DelayedWhen::ExtraTurnStep { turn, .. } => turn != extra && !s.extra_turns.iter().any(|t| t.id == turn),
        // "On your next turn" is the turn ending now, if it is that player's first since the ability was made, and it began: a
        // skipped turn ends here too, under the player who skipped it, but it is not their next turn, and the ability waits
        // for the first that isn't skipped (CR 614.10a).
        DelayedWhen::NextTurnStep { player, after, .. } => s.turn.active == player && s.player(player).turn_began > after,
        DelayedWhen::WhenEvent => s.obj(d.watched()).became.is_some(),
        _ => false,
    };
    // So do the effects about that turn alone ("during that turn"): they last through its cleanup step, and end as the next turn
    // begins, or as it would have begun for a player who has left (CR 500.7, 800.4m).
    let spent = |e: &ContinuousEffect, s: &GameState| {
        matches!(e.duration, Duration::EndOfExtraTurn(turn) if turn != extra && !s.extra_turns.iter().any(|t| t.id == turn))
    };
    if s.effects.iter().any(|e| spent(e, s)) {
        let ended: Vec<_> = s.effects.iter().filter(|e| spent(e, s)).map(|e| e.duration).collect();
        s.effects_retain(|e| !ended.contains(&e.duration));
    }
    if s.delayed.iter().any(|d| stale(d, s)) {
        let mut at = 0;
        while at < s.delayed.len() {
            if stale(&s.delayed[at], s) {
                s.delayed_remove(at);
            } else {
                at += 1;
            }
        }
    }
    player
}

/// A player who has left the game would have begun a turn now, which doesn't begin (CR 800.4k): the effects that last until
/// their next turn - "until your next turn", and the delayed abilities that last as long - end now, as they would have as
/// it began (CR 800.4m: they neither expire as the player leaves nor last forever). Goad from a player who has left
/// ("until your next turn", CR 701.15a) ends here, as do Sparas Adjudicators' "can't attack or block until its controller's
/// next turn".
fn turn_would_have_begun(s: &mut GameState, p: PlayerId) {
    // "During that player's next untap step" too: the untap step would have been that turn's first step.
    let waits = |e: &ContinuousEffect| matches!(e.duration, Duration::UntilNextTurnOf(q) | Duration::EndOfNextTurnOf(q) | Duration::DuringNextUntapStepOf(q) if q == p)
        || matches!(e.duration, Duration::UntilNextStepOf { player: Some(q), .. } | Duration::UntilEndOfNextStepOf { player: Some(q), .. } if q == p);
    if s.effects.iter().any(waits) {
        s.effects_retain(|e| !waits(e));
    }
    for at in (0..s.delayed.len()).rev() {
        if matches!(s.delayed[at].when, DelayedWhen::UntilNextTurnOf(q) | DelayedWhen::NextTurnStep { player: q, .. } if q == p)
            || matches!(s.delayed[at].when, DelayedWhen::StepOfNextTurn { of: Some(player), .. } if player == p) {
            s.delayed_remove(at);
        }
    }
}

/// The start of the game (CR 103): who goes first, the opening hands, mulligans.
/// The mulligans of `taken` that count (CR 103.5c): in a multiplayer game the first is free, putting no card on the
/// bottom and not counting toward the number a player may take. That is the number of cards put on the bottom on keeping.
pub fn counted_mulligans(s: &GameState, taken: u8) -> u8 {
    if s.players.len() >= 3 { taken.saturating_sub(1) } else { taken }
}

pub fn opening(cx: &mut Cx) -> R<Flow> {
    // A scenario's fixed deal has dealt and kept the hands already, and goes straight to the opening-hand actions.
    let dealt = cx.s().seats().all(|p| cx.s().player(p).kept);
    if !dealt {
        let chooser = cx.s().turn.starting;
        let candidates: Vec<PlayerId> = cx.s().seats().collect();
        if let Answer::Player(first) =
            cx.ask_core(chooser, RequestKind::ChooseStartingPlayer { candidates }, false)?
        {
            cx.st().update_turn(|t| {
                t.starting = first;
                t.active = first;
                t.priority = first;
            });
        }
    }
    let order: Vec<PlayerId> = cx.s().apnap().collect();
    reveal_companions(cx, &order);
    if !dealt {
        for &p in &order {
            for _ in 0..7 {
                ops::draw(cx, p);
            }
        }
    }

    // The London mulligan (CR 103.5): undecided players decide in turn order, those who mulligan redraw together,
    // and the round repeats.
    loop {
        // A player who has left decides nothing (CR 800.4a).
        let undecided: Vec<PlayerId> = order.iter().copied().filter(|&p| !cx.s().player(p).kept && !cx.s().player(p).lost).collect();
        if undecided.is_empty() {
            break;
        }
        let mut redraw = Vec::new();
        for p in undecided {
            let mulligans = cx.s().player(p).mulligans;
            // Seven counted mulligans and the player keeps (`docs/rules-exceptions.md` RX-6), putting all seven on the
            // bottom as any keep does.
            let answer = if counted_mulligans(cx.s(), mulligans) >= 7 {
                Some(Answer::No)
            } else {
                cx.ask_rule(p, RequestKind::Mulligan { mulligans })?
            };
            match answer {
                None => continue,
                Some(Answer::Yes) => redraw.push(p),
                Some(_) => {
                    cx.st().update_player(p, |pl| pl.kept = true);
                    let bottom = counted_mulligans(cx.s(), mulligans);
                    if bottom > 0 {
                        let hand = cx.s().zone(Zone::hand(p)).to_vec();
                        let answer = cx.ask_rule(
                            p,
                            RequestKind::choose_cards(hand[0], Purpose::Discard, hand.clone(), hand, u32::from(bottom), false, ""),
                        )?;
                        if let Some(Answer::Cards(bottom)) = answer {
                            for card in bottom {
                                let library = Zone::library(p);
                                let new = ops::move_to(cx, card, library);
                                // To the bottom, not the top.
                                cx.st().zone_remove(library, new);
                                cx.st().zone_push(library, new);
                            }
                        }
                    }
                }
            }
        }
        for p in redraw {
            for id in cx.s().zone(Zone::hand(p)).to_vec() {
                ops::move_to(cx, id, Zone::library(p));
            }
            shuffle_library(cx, p);
            for _ in 0..7 {
                ops::draw(cx, p);
            }
            cx.st().update_player(p, |pl| pl.mulligans += 1);
        }
    }

    // CR 103.6a: with the hands kept, each player in turn order, the starting player first, may begin the game with cards
    // of their opening hand on the battlefield.
    for &p in &order {
        for card in cx.s().zone(Zone::hand(p)).to_vec() {
            // A player who has left begins the game with nothing (CR 800.4a): their cards have left with them.
            if cx.s().player(p).lost {
                break;
            }
            let Some(action) = cx.card(cx.s().obj(card).card).opening_hand else { continue };
            if !(action.condition)(cx, &Asker { instance: None, source: card, controller: p, by: None, spell: false }) {
                continue;
            }
            if !matches!(cx.ask_rule(p, RequestKind::May { source: card, purpose: Purpose::Ask(&BEGIN_ON_BATTLEFIELD) })?, Some(Answer::Yes)) {
                continue;
            }
            let with: Vec<(CounterKind, i32)> = action.counter.map(|(k, n)| (k, i32::from(n))).into_iter().collect();
            ops::put_onto_battlefield_by(cx, &[(card, p, false, p)], &with)?;
            let hand = cx.s().zone(Zone::hand(p)).to_vec();
            if action.exile_from_hand > 0 && !hand.is_empty() {
                let count = u32::from(action.exile_from_hand).min(u32::try_from(hand.len()).unwrap_or(u32::MAX));
                let ask = RequestKind::choose_cards(card, Purpose::Exile, hand.clone(), hand, count, false, "");
                if let Some(Answer::Cards(cards)) = cx.ask_rule(p, ask)? {
                    for c in cards {
                        ops::move_to_for(cx, c, Zone::exile(p), ZoneReason::Exile);
                    }
                }
            }
        }
    }
    Ok(Flow::Done)
}

/// CR 103.2b, 702.139a: once the starting player is known, each player in turn order, the starting player first, reveals the
/// companion they chose as their deck was built, if their starting deck fulfils its condition (`Game::new` judged it as the
/// deck was dealt). The card stays outside the game, revealed (`Player::companion`). Nothing is asked: the choice was made at
/// deck building, so no question - or its absence - says anything of a player's sideboard.
fn reveal_companions(cx: &mut Cx, order: &[PlayerId]) {
    for &p in order {
        let Some(card) = cx.s().player(p).companion_to_reveal else { continue };
        cx.st().update_player(p, |pl| pl.companion_to_reveal = None);
        // A player who has left reveals nothing (CR 800.4a).
        if cx.s().player(p).lost {
            continue;
        }
        cx.st().update_player(p, |pl| pl.companion = Some(card));
        cx.record(Event::Revealed { player: p, card });
    }
}

/// The cards with companion `player` owns outside the game whose condition their starting deck - their library and hand, before
/// the game begins - fulfils, the game's format's minimum deck size being the one a condition names (`GameState::minimum_deck_size`,
/// CR 702.139a-b, 103.2b).
pub fn eligible_companions(q: &dyn Query, player: PlayerId) -> Vec<ObjectId> {
    let s = q.state();
    let outside = s.zone(Zone::sideboard(player));
    if !outside.iter().any(|&id| q.card(s.obj(id).card).companion.is_some()) {
        return Vec::new();
    }
    let cards: Vec<ObjectId> = s.zone(Zone::library(player)).iter().chain(s.zone(Zone::hand(player)))
        .chain(s.zone(Zone::COMMAND).iter().filter(|&&id| s.obj(id).owner == player && s.obj(id).commander))
        .copied().collect();
    let deck = StartingDeck { player, cards: &cards, minimum_size: s.minimum_deck_size };
    outside.iter().copied().filter(|&id| q.card(s.obj(id).card).companion.is_some_and(|condition| condition(q, &deck))).collect()
}

fn push_priority(cx: &mut Cx) {
    cx.st().frame_push(Frame::new(FrameKind::Priority));
}

/// The step's turn-based actions (CR 500.2), then its priority round.
pub fn begin_step(cx: &mut Cx) -> R<Flow> {
    let turn = cx.s().turn;
    let active = turn.active;
    if turn.step() != Step::Untap {
        // CR 103.8a skips the first draw STEP, including its triggers and priority, not just its draw action.
        let first_draw = turn.step() == Step::Draw && turn.number == 1 && active == turn.starting && cx.s().players.len() == 2 && !turn.phase_additional;
        if first_draw || crate::static_rules::skips_step(cx, active, turn.step()) {
            return Ok(Flow::Done);
        }
        crate::settle::step_began(cx, turn.step());
    }
    match turn.step() {
        Step::Untap => {
            if turn.phase_additional {
                // Another beginning phase has the same skip rules and step deadlines, without beginning another turn.
                if crate::static_rules::skips_step(cx, active, Step::Untap) { return Ok(Flow::Done); }
                crate::settle::expire_step_deadlines(cx, Step::Untap);
                crate::phasing::untap_step(cx, active);
                crate::day_night::check_untap_step(cx)?;
                untap_step(cx, active)?;
                return Ok(Flow::Done);
            }
            // CR 614.10, 500.11: a turn that would begin may be skipped instead, and then never begins. The control of a
            // player is still in force while that is decided: it ends only as the next turn begins (CR 723.1).
            if skip_turn(cx, active)? {
                // The next player's turn takes its place, under this one's number: as if cleanup had just ended it.
                cx.st().update_turn(|t| {
                    t.number -= 1;
                    t.set_step(Step::Cleanup);
                });
                return Ok(Flow::Done);
            }
            // The turn begins: the control of a player for the turn before ends (CR 723.1), lasting through its cleanup and
            // any turn skipped since, and the effects waiting to control this turn's player during it apply.
            let controllers: Vec<PlayerId> = (0..cx.s().players.len()).map(|i| cx.s().player_controller(PlayerId(i as u8))).collect();
            if cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::ControlsPlayer(_))) {
                cx.st().effects_retain(|e| !matches!(e.kind, EffectKind::ControlsPlayer(_)));
            }
            take_controlled_turn(cx, active);
            ops::record_player_control(cx, &controllers);
            // CR 502: the turn begins; the active player's permanents untap. No one gets priority.
            let number = turn.number;
            cx.st().update_player(active, |p| {
                p.turn_began = number;
                p.lands_played = 0;
            });
            // How many turns a player has taken is asked of the history (`cantrip_mechanics::history::in_your_first_turns`).
            // Whether it is an extra turn is part of its beginning (CR 500.7: "haven't begun an extra turn this game").
            cx.record(Event::TurnBegan { player: active, extra: turn.extra != 0 });
            // "Until your next turn" ends as that turn begins (CR 611.2b).
            if cx.s().effects.iter().any(|e| e.duration == Duration::UntilNextTurnOf(active)) {
                cx.st().effects_retain(|e| e.duration != Duration::UntilNextTurnOf(active));
                ops::settle_lasting_and_control(cx);
            }
            // The same for a delayed ability that lasts until then (CR 603.7b).
            for at in (0..cx.s().delayed.len()).rev() {
                if cx.s().delayed[at].when == DelayedWhen::UntilNextTurnOf(active) {
                    cx.st().delayed_remove(at);
                }
            }
            // "Until the end of your next turn" is this turn from here on: it ends in the cleanup step (CR 514.2, 611.2a).
            for at in 0..cx.s().effects.len() {
                if cx.s().effects[at].duration == Duration::EndOfNextTurnOf(active) {
                    let effect = ContinuousEffect { duration: Duration::EndOfTurn, ..cx.s().effects[at] };
                    cx.st().effect_set(at, effect);
                }
            }
            // The turn begins even when its first step is skipped (CR 500.11, 614.10a).
            // Turn-bound durations ended above; skipped untap-step deadlines, phasing, day/night, exert and untaps wait.
            if crate::static_rules::skips_step(cx, active, Step::Untap) {
                return Ok(Flow::Done);
            }
            crate::settle::expire_step_deadlines(cx, Step::Untap);
            // CR 502.1-2: phasing, then day/night, before the simultaneous untap.
            crate::phasing::untap_step(cx, active);
            crate::day_night::check_untap_step(cx)?;
            untap_step(cx, active)?;
            return Ok(Flow::Done);
        }
        Step::Draw => ops::draw_cards(cx, active, 1),
        Step::PrecombatMain => ops::add_lore_counters(cx),
        Step::DeclareAttackers => declare_attackers(cx)?,
        Step::DeclareBlockers => declare_blockers(cx)?,
        Step::CombatDamage => combat_damage(cx)?,
        Step::Cleanup => {
            cleanup(cx)?;
            // CR 514.3a: the state-based actions due are performed; if any were, or a triggered ability is waiting to be
            // put on the stack (one that triggered during the cleanup step, or while an effect ended the turn, CR
            // 724.1f), players get priority, and another cleanup step follows this one.
            let acted = crate::settle::state_based_actions_alone(cx)?;
            if acted || !cx.s().triggers.is_empty() {
                cx.st().update_turn(|t| t.cleanup_again = true);
                push_priority(cx);
            }
            return Ok(Flow::Done);
        }
        _ => {}
    }
    push_priority(cx);
    Ok(Flow::Done)
}

/// The turn `player` takes now is their "next turn" for every effect waiting to control them during it (CR 723.1, 723.1b: a
/// skipped turn was not taken, and they wait on). Each becomes the control of the player for this turn, keeping the
/// timestamp it was created with: the last created whose controller is still in the game is the one that works (723.1a,
/// `GameState::player_controller`), and if that controller leaves during the turn the next newest works on (CR 800.4a ends
/// only the leaver's). One whose controller has already left gives no control (CR 800.4a). Each that says "after that turn,
/// that player takes an extra turn" (Emrakul, the Promised End) gives that turn now, the overwritten ones and those of a
/// controller who has left too: the newest extra turn is taken first (CR 500.7), so it comes right after this one.
fn take_controlled_turn(cx: &mut Cx, player: PlayerId) {
    let waiting = |e: &ContinuousEffect| matches!(e.kind, EffectKind::ControlNextTurn { player: p, .. } if p == player);
    if !cx.s().effects.iter().any(waiting) {
        return;
    }
    let pending: Vec<ContinuousEffect> = cx.s().effects.iter().copied().filter(waiting).collect();
    cx.st().effects_retain(|e| !waiting(e));
    for e in &pending {
        if cx.s().in_game(e.controller) {
            // Created when the effect it comes from was: what a later one overwrites (CR 723.1a).
            cx.st().effect_push(ContinuousEffect { kind: EffectKind::ControlsPlayer(player), ..*e });
        }
    }
    for e in &pending {
        if matches!(e.kind, EffectKind::ControlNextTurn { then_extra_turn: true, .. }) {
            cantrip_api::Game::add_extra_turn(cx, player);
        }
    }
    // An effect that begins changes the game with no event (CR 603.8).
    cx.state_changed();
}

/// What the effect that makes a player skip their next turn does, for the player who orders it with an optional skip of the
/// same turn (CR 616.1).
const SKIP_NEXT_TURN_HINT: &str = "you skip this turn";

/// Whether the player skips the turn about to begin (CR 614.10, 500.11). Two kinds of replacement effect skip it:
///
/// - a resolved spell or ability's "[that player] skips their next N turns", one `EffectKind::SkipNextTurn` for each turn
///   to skip, which must be applied: the oldest is used up by this turn, the rest wait for the turns after it
///   (CR 614.10a);
/// - a permanent's "if you would begin your turn while ..., you may skip that turn instead. If you do, ..." (Time
///   Vault, `CardDef::turn_begin`), which the player may decline. Each that applies is offered in turn, and the first
///   accepted skips the turn and does what it says it does then.
///
/// With both, the player chooses which applies (CR 616.1): an optional skip chosen first is taken (declining it would
/// leave the turn to the effect that must skip it), and the effect that must skip a turn then still waits for the next
/// turn, as it skipped none (CR 614.10a). Nothing is asked when nothing applies.
fn skip_turn(cx: &mut Cx, player: PlayerId) -> R<bool> {
    let pending = cx.s().effects.iter().position(|e| e.kind == EffectKind::SkipNextTurn(player));
    let mut optional: Vec<(ObjectId, &'static TurnBeginRule)> = Vec::new();
    for id in cx.s().battlefield().to_vec() {
        let obj = cx.s().obj(id);
        if obj.controller != player {
            continue;
        }
        // A scan once a turn of what is on the battlefield, for a card that has such a rule: of every definition it has, a
        // merged permanent's other components' among them (CR 702.140e), each asked as its own card's.
        let texts: Vec<(CardId, Option<Linked>)> = cx.cards().texts_linked(cx.s(), id).collect();
        if texts.iter().any(|&(card, _)| !cx.card(card).turn_begin.is_empty()) && cx.lost_all_abilities(id) {
            continue;
        }
        for (card, by) in texts {
            let asker = Asker { instance: None, source: id, controller: player, by, spell: false };
            for rule in cx.card(card).turn_begin {
                if (rule.applies)(&*cx, &asker, player) {
                    optional.push((id, rule));
                }
            }
        }
    }
    let Some(at) = pending else {
        for (id, rule) in optional {
            if cx.ask_rule(player, RequestKind::May { source: id, purpose: Purpose::Ask(rule.question) })? == Some(Answer::Yes) {
                // A replacement effect's own doing, its controller's (CR 609.1, 614.10).
                cx.acting_as(player, |cx| (rule.on_skip)(cx, &Asker { instance: None, source: id, controller: player, by: None, spell: false }))?;
                return Ok(true);
            }
        }
        return Ok(false);
    };
    if !optional.is_empty() {
        let mut sources = vec![cx.s().effects[at].source];
        let mut hints = vec![SKIP_NEXT_TURN_HINT];
        for &(id, rule) in &optional {
            sources.push(id);
            hints.push(rule.hint);
        }
        let ask = RequestKind::ChooseReplacement { subject: None, life: None, sources, hints, damage: None, turn: true, counters: Vec::new(), draw: false };
        if let Some(Answer::Index(i)) = cx.ask_rule(player, ask)?
            && i > 0
            && let Some(&(id, rule)) = optional.get(i as usize - 1)
        {
            // A replacement effect's own doing, its controller's (CR 609.1, 614.10).
            cx.acting_as(player, |cx| (rule.on_skip)(cx, &Asker { instance: None, source: id, controller: player, by: None, spell: false }))?;
            return Ok(true);
        }
    }
    let keep: Vec<bool> = (0..cx.s().effects.len()).map(|i| i != at).collect();
    cx.st().effects_keep(&keep);
    Ok(true)
}

/// CR 502.3: the active player's permanents untap, all at once. Except an exerted one, for the untap step it sits out
/// (CR 701.43a), and one that doesn't untap during its controller's untap step (CR 502.3: its own `Keyword::DoesNotUntap`,
/// another permanent's `CardDef::untap_bans`, or a resolved effect's, `static_rules::untap_step_skips`); and one with a stun counter
/// has a counter removed from it instead (CR 122.1d). With them, the other players' permanents that a static ability of
/// a permanent another player controls untaps "during each other player's untap step" (`CardDef::untap_in_others_untap_steps`).
pub(crate) fn untap_step(cx: &mut Cx, active: PlayerId) -> R<()> {
    // CR 502.3: the active player determines which permanents will untap, then they untap together. A permanent whose
    // card says "you may choose not to untap this ... during your untap step" is the active player's to keep tapped:
    // asked of each, in battlefield order, before any untaps.
    let mut untapping = Vec::new();
    // The permanents of the other players with an ability that untaps permanents during this untap step: a table lookup
    // per permanent, and nothing more in a game without one.
    let mut others: Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> = Vec::new();
    for id in cx.s().battlefield().to_vec() {
        let obj = cx.s().obj(id);
        if obj.controller != active {
            // A merged permanent has its other components' rules too (CR 702.140e).
            for (card, by) in cx.cards().texts_linked(cx.s(), id) {
                let def = cx.card(card);
                if !def.untap_in_others_untap_steps.is_empty() {
                    others.push((id, obj.controller, def, by));
                }
            }
            continue;
        }
        if !obj.tapped
            || obj.marks.exerted_by & (1 << active.0) != 0
            || cx.chars(id).has(Keyword::DoesNotUntap)
            || crate::static_rules::untap_banned(cx, cx.cards(), id)
            || crate::static_rules::untap_step_skips(cx, cx.cards(), id, active)
        {
            continue;
        }
        if cx.cards().texts(cx.s(), obj).any(|c| cx.card(c).may_not_untap) && !cx.lost_all_abilities(id) {
            let ask = RequestKind::May { source: id, purpose: Purpose::Ask(&UNTAP_IT) };
            if cx.ask_rule(active, ask)? != Some(Answer::Yes) {
                continue;
            }
        }
        untapping.push(id);
    }
    crate::untap_limits::restrict(cx, active, &mut untapping)?;
    // "Untap [these] during each other player's untap step" (CR 611.3a): what each rule is true of now, before anything
    // untaps, untaps with the active player's permanents and is no one's choice (Bender's Waterskin ruling, 2025-10-02).
    // A permanent of a player who isn't active is outside "doesn't untap during its controller's untap step" (CR 502.3,
    // 701.43a); a stun counter still replaces its untap (CR 122.1d, `ops::untap`). A source that has lost all its
    // abilities has no such rule (CR 613.1f).
    others.retain(|&(id, _, _, _)| !cx.lost_all_abilities(id));
    if !others.is_empty() {
        for id in cx.s().battlefield().to_vec() {
            let obj = cx.s().obj(id);
            if obj.controller == active || !obj.tapped || obj.marks.exerted_by & (1 << active.0) != 0 {
                continue;
            }
            if others.iter().any(|&(source, controller, def, by)| {
                let asker = Asker { instance: None, source, controller, by, spell: false };
                def.untap_in_others_untap_steps.iter().any(|rule| rule(&*cx, &asker, id))
            }) {
                untapping.push(id);
            }
        }
    }
    ops::untap_together(cx, &untapping);
    // "During its controller's next untap step" and "during [this player's] next untap step" end with this untap step,
    // whatever it untapped (CR 611.2a; the Frost Titan ruling): the first is over for each permanent the active player
    // controls now, whoever controlled it as the effect began (the Tamiyo, the Moon Sage ruling).
    let s = cx.s();
    let ends = |e: &ContinuousEffect| match e.duration {
        Duration::DuringNextUntapStepOf(p) => p == active,
        Duration::DuringItsControllersNextUntapStep => s.obj(e.affected).controller == active,
        Duration::DuringControllersNextUntapSteps(n) => n <= 1 && s.obj(e.affected).controller == active,
        _ => false,
    };
    if s.effects.iter().any(ends) {
        let keep: Vec<bool> = s.effects.iter().map(|e| !ends(e)).collect();
        cx.st().effects_keep(&keep);
    }
    // These are remaining future steps, not a tally of past actions. Every independent effect advances together;
    // skipped steps never enter this procedure. Read the controllers before settling control after the untap.
    for at in 0..cx.s().effects.len() {
        let e = cx.s().effects[at];
        if let Duration::DuringControllersNextUntapSteps(n) = e.duration
            && cx.s().obj(e.affected).controller == active {
            cx.st().effect_set(at, ContinuousEffect { duration: Duration::DuringControllersNextUntapSteps(n - 1), ..e });
        }
    }
    // No player gets priority in the untap step (CR 502.4), so no state-based actions are checked before the upkeep begins:
    // an effect "for as long as [it] remains tapped" ends the moment its source untaps (CR 611.2b), and the controller it
    // gave back is the one the upkeep's "at the beginning of your upkeep" abilities see (CR 613.1b).
    ops::settle_lasting_and_control(cx);
    // That untap step has now happened for everything its player exerted, whoever controls it now
    // (CR 701.43a-b). Phasing leaves the object and its timed effects intact (CR 702.26d,f): a
    // prohibition expires even while Oubliette keeps that object phased out. Other players' bits remain.
    let surviving: Vec<_> = cx.s().battlefield().iter().chain(cx.s().zone(Zone::PHASED_OUT)).copied().collect();
    for id in surviving {
        if cx.s().obj(id).marks.exerted_by & (1 << active.0) != 0 {
            cx.st().update_obj(id, |o| o.marks.exerted_by &= !(1 << active.0));
        }
    }
    Ok(())
}

/// "You may choose not to untap this ... during your untap step" (CR 502.3), asked of the active player for each such
/// permanent of theirs that is tapped.
const UNTAP_IT: Question = Question::new("UntapInUntapStep", "untap it during this untap step?", "You may choose to leave it tapped", "Untap it", "Leave it tapped");

/// The battles the active player may attack (CR 310.9b, 508.1b): those whose protector is one of the defending
/// players. A Siege its attacker controls is one of them (CR 310.12a); one the attacker protects never is.
fn attackable_battles(q: &dyn Query, defending: &[PlayerId]) -> Vec<ObjectId> {
    let s = q.state();
    if !s.battlefield().iter().any(|&id| s.obj(id).marks.protector.is_some()) {
        return Vec::new();
    }
    s.battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            s.obj(id).marks.protector.is_some_and(|p| defending.contains(&p)) && {
                let c = q.chars(id);
                c.is(CardType::Battle) && !c.has(Keyword::CantBeAttacked)
            }
        })
        .collect()
}

/// The planeswalkers the active player may attack (CR 508.1b): those a defending player controls, except one that
/// can't be attacked (CR 508.1c).
fn attackable_planeswalkers(q: &dyn Query, defending: &[PlayerId]) -> Vec<ObjectId> {
    let s = q.state();
    s.battlefield()
        .iter()
        .copied()
        .filter(|&id| defending.contains(&s.obj(id).controller) && {
            let c = q.chars(id);
            c.is(CardType::Planeswalker) && !c.has(Keyword::CantBeAttacked)
        })
        .collect()
}

/// What `active` may attack (CR 508.1b): each opponent (the defending players), the planeswalkers they control and the
/// battles they protect that can be attacked, apart, and all of them as targets in that order.
fn attack_targets(q: &dyn Query, active: PlayerId) -> (Vec<PlayerId>, Vec<ObjectId>, Vec<ObjectId>, Vec<Target>) {
    let players: Vec<PlayerId> = q.state().opponents(active).collect();
    // CR 310.9b, 508.1b: every battle a defending player protects, whoever controls it. Looked for only when a battle
    // is on the battlefield.
    let battles = attackable_battles(q, &players);
    let planeswalkers = attackable_planeswalkers(q, &players);
    let targets: Vec<Target> = players
        .iter()
        .map(|&p| Target::Player(p))
        .chain(planeswalkers.iter().chain(&battles).map(|&id| Target::Object(id)))
        .collect();
    (players, planeswalkers, battles, targets)
}

/// Whether the static abilities on the battlefield leave `creature` something to attack were its controller attacking now
/// (CR 508.1c): one of the players, planeswalkers and battles its controller could attack.
pub(crate) fn attack_rules_allow(q: &dyn Query, cards: &CardTable, creature: ObjectId) -> bool {
    crate::static_rules::attack_rules_allow_any(q, cards, creature, || attack_targets(q, q.state().obj(creature).controller).3)
}

fn defending_player(cx: &Cx, attack: Target) -> Option<PlayerId> {
    defending_player_of(cx.s(), attack)
}

fn declare_attackers(cx: &mut Cx) -> R<()> {
    spend_next_turn_effects(cx);
    let active = cx.s().turn.active;
    let attack_haste = std::cell::OnceCell::new();
    let eligible: Vec<ObjectId> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let obj = cx.s().obj(id);
            let chars = cx.chars(id);
            obj.controller == active
                && chars.is(CardType::Creature)
                && !chars.is(CardType::Battle)
                && !obj.tapped
                // CR 508.1c: an effect saying it can't attack; a static ability's restriction is read below, of each attack.
                && !cx.s().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::CantAttack && in_force(cx, e))
                // CR 508.1c: a resolved rule about an open set that says it, or those who control it, can't attack this turn
                // (`LastingRuleKind::CantAttack`: Peace in Our Time).
                && crate::static_rules::lasting_rules_on(cx, cx.cards(), id, LastingRuleKind::CantAttack).next().is_none()
                && (!summoning_sick(cx.cards(), cx.s(), &cx.g.log, id)
                    || !attack_haste.get_or_init(|| crate::static_rules::attack_haste_sources(cx)).is_empty())
                // CR 702.3b: defender, unless an effect or a static ability lets it attack as though it didn't have it
                // (CR 609.4), which lifts no other restriction: asked last, of a creature that could otherwise attack.
                && (!chars.has(Keyword::Defender) || crate::static_rules::may_attack_despite_defender(&*cx, cx.cards(), id))
        })
        .collect();
    if eligible.is_empty() {
        return Ok(());
    }
    let (players, planeswalkers, battles, targets) = attack_targets(&*cx, active);
    // CR 508.1c: the attacks a static ability forbids ("this creature can't attack unless ...", "creatures with flying
    // can't attack you"). A creature forbidden every attack open to it can't attack at all.
    let mut barred = crate::static_rules::attacks_barred(&*cx, cx.cards(), &eligible, &targets);
    // The permission may name particular defenders; include a sick creature
    // only for the attacks it permits. Every independent restriction still holds.
    if let Some(sources) = attack_haste.get().filter(|s| !s.is_empty()) {
        for &creature in &eligible {
            if summoning_sick(cx.cards(), cx.s(), &cx.g.log, creature) {
                for &defender in &targets {
                    if !sources.iter().any(|(asker, def)| def.attacks_as_though_haste.iter().any(|rule| rule(cx, asker, creature, defender)))
                        && !barred.contains(&(creature, defender)) {
                        barred.push((creature, defender));
                    }
                }
            }
        }
    }
    // CR 508.1c, 611.2c: the attacks a resolved rule about an open set forbids - "until your next turn, creatures can't
    // attack you" (Chronomantic Escape), barring the attack on that player alone.
    for attack in crate::static_rules::lasting_rules_barring_attacks(&*cx, cx.cards(), &eligible, &targets) {
        if !barred.contains(&attack) {
            barred.push(attack);
        }
    }
    let mut eligible = eligible;
    if !barred.is_empty() {
        eligible.retain(|&id| targets.is_empty() || targets.iter().any(|&at| !barred.contains(&(id, at))));
        barred.retain(|x| eligible.contains(&x.0));
        if eligible.is_empty() {
            return Ok(());
        }
    }
    // CR 508.1d, 508.1h: what each attack costs, where a static ability makes attacking cost something.
    let costs = crate::static_rules::attack_costs(cx, &eligible, &targets);
    // CR 508.1d: the requirements, one for each ability, effect and player they come from, and the most of them a
    // declaration can obey without paying a cost to attack - a cost is never a requirement.
    let requirements = attack_rules(cx, &eligible, &targets, barred, &costs);
    let can_pay = if costs.is_empty() { 0 } else { actions::available(cx, active).most_unrestricted() };
    let kind = RequestKind::DeclareAttackers { eligible, players, planeswalkers, battles, requirements, costs, can_pay };
    let attacks = loop {
        // A declaration whose costs are not paid is taken back (CR 508.1, 733): the game returns to the moment before it,
        // the mana abilities activated while paying undone but those that may not be (`takeback`), and the attackers are
        // declared again.
        let mark = cx.s().mark();
        let log_mark = cx.g.log.len();
        let since = crate::takeback::since(cx.g);
        // An active player who has left declares nothing, and the turn goes on without them (CR 800.4j).
        let Some(Answer::Attack(attacks)) = cx.ask_rule(active, kind.clone())? else { return Ok(()) };
        // CR 508.1e: the bands among them, announced before they tap.
        let bands = declare_bands(cx, active, &attacks)?;
        // CR 508.1f: the chosen creatures become tapped, which is no cost, before any cost is paid (CR 508.1g-j).
        // They become tapped at once: "whenever one or more creatures you control become tapped" triggers once (CR 603.2c).
        let tapping: Vec<ObjectId> = attacks.iter().map(|&(id, _)| id).filter(|&id| !cx.chars(id).has(Keyword::Vigilance)).collect();
        ops::tap_together(cx, &tapping);
        // CR 508.1g: the optional costs to attack the active player chooses to pay.
        let options = choose_attack_options(cx, active, &attacks)?;
        let RequestKind::DeclareAttackers { costs, .. } = &kind else { unreachable!() };
        // CR 508.1h: the total cost to attack, locked in now; mana abilities activated while paying it (CR 508.1i) change
        // nothing of it. CR 508.1j: it is paid all at once.
        let total = AttackCost::total(costs, &attacks);
        if !total.is_zero() {
            let paying: Vec<ObjectId> = attacks.iter().filter(|&&(a, t)| !AttackCost::free(costs, a, t)).map(|a| a.0).collect();
            let source = costs
                .iter()
                .find(|c| attacks.contains(&(c.attacker, c.target)))
                .map(|c| c.source)
                .expect("a cost to attack has a source");
            let helpers = actions::Helpers { attackers: &paying, ..actions::Helpers::NONE };
            match actions::pay_mana_with(cx, active, source, total, &helpers) {
                Ok(_) => {}
                Err(Stop::Cancel) => {
                    crate::takeback::take_back(cx, mark, log_mark, since);
                    continue;
                }
                Err(stop) => return Err(stop),
            }
        }
        // CR 508.1j: the optional costs are paid with the rest. A creature to be tapped that a mana ability tapped, or that is
        // gone, can't be: the declaration is illegal and taken back (CR 508.1, 733.1).
        if !options.iter().all(|o| o.tapped == ObjectId::NONE || enlistable_now(cx, active, o.tapped)) {
            crate::takeback::take_back(cx, mark, log_mark, since);
            continue;
        }
        for o in &options {
            match o.option {
                AttackOption::Enlist => ops::tap(cx, o.tapped),
                AttackOption::Exert => ops::exert(cx, o.attacker, active),
            }
        }
        break (attacks, bands, options);
    };
    let (attacks, bands, options) = attacks;
    let mut attackers = Vec::new();
    for ((id, target), band) in attacks.into_iter().zip(bands) {
        // Each names a player or a planeswalker or battle of a defending player (CR 508.1b), which the declaration was checked
        // against: its defending player is the one that names (CR 508.5).
        let Some(defending) = defending_player(cx, target) else { continue };
        let attacked_as = target.object().map_or(TypeSet::NONE, |d| ops::attackable_types(&cx.chars(d)));
        attackers.push(Attacker { band, ..Attacker::new(id, target, defending, attacked_as) });
    }
    let attacked = !attackers.is_empty();
    cx.st().update_turn(|t| t.attacked = attacked);
    cx.st().set_attackers(attackers.clone());
    // One batch (CR 603.2c): "whenever one or more creatures attack" triggers once; "whenever you attack" after the
    // creatures' own abilities (CR 508.1m).
    // Each keeps the attacker as it was declared (`history::Attacked::chars`): "during any turn you attacked with a
    // Rogue" asks what it was then, not what it has become or who has taken it since.
    let mut events: Vec<Event> = Vec::with_capacity(attackers.len() + options.len() + 1);
    for a in &attackers {
        let chars = cx.chars(a.id);
        let known = ops::last_known_from(cx, a.id, &chars);
        let known = cx.st().last_known_push(known);
        events.push(Event::Attacks { attacker: a.id, defender: a.target, known, defending: a.defending });
    }
    // The optional costs paid (CR 508.1g, 508.1j): what their linked "when you do" abilities trigger on (CR 607.2h), in the
    // same batch, so they go on the stack with the attack triggers, before blockers are declared.
    events.extend(options.iter().map(|o| Event::PaidToAttack { attacker: o.attacker, option: o.option, ability: o.ability, instance: o.instance, tapped: o.tapped }));
    if attacked {
        events.push(Event::AttackersDeclared { player: active, count: count_u32(attackers.len()) });
    }
    cx.emit_batch(&events, None);
    Ok(())
}

/// "You may exert this creature as it attacks" (CR 701.43d), asked of the attacking player for each instance an attacker has.
const EXERT_IT: Question = Question::new(
    "Exert",
    "exert it as it attacks?",
    "An exerted creature won't untap during your next untap step",
    "Exert it",
    "Don't exert it",
)
.no_default();

/// One optional cost to attack chosen to be paid (CR 508.1g): the attacker, which instance of which of its abilities it is
/// (`Event::PaidToAttack`), and the creature it taps, if any.
#[derive(Clone, Copy)]
struct AttackOptionPaid {
    attacker: ObjectId,
    option: AttackOption,
    ability: AbilityRef,
    instance: u8,
    tapped: ObjectId,
}

/// The optional costs to attack (CR 508.1g) the active player chooses to pay for the creatures they chose to attack with,
/// once those are tapped (CR 508.1f) and before the total cost is locked in (CR 508.1h): each instance of each attacker's
/// (`TriggeredDef::attack_option`), in the order the attacks were declared and the attacker has them, asked one at a time.
/// Enlist (CR 702.154a) is asked as a choice of up to one creature among those it may tap - none being the cost not paid -
/// and not when there is none; a creature chosen for one enlist ability is not offered for another (the rulings). Exert
/// (CR 701.43d) is a yes or no. Asked of the attackers as they are now: an attacker that lost its abilities is offered none.
fn choose_attack_options(cx: &mut Cx, active: PlayerId, attacks: &[(ObjectId, Target)]) -> R<Vec<AttackOptionPaid>> {
    let mut paid: Vec<AttackOptionPaid> = Vec::new();
    for &(attacker, _) in attacks {
        let chars = cx.chars(attacker);
        let offered: Vec<(AbilityRef, u8, AttackOption)> = chars
            .abilities()
            .enumerate()
            .filter_map(|(nth, a)| match cx.cards().ability(a) {
                AbilityDef::Triggered(t) => t.attack_option.filter(|_| {
                    t.attack_condition.is_none_or(|condition| {
                        condition(cx, &Asker { instance: None, source: attacker, controller: active, by: cx.link_nth(attacker, &chars, nth), spell: false })
                    })
                }).map(|o| (a, chars.instance(nth), o)),
                _ => None,
            })
            .collect();
        for (ability, instance, option) in offered {
            let tapped = match option {
                AttackOption::Exert => {
                    let ask = RequestKind::May { source: attacker, purpose: Purpose::Ask(&EXERT_IT) };
                    if cx.ask_rule(active, ask)? != Some(Answer::Yes) {
                        continue;
                    }
                    ObjectId::NONE
                }
                AttackOption::Enlist => {
                    let cards: Vec<ObjectId> = cx
                        .s()
                        .battlefield()
                        .iter()
                        .copied()
                        .filter(|&id| !attacks.iter().any(|a| a.0 == id) && !paid.iter().any(|p| p.tapped == id) && enlistable_now(cx, active, id))
                        .collect();
                    if cards.is_empty() {
                        continue;
                    }
                    let ask = RequestKind::ChooseCards { source: attacker, purpose: Purpose::Enlist, cards, rule: Box::new(Selection::new(Count::UpTo(1)).called("creature")) };
                    match cx.ask_rule(active, ask)? {
                        Some(Answer::Cards(chosen)) if chosen.len() == 1 => chosen[0],
                        _ => continue,
                    }
                }
            };
            paid.push(AttackOptionPaid { attacker, option, ability, instance, tapped });
        }
    }
    Ok(paid)
}

/// Whether `id` may be tapped for an enlist ability of a creature `player` attacks with (CR 702.154a), past not attacking and
/// not being tapped for another: an untapped creature `player` controls that has haste or has been under their control
/// continuously since the turn began.
fn enlistable_now(cx: &Cx, player: PlayerId, id: ObjectId) -> bool {
    let obj = cx.s().obj(id);
    obj.zone == Zone::BATTLEFIELD
        && obj.controller == player
        && !obj.tapped
        && cx.chars(id).is(CardType::Creature)
        && !summoning_sick(cx.cards(), cx.s(), &cx.g.log, id)
}

/// The attacking bands the active player announces (CR 508.1e, 702.22c), one band number for each of `attacks` (0 for none).
/// Each attacker with banding not yet in a band is asked in turn for the other attackers that band with it: any of those
/// attacking what it attacks (CR 702.22d) and in no band yet (each creature is in one band at most), any number with banding
/// and at most one without (the core refuses more, `validate`). Choosing none forms no band with it. Nothing is asked where no
/// attacker has banding, or one has no other attacker beside it.
fn declare_bands(cx: &mut Cx, active: PlayerId, attacks: &[(ObjectId, Target)]) -> R<Vec<u32>> {
    let mut band = vec![0u32; attacks.len()];
    let mut next = 1;
    for i in 0..attacks.len() {
        let (source, target) = attacks[i];
        if band[i] != 0 || !cx.chars(source).has(Keyword::Banding) {
            continue;
        }
        let eligible: Vec<ObjectId> =
            (0..attacks.len()).filter(|&j| j != i && band[j] == 0 && attacks[j].1 == target).map(|j| attacks[j].0).collect();
        if eligible.is_empty() {
            continue;
        }
        let count = u32::try_from(eligible.len()).unwrap_or(u32::MAX);
        let ask = RequestKind::ChooseCards { source, purpose: Purpose::Band, cards: eligible, rule: Box::new(Selection::new(Count::UpTo(count)).called("attacking creature").with(BAND)) };
        let Some(Answer::Cards(chosen)) = cx.ask_rule(active, ask)? else { continue };
        if chosen.is_empty() {
            continue;
        }
        band[i] = next;
        for c in chosen {
            if let Some(j) = attacks.iter().position(|a| a.0 == c) {
                band[j] = next;
            }
        }
        next = next.saturating_add(1);
    }
    Ok(band)
}

/// "Any number of creatures with banding and up to one without" (CR 702.22c): the creatures that join a band.
fn up_to_one_without_banding(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let banding = |c: ObjectId| q.chars(c).has(Keyword::Banding);
    let without = choice.selected.iter().filter(|&&c| !banding(c)).count();
    Verdict { complete: without <= 1, addable: Bits::of(choice.candidates.len(), |i| without == 0 || banding(choice.candidates[i])) }
}

/// The rule of a band's other members (CR 702.22c), a part of the question that asks for them.
const BAND: Part = cantrip_api::select::part(&(up_to_one_without_banding as SelectFn), 0, "");

/// Whether `blocker` may block `attacker` (CR 509.1b): a creature that can't block (decayed's too, CR 702.147a), one that
/// can't be blocked (an ability, or a resolved effect: `EffectKind::CantBeBlocked`), a resolved restriction on the pair
/// (`EffectKind::BlockRestriction`), one that can block only creatures with flying, flying (CR 702.9b), shadow (CR 702.28b),
/// horsemanship (CR 702.31b), fear (CR 702.36b), intimidate (CR 702.13b), skulk (CR 702.118b), landwalk (CR 702.14c),
/// protection (CR 702.16f). Menace (CR 702.111b) is about the
/// whole declaration, not a pair.
fn can_block(cx: &Cx, blocker: ObjectId, attacker: ObjectId, rule_sources: &[ObjectId]) -> bool {
    let a = cx.chars(attacker);
    let b = cx.chars(blocker);
    if a.has(Keyword::CantBeBlocked) || b.has(Keyword::CantBlock) || b.has(Keyword::Decayed) || (b.has(Keyword::CanBlockOnlyCreaturesWithFlying) && !a.has(Keyword::Flying)) {
        return false;
    }
    // "It can't be blocked this turn" from a resolved spell or ability: an effect on the attacker, not an ability, so it holds
    // whatever abilities the attacker has lost since (CR 113.1a, 611.2c, 613.1f).
    if cx.s().effects.iter().any(|e| e.affected == attacker && e.kind == EffectKind::CantBeBlocked) {
        return false;
    }
    // A resolved restriction on who may block whom ("it can't be blocked this turn except by creatures with haste"): an
    // effect too, so it holds whatever abilities its source or the creature have lost since (CR 113.1a, 611.2c, 613.1f).
    for e in &cx.s().effects {
        if let EffectKind::BlockRestriction { card, index } = e.kind
            && cx.card(card).lasting_block_rules.get(index as usize).is_some_and(|r| !(r.allows)(cx, e.affected, blocker, attacker))
        {
            return false;
        }
    }
    if a.has(Keyword::Flying) && !b.has(Keyword::Flying) && !b.has(Keyword::Reach) {
        return false;
    }
    if a.has(Keyword::Shadow) != b.has(Keyword::Shadow) {
        return false;
    }
    if a.has(Keyword::Horsemanship) && !b.has(Keyword::Horsemanship) {
        return false;
    }
    if a.has(Keyword::Fear) && !b.is(CardType::Artifact) && !b.colors.has(Color::Black) {
        return false;
    }
    if a.has(Keyword::Intimidate) && !b.is(CardType::Artifact) && !b.colors.intersects(a.colors) {
        return false;
    }
    // Skulk (CR 702.118b): power as the layers have it now.
    if a.has(Keyword::Skulk) && b.power.unwrap_or(0) > a.power.unwrap_or(0) {
        return false;
    }
    // "Can't be blocked by creatures with power 2 or less" (CR 509.1b): a creature with no power can't block it either.
    if a.has(Keyword::CantBeBlockedByPowerTwoOrLess) && b.power.is_none_or(|p| p <= 2) {
        return false;
    }
    // CR 702.14c: only a defending player's current lands qualify. Raw membership is exact: snow islandwalk
    // does not confer ordinary islandwalk and cannot evade blockers through a non-snow Island.
    if a.keywords.has_landwalk() {
        // CR 609.4: "as though it didn't have [landwalk]" changes only blocking. The attacker retains the ability,
        // including for other abilities that ask about it; flying, protection and other landwalks still restrict it.
        let mut walks = a.keywords;
        for &source in rule_sources {
            for (card, by) in cx.cards().texts_linked(cx.s(), source) {
                let asker = Asker { instance: None, source, controller: cx.s().obj(source).controller, by, spell: false };
                for exception in cx.card(card).landwalk_exceptions {
                    walks.0 &= !(exception(cx, &asker, blocker, attacker)).for_effect().0;
                }
            }
        }
        let defending = cx.s().obj(blocker).controller;
        let walked = cx.s().battlefield().iter().any(|&id| {
            if cx.s().obj(id).controller != defending { return false; }
            let land = cx.chars(id);
            land.is(CardType::Land) && (
                Keyword::LANDWALKS.iter().any(|&(walk, ty)| walks.has(walk) && land.has_subtype(ty))
                || (land.supertypes.snow && Keyword::SNOW_LANDWALKS.iter().any(|&(walk, ty)| walks.has(walk) && land.has_subtype(ty)))
                || (walks.has(Keyword::SnowLandwalk) && land.supertypes.snow)
                || (walks.has(Keyword::Desertwalk) && land.has_subtype(Subtype::Desert))
                || (walks.has(Keyword::NonbasicLandwalk) && !land.supertypes.basic)
                || (walks.has(Keyword::LegendaryLandwalk) && land.supertypes.legendary)
            )
        });
        if walked { return false; }
    }
    // CR 509.1b: a static ability that limits who may block whom, of the blocker, of the attacker, or of another permanent
    // ("creatures you control with power 2 or less can't be blocked by ...", Delney, Streetwise Lookout).
    for &id in rule_sources {
        // Of every definition it has, a merged permanent's other components' among them (CR 702.140e).
        if !cx.cards().texts(cx.s(), cx.s().obj(id)).all(|card| cx.card(card).block_rules.iter().all(|rule| rule(cx, id, blocker, attacker))) {
            return false;
        }
    }
    !a.protected_from_by(&b, || cantrip_api::mana_value_of(&|c| cx.card(c), cx.s(), blocker, &b), cx.s().obj(blocker).controller)
}

/// The most creatures that may block `attacker` (CR 509.1b): its own "can't be blocked by more than one creature"
/// (`Chars::maximum_blockers`), and each limit a static ability of a permanent among `rule_sources` puts on it
/// (`CardDef::blocker_limits`: Meltstrider's Resolve's on the creature it enchants, which is no ability of the creature and
/// so outlasts its losing them, CR 113.1a, 613.1f). The restrictions are cumulative: the smallest holds.
fn maximum_blockers(cx: &Cx, attacker: ObjectId, rule_sources: &[ObjectId]) -> usize {
    let mut most = cx.chars(attacker).maximum_blockers();
    for &id in rule_sources {
        let limits: Vec<_> = cx.cards().texts(cx.s(), cx.s().obj(id)).flat_map(|card| cx.card(card).blocker_limits.iter().copied()).collect();
        for limit in limits {
            if let Some(n) = limit(cx, id, attacker) {
                most = most.min(n);
            }
        }
    }
    most
}

/// How many attackers `blocker` can block (CR 509.1a): one, and what each resolved "can block any number of creatures this
/// turn" or "an additional creature" on it (`EffectKind::BlockCapacity`) and each static ability of a permanent among
/// `rule_sources` (`CardDef::block_capacities`, its own among them) adds (`BlockCapacity::total`); `u32::MAX` for any number.
fn block_capacity(cx: &Cx, blocker: ObjectId, rule_sources: &[ObjectId]) -> u32 {
    let resolved = cx.s().effects.iter().filter(|e| e.affected == blocker).filter_map(|e| match e.kind {
        EffectKind::BlockCapacity(c) => Some(c),
        _ => None,
    });
    let statics = rule_sources.iter().flat_map(|&id| {
        cx.cards()
            .texts(cx.s(), cx.s().obj(id))
            .flat_map(move |card| cx.card(card).block_capacities.iter().filter_map(move |rule| rule(cx, id, blocker)))
    });
    let chars = cx.chars(blocker);
    let abilities = chars.abilities().filter_map(|ability| {
        if let AbilityDef::BlockAllowance(def) = cx.cards().ability(ability) {
            let asker = Asker::new(blocker, cx.controller(blocker));
            Some(BlockCapacity::Additional(def.additional_by.map_or(def.additional, |amount| amount(cx, &asker))))
        } else { None }
    });
    let generated = rule_sources.iter().flat_map(|&source| {
        cx.cards().texts(cx.s(), cx.s().obj(source)).flat_map(move |card| {
            cx.card(card).additional_blocks.iter().map(move |rule| BlockCapacity::Additional(rule(cx, source, blocker)))
        })
    });
    let lasting = cx.s().effects.iter().filter_map(|effect| {
        let EffectKind::LastingRule { card, index, about } = effect.kind else { return None };
        let rule = cx.card(card).lasting_rules.get(index as usize)?;
        let LastingRuleKind::AdditionalBlocks(amount) = rule.rule else { return None };
        (rule.applies)(cx, &Asker::new(effect.source, effect.controller), about, blocker).then_some(BlockCapacity::Additional(amount))
    });
    BlockCapacity::total(resolved.chain(statics).chain(abilities).chain(generated).chain(lasting))
}

/// The requirements on a declaration of attackers by `eligible` among `targets` (CR 508.1d), one for each source: each
/// instance of "attacks each combat if able" it has (printed or gained, `Chars::must_attack`); one for each "attacks if able"
/// rule about a set that takes it in, and two for each that also says "attacks a player other than you if able"
/// (`LastingRuleKind`); for each player who goaded it,
/// two (CR 701.15b-c: attack, and attack a player other than that one), goad being an effect and no ability, which a creature
/// that loses its abilities is still under; and each "attacks [that player or permanent] if able" in force now
/// (`EffectKind::MustAttackAt`, `in_force`).
/// With the attacks a static ability forbids, `barred`, and the cap on how many may attack each of `targets` a static
/// ability sets (CR 508.1c); with the most of them any declaration obeys attacking for free, and one that does
/// (`AttackRules::solve`).
fn attack_rules(cx: &Cx, eligible: &[ObjectId], targets: &[Target], barred: Vec<(ObjectId, Target)>, costs: &[AttackCost]) -> AttackRules {
    let mut required = Vec::new();
    // The static abilities that goad, found once for every creature.
    let goad_sources = crate::static_rules::goad_sources(cx, cx.cards());
    for &id in eligible {
        for _ in 0..cx.chars(id).must_attack {
            required.push(RequiredAttack { attacker: id, aim: AttackAim::Anything });
        }
        // "Each creature attacks this turn if able" (Goblin Diplomats): one for each resolved rule about an open set that takes
        // it in now (CR 611.2c), an effect and no ability, so one that has lost its abilities attacks too.
        for _ in crate::static_rules::lasting_rules_on(cx, cx.cards(), id, LastingRuleKind::MustAttack) {
            required.push(RequiredAttack { attacker: id, aim: AttackAim::Anything });
        }
        // "Until your next turn, each creature attacks each combat if able and attacks a player other than you if able"
        // (Maximum Carnage): goad's two requirements, from a rule of the game and no designation (the rulings).
        for e in crate::static_rules::lasting_rules_on(cx, cx.cards(), id, LastingRuleKind::MustAttackPlayerOtherThanYou) {
            required.push(RequiredAttack { attacker: id, aim: AttackAim::Anything });
            required.push(RequiredAttack { attacker: id, aim: AttackAim::PlayerOtherThan(e.controller) });
        }
        // Goad (CR 701.15a-d), from a resolved effect and from a static ability that says the creature is goaded alike; the
        // same player goading it again makes no further requirement (CR 701.15d).
        for goader in crate::static_rules::goaders_among(cx, &goad_sources, id) {
            required.push(RequiredAttack { attacker: id, aim: AttackAim::Anything });
            required.push(RequiredAttack { attacker: id, aim: AttackAim::PlayerOtherThan(goader) });
        }
        for e in cx.s().effects.iter().filter(|e| e.affected == id) {
            match e.kind {
                EffectKind::MustAttackAt(at) if in_force(cx, e) => required.push(RequiredAttack { attacker: id, aim: AttackAim::At(at) }),
                _ => {}
            }
        }
    }
    let alone = crate::static_rules::attacks_alone_barred(cx, cx.cards(), eligible);
    let mut rules = AttackRules { required, barred, alone_barred: alone, limits: crate::static_rules::attack_limits(cx, targets), most: 0, suggested: Vec::new() };
    if !rules.required.is_empty() {
        (rules.most, rules.suggested) = rules.solve(eligible, targets, costs);
    }
    rules
}

/// Whether a resolved effect on a declaration of attackers applies now, as its duration has it (CR 611.2a). One "during its
/// controller's next turn" (`Duration::DuringControllersNextTurn`) does only during the first turn its affected creature's
/// controller - whoever controls it now - has begun since both the turn the effect began in and the turn they gained the
/// creature (`controlled_since`): a creature that changes controllers before its chance to attack is under it during its new
/// controller's next turn (Gideon, Battle-Forged's ruling), though a turn of theirs came before they had it. This is that
/// player's turn, after both, and no turn of theirs began in between, as the history's `TurnBegan` records have it (extra
/// turns are turns; a skipped one never began, CR 500.7, 614.10). Every other duration applies for as long as the effect lasts.
fn in_force(cx: &Cx, e: &ContinuousEffect) -> bool {
    let Duration::DuringControllersNextTurn(began) = e.duration else { return true };
    let s = cx.s();
    let creature = s.obj(e.affected);
    let controller = creature.controller;
    let since = began.max(creature.controlled_since);
    let now = s.turn.number;
    if s.turn.active != controller || now <= since {
        return false;
    }
    let history = History::with_receipts(&cx.g.log, now, &cx.s().cost_receipts);
    !history
        .all()
        .iter()
        .rev()
        .skip_while(|r| r.turn >= now)
        .take_while(|r| r.turn > since)
        .any(|r| matches!(r.event, Event::TurnBegan { player, .. } if player == controller))
}

/// "During its controller's next turn" is this turn for each such effect in force now (`in_force`): from here it lasts until
/// end of turn, whoever controls its creature later this turn (CR 611.2a). A change of control moves it to the new
/// controller's next turn only before the creature has had its chance to attack (Gideon, Battle-Forged's ruling): asked as
/// attackers are declared, and at the cleanup step for a turn with no declaration (no combat, no creature able to attack).
fn spend_next_turn_effects(cx: &mut Cx) {
    let spent: Vec<usize> = (0..cx.s().effects.len())
        .filter(|&i| {
            let e = &cx.s().effects[i];
            matches!(e.duration, Duration::DuringControllersNextTurn(_)) && in_force(cx, e)
        })
        .collect();
    for i in spent {
        let effect = ContinuousEffect { duration: Duration::EndOfTurn, ..cx.s().effects[i] };
        cx.st().effect_set(i, effect);
    }
}

/// "Can't block" (CR 509.1b): from an effect on this creature, or a resolved rule about an open set that takes it in now
/// (`LastingRuleKind::CantBlock`: Falter, Aragorn, King of Gondor, Headliner Scarlett).
fn cant_block(cx: &Cx, id: ObjectId) -> bool {
    cx.s().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::CantBlock)
        || crate::static_rules::lasting_rules_on(cx, cx.cards(), id, LastingRuleKind::CantBlock).next().is_some()
}

/// Live sources of static blocking rules, shared by declaration legality and its rule-specific tests.
fn blocking_rule_sources(cx: &Cx) -> Vec<ObjectId> {
    cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let obj = cx.s().obj(id);
            cx.cards().texts(cx.s(), obj).any(|card| cx.cards().block_rules[card.0 as usize]) && !cx.lost_all_abilities(id)
        })
        .collect()
}

fn declare_blockers(cx: &mut Cx) -> R<()> {
    let attackers: Vec<ObjectId> = cx.s().attackers.iter().map(|a| a.id).collect();
    if attackers.is_empty() {
        return Ok(());
    }
    // The defending player of the first attacker (CR 508.5): one attacking a planeswalker or battle removed from combat
    // still has one, and may still be blocked (CR 506.4c).
    let defender = cx.s().attackers[0].defending;
    let eligible: Vec<ObjectId> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| cx.s().obj(id).controller == defender && cx.chars(id).is(CardType::Creature) && !cx.s().obj(id).tapped)
        .filter(|&id| !cant_block(cx, id))
        .collect();
    // The permanents whose static abilities limit who may block whom, or by how many (CR 509.1b), and which have not lost
    // them (CR 613.1f).
    let rule_sources = blocking_rule_sources(cx);
    let mut allowed = Vec::new();
    for &b in &eligible {
        for &a in &attackers {
            if can_block(cx, b, a, &rule_sources) {
                allowed.push((b, a));
            }
        }
    }
    // Menace (CR 702.111b) and "except by three or more creatures" (CR 509.1b): an attacker that can't be blocked by
    // fewer than so many creatures is offered only if that many could, and not at all if it can't be blocked by that many
    // either ("can't be blocked by more than one creature" as well, CR 509.1b: the restrictions are cumulative).
    let menace: Vec<ObjectId> = attackers
        .iter()
        .copied()
        .filter(|&a| {
            let fewest = cx.chars(a).minimum_blockers();
            allowed.iter().filter(|p| p.1 == a).count() < fewest || fewest > maximum_blockers(cx, a, &rule_sources)
        })
        .collect();
    allowed.retain(|p| !menace.contains(&p.1));
    if !allowed.is_empty() {
        // CR 509.1b-c: each attacker's limits, and the blocks effects require if able, with the most of them a declaration can obey.
        let requirements = block_rules(cx, &attackers, &allowed, &rule_sources);
        // A defending player who has left blocks with nothing (CR 800.4a): no one else declares their blocks.
        let answer = cx.ask_rule(defender, RequestKind::DeclareBlockers { eligible, attackers, allowed, requirements })?;
        if let Some(Answer::Block(blocks)) = answer {
            // CR 509.1g: the blockers are declared together, a "blocks" event for each attacker each blocks; only its first
            // declared block is marked as declared - not the others of a band it blocks through banding, nor the other attackers
            // a creature that can block several was declared to block - so "whenever this creature blocks" triggers once
            // (CR 509.3a) and "whenever it blocks a creature" once for each (CR 509.3b).
            let mut events: Vec<Event> = Vec::with_capacity(blocks.len());
            let mut list: Vec<Block> = Vec::new();
            let mut attackers = cx.s().attackers.clone();
            for (blocker, attacker) in blocks {
                // CR 702.22h: a creature blocking a member of an attacking band blocks each other member of it too, a block of
                // each (CR 509.3b, 509.3d), including any it blocks automatically through banding.
                let band = attackers.iter().find(|a| a.id == attacker).map_or(0, |a| a.band);
                let mut blocked = vec![attacker];
                blocked.extend(attackers.iter().filter(|a| band != 0 && a.band == band && a.id != attacker).map(|a| a.id));
                for attacker in blocked {
                    // A blocker declared against two members of one band still blocks each member only once. Its
                    // unqualified "blocks" trigger is once for the whole declaration, not once per attacked creature.
                    if list.iter().any(|b: &Block| b.blocker == blocker && b.attacker == attacker) { continue; }
                    let first = !list.iter().any(|b: &Block| b.blocker == blocker);
                    events.push(Event::Blocks { blocker, attacker, declared: first });
                    list.push(Block { blocker, attacker, first_strike: false });
                    if let Some(a) = attackers.iter_mut().find(|a| a.id == attacker) {
                        if !a.blocked {
                            events.push(Event::BecameBlocked { attacker });
                        }
                        a.blocked = true;
                    }
                }
            }
            cx.st().set_attackers(attackers);
            cx.st().set_blocks(list);
            if !events.is_empty() {
                cx.emit_batch(&events, None);
            }
        }
    }
    // CR 509.1h: the attackers nothing blocked are unblocked, announced together.
    let unblocked: Vec<Event> = cx
        .s()
        .attackers
        .iter()
        .filter(|a| !a.blocked)
        .map(|a| Event::Unblocked { attacker: a.id, defender: a.target })
        .collect();
    if !unblocked.is_empty() {
        cx.emit_batch(&unblocked, None);
    }
    Ok(())
}

/// Whether a creature has first strike or double strike now (CR 702.7b, 702.4b).
fn strikes_first(cx: &Cx, id: ObjectId) -> bool {
    let c = cx.chars(id);
    c.has(Keyword::FirstStrike) || c.has(Keyword::DoubleStrike)
}

/// Whether a creature assigns combat damage in this combat damage step (CR 510.4): in a first-strike step only those with
/// first strike or double strike; in the regular step after one, those that had neither as the first-strike step began
/// (`had_first_strike`, recorded then), and those that have double strike now - so a creature that loses first strike
/// after dealing first-strike damage deals no more, and one that gains it only after that step began deals its damage now.
/// With no first-strike step, every creature does.
fn deals_now(cx: &Cx, id: ObjectId, had_first_strike: bool, first_strike_step: bool) -> bool {
    if first_strike_step { strikes_first(cx, id) } else { !had_first_strike || cx.chars(id).has(Keyword::DoubleStrike) }
}

/// The rules of a declaration of blockers among `allowed` (CR 509.1b-c): each attacker's fewest and most blockers; the
/// requirements - those of static abilities (Lure's, `static_rules::block_requirements`) and of each creature an effect
/// says blocks if able (`EffectKind::MustBlock`) that may block something, or blocks one attacker if able (`EffectKind::MustBlockAttacker`) that may block it; and the most of them a declaration can obey, with
/// one that does (`BlockRequirements::solve`). A creature that can't block, or may block nothing, is required nothing (CR
/// 509.1c: a requirement is obeyed only as far as the restrictions allow).
fn block_rules(cx: &Cx, attackers: &[ObjectId], allowed: &[(ObjectId, ObjectId)], rule_sources: &[ObjectId]) -> BlockRequirements {
    let limits = attackers
        .iter()
        .map(|&a| (a, count_u32(cx.chars(a).minimum_blockers()), u32::try_from(maximum_blockers(cx, a, rule_sources)).unwrap_or(u32::MAX)))
        .collect();
    let mut required = crate::static_rules::block_requirements(cx, allowed);
    // One requirement for each effect (CR 509.1c counts requirements): two "blocks this turn if able" on a creature are two.
    for e in &cx.s().effects {
        if e.kind == EffectKind::MustBlock && allowed.iter().any(|p| p.0 == e.affected) {
            required.push(RequiredBlock { blocker: Some(e.affected), attacker: None });
        }
        // Provoke's "blocks this creature if able": the one block, where the restrictions allow it (CR 509.1c).
        if let EffectKind::MustBlockAttacker(attacker) = e.kind
            && allowed.contains(&(e.affected, attacker))
        {
            required.push(RequiredBlock { blocker: Some(e.affected), attacker: Some(attacker) });
        }
        // "Blocks each attacking creature if able": one requirement for each attacker it may block (CR 509.1c).
        if e.kind == EffectKind::MustBlockEach {
            required.extend(allowed.iter().filter(|p| p.0 == e.affected).map(|&(b, a)| RequiredBlock { blocker: Some(b), attacker: Some(a) }));
        }
    }
    // The creatures that can block more than one attacker (CR 509.1a): none without a static ability that limits or widens
    // blocking or a resolved effect that widens it.
    let mut capacities: Vec<(ObjectId, u32)> = Vec::new();
    {
        for &(b, _) in allowed {
            if !capacities.iter().any(|c| c.0 == b) {
                capacities.push((b, block_capacity(cx, b, rule_sources)));
            }
        }
        capacities.retain(|c| c.1 != 1);
    }
    // A creature blocking one member of an attacking band blocks every member (CR 702.22h), so blocking any member obeys a
    // requirement to block another, and one block may obey two.
    let bands = cx.s().attackers.iter().filter(|a| a.band != 0).map(|a| (a.id, a.band)).collect();
    // No block costs anything to declare yet (CR 509.1d): no effect that makes one is built.
    let mut rules = BlockRequirements { required, most: 0, suggested: Vec::new(), bands, limits, costs: Vec::new(), capacities };
    if !rules.required.is_empty()
        && let Some((most, suggested)) = rules.solve(allowed, &[], None, true)
    {
        rules.most = most;
        if most > 0 {
            rules.suggested = suggested;
        }
    }
    rules
}

/// CR 510.1a, 609.4: an earlier resolving effect can say the creature assigns no combat damage. It stays in combat,
/// remains a receiver of assigned damage, and still makes a first-strike step occur when appropriate (CR 510.4).
fn assigns_combat_damage(cx: &Cx, creature: ObjectId) -> bool {
    !cx.s().effects.iter().any(|e| e.affected == creature && e.kind == EffectKind::NoCombatDamageAssignment)
}

/// Combat damage (CR 510): every attacker and blocker deals its damage at once. A creature with a choice in where its damage
/// goes - an attacker blocked by two or more creatures, or a trampler with more than lethal for its blocker (CR 510.1c,
/// 702.19b), a blocker blocking two or more creatures (CR 510.1d) - divides it as the player banding names chooses, else as
/// its controller does (CR 702.22j-k); the active player announces first, then each defending player (CR 510.1).
fn combat_damage(cx: &mut Cx) -> R<()> {
    // CR 510.4: with a first or double striker in combat as the step begins, there is a first-strike damage step before
    // the regular one.
    let in_combat: Vec<ObjectId> =
        cx.s().attackers.iter().map(|a| a.id).chain(cx.s().blockers.iter().copied()).collect();
    let round = cx.s().turn.damage_round;
    let first_strike_step = round == 0 && in_combat.iter().any(|&id| live(cx.s(), id) && strikes_first(cx, id));
    cx.st().update_turn(|t| t.damage_round = if first_strike_step { 1 } else { 2 });
    // CR 510.4: which attackers and blockers had first strike or double strike as the first-strike step began, which the
    // regular step after it asks.
    if first_strike_step {
        let mut attackers = cx.s().attackers.clone();
        for a in &mut attackers {
            a.first_strike = live(cx.s(), a.id) && strikes_first(cx, a.id);
        }
        let mut blocks = cx.s().blocks.clone();
        for b in &mut blocks {
            b.first_strike = live(cx.s(), b.blocker) && strikes_first(cx, b.blocker);
        }
        cx.st().set_attackers(attackers);
        cx.st().set_blocks(blocks);
    }

    // The creatures dealing combat damage now: the attackers, and each blocker once however many creatures it blocks.
    let attackers: Vec<Attacker> = cx
        .s()
        .attackers
        .iter()
        .copied()
        .filter(|a| live(cx.s(), a.id) && deals_now(cx, a.id, a.first_strike, first_strike_step) && assigns_combat_damage(cx, a.id))
        .collect();
    let mut blockers: Vec<ObjectId> = Vec::new();
    for b in &cx.s().blocks {
        if !blockers.contains(&b.blocker) && live(cx.s(), b.blocker) && deals_now(cx, b.blocker, b.first_strike, first_strike_step)
            && assigns_combat_damage(cx, b.blocker) {
            blockers.push(b.blocker);
        }
    }

    // The divisions, each with the player who makes it.
    let mut divisions: Vec<(PlayerId, DamageDivision)> = Vec::new();
    for attacker in attackers.iter().filter(|a| a.blocked) {
        if let Some(d) = attacker_division(cx, attacker) {
            let by = if d.banding { cx.s().obj(d.creatures[0]).controller } else { cx.s().obj(attacker.id).controller };
            divisions.push((by, d));
        }
    }
    for &blocker in &blockers {
        if let Some(d) = blocker_division(cx, blocker) {
            let by = if d.banding { cx.s().obj(d.creatures[0]).controller } else { cx.s().obj(blocker).controller };
            divisions.push((by, d));
        }
    }
    let mut chosen: Vec<(ObjectId, Option<ObjectId>, u32)> = Vec::new();
    let players: Vec<PlayerId> = cx.s().apnap().collect();
    for player in players {
        let mine: Vec<DamageDivision> = divisions.iter().filter(|(by, _)| *by == player).map(|(_, d)| d.clone()).collect();
        if !mine.is_empty()
            && let Some(Answer::Shares(shares)) = cx.ask_rule(player, RequestKind::AssignCombatDamage { divisions: mine })?
        {
            chosen.extend(shares);
        }
    }

    let assignments = combat_assignments(cx, &attackers, &blockers, &chosen);
    // CR 510.2: all of it is dealt at once, and announced at once (CR 603.2c). A prevention shield two or more sources would
    // deal damage to prevents whose damage its player or controller chooses (CR 615.7).
    cx.instruction(|cx| {
        let (_, _, mut batch) = ops::damage_batch(cx, &assignments, true);
        batch.rest_of_preventions(cx);
    }, |_| true);
    Ok(())
}

/// CR 510.1/800.4e: assignments are legal before damage or its replacements are attempted.
fn combat_assignments(cx: &Cx, attackers: &[Attacker], blockers: &[ObjectId], chosen: &[(ObjectId, Option<ObjectId>, u32)]) -> Vec<(ObjectId, Target, u32)> {
    let mut assignments = Vec::new();
    for attacker in attackers {
        if chosen.iter().any(|s| s.0 == attacker.id) {
            for &(source, to, amount) in chosen.iter().filter(|s| s.0 == attacker.id) {
                let target = match to {
                    Some(creature) => Target::Object(creature),
                    None if combat_defender_present(cx, attacker.target) => attacker.target,
                    None => continue,
                };
                assignments.push((source, target, amount));
            }
            continue;
        }
        let power = crate::static_rules::combat_damage_of(cx, attacker.id);
        if !attacker.blocked {
            if combat_defender_present(cx, attacker.target) {
                assignments.push((attacker.id, attacker.target, power));
            }
            continue;
        }
        match blockers_of(cx, attacker.id).as_slice() {
            // CR 510.1c: a blocked attacker whose blockers are all gone assigns no combat damage - unless it has trample,
            // which then assigns all of it to the player or planeswalker it attacks (CR 702.19d).
            [] => {
                if cx.chars(attacker.id).has(Keyword::Trample) && combat_defender_present(cx, attacker.target) {
                    assignments.push((attacker.id, attacker.target, power));
                }
            }
            // CR 510.1c: blocked by exactly one creature, all of it to that creature.
            &[only] => assignments.push((attacker.id, Target::Object(only), power)),
            // Two or more: divided above, unless it assigns no combat damage at all (CR 510.1a: 0 power).
            _ => {}
        }
    }
    for &blocker in blockers {
        if chosen.iter().any(|s| s.0 == blocker) {
            for &(source, to, amount) in chosen.iter().filter(|s| s.0 == blocker) {
                if let Some(creature) = to {
                    assignments.push((source, Target::Object(creature), amount));
                }
            }
            continue;
        }
        // CR 510.1d: blocking exactly one creature, all of it to that creature; blocking none still in combat, none.
        if let &[only] = blocked_by(cx, blocker).as_slice() {
            assignments.push((blocker, Target::Object(only), crate::static_rules::combat_damage_of(cx, blocker)));
        }
    }
    assignments
}

/// The creatures still blocking `attacker` (CR 509.1h, 506.4): its blockers that have not left combat.
fn blockers_of(cx: &Cx, attacker: ObjectId) -> Vec<ObjectId> {
    cx.s().blocks.iter().filter(|b| b.attacker == attacker && live(cx.s(), b.blocker)).map(|b| b.blocker).collect()
}

/// The creatures `blocker` is still blocking (CR 509.1g, 506.4): those of its blocks whose attacker has not left combat.
fn blocked_by(cx: &Cx, blocker: ObjectId) -> Vec<ObjectId> {
    cx.s().blocks.iter().filter(|b| b.blocker == blocker && live(cx.s(), b.attacker)).map(|b| b.attacker).collect()
}

/// Whether combat damage can be assigned to what an attacker attacks (CR 510.1b, 702.19b): a player still in the game
/// (CR 800.4e), or a planeswalker or battle - which the combat state names only while it is being attacked (removed from
/// combat, it is cleared, `ops::remove_from_combat`, `ops::settle_combat`); a battle still being attacked keeps a
/// protector who has left the game until combat ends (CR 704.5x). Nothing, for an attacker attacking nothing (CR 506.4c).
/// A permanent named is still checked to be on the battlefield as a planeswalker or battle, so that a removal some path
/// failed to record deals no damage to a permanent that is not being attacked.
fn combat_defender_present(cx: &Cx, target: Target) -> bool {
    match target {
        Target::Player(p) => cx.s().in_game(p),
        Target::Object(id) => live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Battlefield) && ops::attackable_types(&cx.chars(id)) != TypeSet::NONE,
        _ => false,
    }
}

/// Lethal damage for each of `creatures` from `source` (CR 702.19b, 120.4a): its toughness less the damage marked on it, or
/// any 1 more from a source with deathtouch (CR 702.2c), unless it already has lethal damage marked.
fn lethal_for(cx: &Cx, source: ObjectId, creatures: &[ObjectId]) -> Vec<u32> {
    let deathtouch = cx.chars(source).has(Keyword::Deathtouch);
    creatures
        .iter()
        .map(|&c| {
            let chars = cx.chars(c);
            let remaining = (i64::from(chars.toughness.unwrap_or(0)) - i64::from(cx.s().obj(c).damage)).max(0) as u32;
            if deathtouch { remaining.min(1) } else { remaining }
        })
        .collect()
}

/// The suggestion for a division the creature's controller makes: lethal to each in turn, the rest to the last creature, or
/// with `rest_to_defender` (trample) to the player or planeswalker attacked. The amounts, and what goes past the creatures.
fn lethal_in_turn(power: u32, lethal: &[u32], rest_to_defender: bool) -> (Vec<u32>, u32) {
    let mut left = power;
    let mut suggested = Vec::new();
    for (i, &l) in lethal.iter().enumerate() {
        let share = if i + 1 == lethal.len() && !rest_to_defender { left } else { left.min(l) };
        suggested.push(share);
        left -= share;
    }
    (suggested, left)
}

/// The suggestion for a division banding gives to the controller of the creatures it is divided among (CR 702.22j-k): all of
/// it to one of them, one it would not kill if there is one (the toughest such), else the one worth least by its power and
/// toughness. Each amount, index for index.
fn spared(cx: &Cx, power: u32, creatures: &[ObjectId], lethal: &[u32]) -> Vec<u32> {
    let worth = |c: ObjectId| {
        let chars = cx.chars(c);
        i32::from(chars.power.unwrap_or(0)) + i32::from(chars.toughness.unwrap_or(0))
    };
    let survivor = (0..creatures.len()).filter(|&i| lethal[i] > power).max_by_key(|&i| lethal[i]);
    let pick = survivor.unwrap_or_else(|| (0..creatures.len()).min_by_key(|&i| worth(creatures[i])).unwrap_or(0));
    (0..creatures.len()).map(|i| if i == pick { power } else { 0 }).collect()
}

/// The damage division a blocked attacker has a choice in: more than one blocker (CR 510.1c), or trample and more damage than
/// is lethal to its one blocker (CR 702.19b). Blocked by a creature with banding, the defending player divides it among its
/// blockers as they choose - none of it to the player, trample or not (CR 702.22j) - and there is no choice with one blocker.
fn attacker_division(cx: &Cx, attacker: &Attacker) -> Option<DamageDivision> {
    let power = crate::static_rules::combat_damage_of(cx, attacker.id);
    let blockers = blockers_of(cx, attacker.id);
    if power == 0 || blockers.is_empty() {
        return None;
    }
    let banding = blockers.iter().any(|&b| cx.chars(b).has(Keyword::Banding));
    let trample = cx.chars(attacker.id).has(Keyword::Trample) && !banding && combat_defender_present(cx, attacker.target);
    let lethal = lethal_for(cx, attacker.id, &blockers);
    if blockers.len() == 1 && (!trample || power <= lethal[0]) {
        return None;
    }
    let (suggested, past) = if banding { (spared(cx, power, &blockers, &lethal), 0) } else { lethal_in_turn(power, &lethal, trample) };
    Some(DamageDivision {
        source: attacker.id,
        banding,
        damage: power,
        creatures: blockers,
        lethal,
        may_assign_to_defender: trample,
        defender: if trample { attacker.target } else { Target::None },
        suggested,
        suggested_to_defender: past,
    })
}

/// The damage division a blocker blocking two or more creatures has (CR 510.1d): among them as its controller chooses, or,
/// when one of them has banding, as the attacking player chooses (CR 702.22k). A blocker blocks two or more by blocking a band
/// (CR 702.22h), or as a creature that can block more than one (CR 509.1a).
fn blocker_division(cx: &Cx, blocker: ObjectId) -> Option<DamageDivision> {
    let power = crate::static_rules::combat_damage_of(cx, blocker);
    let blocked = blocked_by(cx, blocker);
    if power == 0 || blocked.len() < 2 {
        return None;
    }
    let banding = blocked.iter().any(|&a| cx.chars(a).has(Keyword::Banding));
    let lethal = lethal_for(cx, blocker, &blocked);
    let suggested = if banding { spared(cx, power, &blocked, &lethal) } else { lethal_in_turn(power, &lethal, false).0 };
    Some(DamageDivision {
        source: blocker,
        banding,
        damage: power,
        creatures: blocked,
        lethal,
        may_assign_to_defender: false,
        defender: Target::None,
        suggested,
        suggested_to_defender: 0,
    })
}

#[cfg(test)]
#[path = "departed_combat_tests.rs"]
mod departed_combat_tests;

#[cfg(test)]
#[path = "removed_from_combat_tests.rs"]
mod removed_from_combat_tests;

/// Cleanup (CR 514): discard to hand size, then damage wears off and "until end of turn" effects end.
pub(crate) fn cleanup(cx: &mut Cx) -> R<()> {
    let active = cx.s().turn.active;
    // CR 514.1: the active player discards down to their maximum hand size. What triggers "at the beginning of the next
    // cleanup step" (CR 603.7) triggered as the step began (`settle::step_began`) and waits until after these actions (CR
    // 514.3a, `begin_step`).
    let hand = cx.s().zone(Zone::hand(active)).to_vec();
    let maximum = crate::actions::maximum_hand_size(cx, active);
    if let Some(maximum) = maximum.filter(|&m| hand.len() > m as usize) {
        let count = count_u32(hand.len() - maximum as usize);
        if let Some(Answer::Cards(cards)) = cx.ask_rule(active, RequestKind::DiscardToHandSize { count, hand })? {
            ops::discard_together(cx, &cards);
        }
    }
    // CR 514.2: all damage marked on permanents, phased-out ones included, is removed, except where a static ability says it
    // isn't ("damage isn't removed from this creature during cleanup steps"). Having been dealt deathtouch damage matters
    // only to the next check of state-based actions (CR 704.5h), so that record goes either way.
    let damaged: Vec<ObjectId> = cx
        .s()
        .battlefield()
        .iter()
        .chain(cx.s().zone(Zone::PHASED_OUT))
        .copied()
        .filter(|&id| cx.s().obj(id).damage > 0 || cx.s().obj(id).deathtouched)
        .collect();
    if !damaged.is_empty() {
        let stays = crate::static_rules::damage_stays(cx, &damaged);
        for id in damaged {
            let keep = stays.contains(&id);
            cx.st().update_obj(id, |o| {
                if !keep {
                    o.damage = 0;
                }
                o.deathtouched = false;
            });
        }
    }
    // "This combat" ends with its combat phase (`next_step`); one made outside combat ends with the turn at the latest.
    if cx.s().effects.iter().any(|e| e.duration == Duration::EndOfCombat) {
        cx.st().effects_retain(|e| e.duration != Duration::EndOfCombat);
    }
    // "During its controller's next turn" ends with that turn (`spend_next_turn_effects`), however long it waits for it: a
    // change of control before then moves it to the new controller's next turn, so no earlier moment says it never comes.
    // Its creature leaving ends it with the creature (CR 400.7).
    spend_next_turn_effects(cx);
    if cx.s().effects.iter().any(|e| e.duration == Duration::EndOfTurn) {
        // A permanent that was only a copy until end of turn is what it was again (CR 707.2, 611.2b), as far as the copy
        // effects that are left say: the state undoes a copy effect as it removes it (`GameState::effect_remove`).
        cx.st().effects_retain(|e| e.duration != Duration::EndOfTurn);
        // Control is settled for every permanent: an effect that ended may have decided another's controller too (a copy
        // effect that made a permanent an Aura that gives control), before anything reads it (the untap step's "untap all
        // permanents you control").
        ops::settle_lasting_and_control(cx);
    }
    // "This turn" delayed abilities end with it (CR 603.7b), and so does a "when you next ... this turn" that never triggered:
    // as the "until end of turn" effects end, at the same moment (CR 514.2), so one still triggers on what their ending does
    // (Stolen Uniform's "when you lose control of that Equipment this turn" as its "until end of turn" control ends; its
    // ruling: "whether it's because the 'until end of turn' duration ended or ..."), and its trigger gives the active
    // player priority in this cleanup step and another one follows (CR 514.3a).
    for at in (0..cx.s().delayed.len()).rev() {
        let delayed = cx.s().delayed[at];
        let expires = match delayed.when {
            DelayedWhen::ThisTurn | DelayedWhen::WatchingThisTurn | DelayedWhen::NextThisTurn => true,
            DelayedWhen::StepOfNextTurn { of, .. } => crate::settle::delayed_turn(cx, delayed.since, of) != crate::settle::DelayedTurn::Waiting,
            _ => false,
        };
        if expires {
            cx.st().delayed_remove(at);
        }
    }
    // Permissions to play cards end with the turn they name (CR 514.2), "this turn" or "your next turn"; one for a
    // card that has since moved is gone with the old object (CR 400.7).
    if !cx.s().permissions.is_empty() {
        let number = cx.s().turn.number;
        let moved: Vec<ObjectId> =
            cx.s().permissions.iter().filter_map(|p| p.what.card()).filter(|&c| cx.s().obj(c).became.is_some()).collect();
        cx.st().permissions_retain(|p| {
            p.what.card().is_none_or(|card| !moved.contains(&card))
                && match p.grant.expiry {
                    Expiry::Never => true,
                    Expiry::EndOfTurn => false,
                    Expiry::EndOfYourNextTurn => !(p.player == active && number > p.turn),
                    Expiry::UntilYourNextEndStep | Expiry::UntilYourNextStep(_) | Expiry::WhileControls(_) | Expiry::UntilAnotherExiledWith(_) => true,
                }
        });
    }
    Ok(())
}

/// An effect ends the turn (CR 724.1), as the spell or ability with that instruction resolves. What follows the
/// resolution - no more priority in this step, the step's end with combat's, the cleanup step - is `Turn::ending`'s, read
/// by `priority` and `next_step`.
pub fn end_the_turn(cx: &mut Cx) -> R<()> {
    let player = cx.s().turn.active;
    cx.record(Event::TurnEnded { player });
    // CR 724.1a-c. Abilities that trigger from here on wait for the cleanup step (724.1f).
    clear_the_stack_and_check(cx)?;
    // CR 724.1d: the step ends once this resolution is over, and the game skips straight to the cleanup step.
    cx.st().update_turn(|t| t.ending = true);
    Ok(())
}

/// An effect ends the combat phase (CR 724.2), as the spell or ability with that instruction resolves: Mandate of Peace,
/// Merciful Mandate. The first instructions are the turn end's (724.2a-c, `clear_the_stack_and_check`), and what follows
/// the resolution is `Turn::combat_ending`'s, read by `priority` and `next_step`: the step ends with no player given
/// priority again, the combat phase ends with it, and the game skips straight to the next phase - the end of combat step
/// is skipped, so "at end of combat" abilities don't trigger (724.2d-e) - where the abilities that triggered during the
/// process go on the stack (724.2f). Outside a combat phase nothing happens (724.2g).
pub fn end_the_combat_phase(cx: &mut Cx) -> R<()> {
    if !matches!(
        cx.s().turn.step(),
        Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers | Step::CombatDamage | Step::EndOfCombat
    ) {
        return Ok(());
    }
    clear_the_stack_and_check(cx)?;
    cx.st().update_turn(|t| t.combat_ending = true);
    Ok(())
}

/// The instructions an effect that ends the turn or the combat phase begins with (CR 724.1a-c, 724.2a-c): triggered
/// abilities not yet on the stack cease to exist, every object on the stack is exiled, and state-based actions are checked
/// with no player given priority.
fn clear_the_stack_and_check(cx: &mut Cx) -> R<()> {
    // CR 724.1a, 724.2a: abilities that triggered before this began and are not yet on the stack cease to exist. Those
    // that trigger from here on are kept for the step or phase the game goes on to (724.1f, 724.2f).
    while let Some(last) = cx.s().triggers.len().checked_sub(1) {
        cx.st().trigger_remove(last);
    }
    // CR 724.1b, 724.2b: every object on the stack is exiled, the resolving one among them, all at once. An ability on the
    // stack is no card, and is gone; a copy of a spell, exiled, ceases to exist as the state-based actions are checked
    // (CR 704.5e, 707.10a). Exiled is not countered.
    let spells: Vec<ops::Moving> = cx
        .s()
        .stack
        .iter()
        .rev()
        .filter(|i| i.kind == StackKind::Spell)
        .map(|i| ops::Moving::to(i.object, Zone::exile(cx.s().obj(i.object).owner)))
        .collect();
    for at in (0..cx.s().stack.len()).rev() {
        if cx.s().stack[at].kind != StackKind::Spell {
            cx.st().stack_remove(at);
        }
    }
    if !spells.is_empty() {
        ops::move_together(cx, &spells, ZoneReason::Exile);
    }
    // CR 724.1c, 724.2c: state-based actions, with no player given priority and no triggered ability put on the stack.
    crate::settle::state_based_actions_alone(cx)?;
    Ok(())
}

/// The next player in turn order after `p` who has not lost (CR 800.4a).
fn next_in_game(s: &GameState, p: PlayerId) -> PlayerId {
    let mut next = s.next_player(p);
    while s.player(next).lost && next != p {
        next = s.next_player(next);
    }
    next
}

/// A priority round: state-based actions and triggers first, then the player with priority acts or passes; when
/// everyone has passed in succession the top of the stack resolves, or with an empty stack the step ends (CR 117).
pub fn priority(cx: &mut Cx) -> R<Flow> {
    // A player who conceded as priority was waiting leaves the game here, on every replay (`Game::concede`), before anything
    // else: priority is given again once they have, so the state-based actions their leaving calls for are performed and what
    // it triggered - another player taking the initiative, a "when that player loses the game" - goes on the stack first
    // (CR 117.5, 800.4a), not after the player holding priority acts or passes.
    cx.concessions_here();
    // CR 724.1c-d, 724.2c-d: an effect has ended the turn or the combat phase. No player gets priority again in this step,
    // and the abilities that triggered since wait for the cleanup step (724.1f) or the next phase (724.2f).
    if cx.s().turn.ending || cx.s().turn.combat_ending {
        return Ok(Flow::Done);
    }
    if cx.s().unsettled || !cx.s().triggers.is_empty() {
        cx.st().frame_push(Frame::new(FrameKind::Settle));
        return Ok(Flow::Again);
    }
    if cx.s().is_over() {
        return Ok(Flow::Done);
    }
    // A player who has lost has left the game and is not given priority (CR 800.4a): it goes to the next player still in.
    let mut player = cx.s().turn.priority;
    if cx.s().player(player).lost {
        player = next_in_game(cx.s(), player);
        cx.st().update_turn(|t| t.priority = player);
    }
    // RX-4: the game has come round a loop of passes for the last time, and is a draw.
    if crate::loops::at_priority(cx) {
        return Ok(Flow::Done);
    }
    let options = actions::priority_options(cx, player);
    // What working out the options asked of the history is no read that tells rounds of a loop of passes apart: while every
    // player passes, what they could have done changes nothing (`loops`).
    cantrip_api::history::take_reads();
    let Some(answer) = cx.ask_rule(player, RequestKind::Priority { options })? else {
        // The player left the game as they held priority: it passes to the next player in turn order still in, as their own
        // priority, no pass counted for the one who left (CR 800.4a, 800.4j).
        let next = next_in_game(cx.s(), player);
        cx.st().update_turn(|t| t.priority = next);
        return Ok(Flow::Again);
    };
    match answer {
        Answer::Act(action) => {
            crate::loops::acted(cx);
            let mut frame = Frame::new(FrameKind::Act { player });
            frame.answers.push((Some(player), Answer::Act(action)));
            cx.st().frame_push(frame);
            Ok(Flow::Again)
        }
        _ => {
            let players = cx.s().apnap().count() as u8;
            let passes = cx.s().turn.passes + 1;
            if passes < players {
                let next = next_in_game(cx.s(), player);
                cx.st().update_turn(|t| {
                    t.passes = passes;
                    t.priority = next;
                });
                return Ok(Flow::Again);
            }
            let active = cx.s().turn.active;
            cx.st().update_turn(|t| {
                t.passes = 0;
                t.priority = active;
            });
            let Some(top) = cx.s().stack.last() else {
                return Ok(Flow::Done);
            };
            let serial = top.serial;
            cx.st().frame_push(Frame::new(FrameKind::Resolve { serial }));
            Ok(Flow::Again)
        }
    }
}

#[allow(dead_code)]
fn _unused(cx: &Cx) -> bool {
    layers::live(cx.s(), ObjectId(0))
}

#[cfg(test)]
#[path = "landwalk_tests.rs"]
mod landwalk_tests;

#[cfg(test)]
#[path = "step_deadline_tests.rs"]
mod step_deadline_tests;
