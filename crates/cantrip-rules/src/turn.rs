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
    shuffle(cx.st(), Zone::library(player));
    cx.record(Event::Shuffled { player });
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
    let s = &mut game.state;
    let turn = s.turn;
    let ending = turn.step();
    // The combat phase ends with its end of combat step, or with whatever step of it an effect ended the turn in
    // (CR 724.1d).
    let combat_ends = ending == Step::EndOfCombat
        || (turn.ending && matches!(ending, Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers | Step::CombatDamage));

    // Mana empties as each step ends (CR 500.4), except the mana that lasts until the end of combat, which is lost as
    // the combat phase ends (CR 106.4, 500.5a).
    if turn.number > 0 {
        for p in s.seats().collect::<Vec<_>>() {
            s.empty_mana(p, !combat_ends);
        }
    }
    // CR 724.1d: a turn ended during combat removes every creature and planeswalker from combat as the step ends, as
    // the end of the combat phase does.
    if turn.number > 0 && combat_ends {
        s.set_attackers(Vec::new());
        s.set_blocks(Vec::new());
        s.update_turn(|t| {
            t.attacked = false;
            t.damage_round = 0;
        });
    }

    let (number, active, step) = if turn.number == 0 {
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
            // An additional combat phase follows the one that ends (CR 500.8): a whole new one, from its beginning.
            Some(Step::PostcombatMain) if ending == Step::EndOfCombat && turn.extra_combats > 0 => {
                (turn.number, turn.active, Step::BeginCombat)
            }
            Some(next) => (turn.number, turn.active, next),
            None => (turn.number + 1, next_turn_taker(s, turn.active), Step::Untap),
        }
    };
    let previous = turn.active;
    s.update_turn(|t| {
        if number != turn.number {
            t.previous = previous;
        }
        t.number = number;
        t.active = active;
        if number != turn.number {
            t.extra_combats = 0;
        } else if ending == Step::EndOfCombat && step == Step::BeginCombat {
            t.extra_combats -= 1;
        }
        t.set_step(step);
        t.priority = active;
        t.passes = 0;
        t.ending = false;
        t.cleanup_again = false;
    });
    s.frame_push(Frame::new(FrameKind::BeginStep));
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
            (next_in_game(s, rotation.unwrap_or(after)), 0)
        }
    };
    if s.turn.extra != extra {
        s.update_turn(|t| t.extra = extra);
    }
    // "At the beginning of that turn's [step]" waits for one extra turn (CR 603.7): once that turn has ended without the
    // step, or will never be taken, the ability never triggers.
    let stale = |d: &cantrip_state::Delayed, s: &GameState| {
        matches!(d.when, DelayedWhen::ExtraTurnStep { turn, .. } if turn != extra && !s.extra_turns.iter().any(|t| t.id == turn))
    };
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

/// The start of the game (CR 103): who goes first, the opening hands, mulligans.
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
            if mulligans >= 7 {
                cx.st().update_player(p, |pl| pl.kept = true);
                continue;
            }
            match cx.ask_rule(p, RequestKind::Mulligan { mulligans })? {
                None => continue,
                Some(Answer::Yes) => redraw.push(p),
                Some(_) => {
                    cx.st().update_player(p, |pl| pl.kept = true);
                    if mulligans > 0 {
                        let hand = cx.s().zone(Zone::hand(p)).to_vec();
                        let answer = cx.ask_rule(
                            p,
                            RequestKind::choose_cards(hand[0], Purpose::Discard, hand.clone(), hand, u32::from(mulligans), false, ""),
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
            if !(action.condition)(cx, &Asker { source: card, controller: p }) {
                continue;
            }
            if !matches!(cx.ask_rule(p, RequestKind::May { source: card, purpose: Purpose::Ask(&BEGIN_ON_BATTLEFIELD) })?, Some(Answer::Yes)) {
                continue;
            }
            let with: Vec<(CounterKind, i16)> = action.counter.map(|(k, n)| (k, n as i16)).into_iter().collect();
            ops::put_onto_battlefield_with(cx, &[(card, p, false)], ZoneReason::Other, &with)?;
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

fn push_priority(cx: &mut Cx) {
    cx.st().frame_push(Frame::new(FrameKind::Priority));
}

/// The step's turn-based actions (CR 500.2), then its priority round.
pub fn begin_step(cx: &mut Cx) -> R<Flow> {
    let turn = cx.s().turn;
    let active = turn.active;
    if turn.step() != Step::Untap {
        crate::settle::step_began(cx, turn.step());
    }
    match turn.step() {
        Step::Untap => {
            // CR 614.10, 500.11: a turn that would begin may be skipped instead, and then never begins.
            if skip_turn(cx, active)? {
                // The next player's turn takes its place, under this one's number: as if cleanup had just ended it.
                cx.st().update_turn(|t| {
                    t.number -= 1;
                    t.set_step(Step::Cleanup);
                });
                return Ok(Flow::Done);
            }
            // CR 502: the turn begins; the active player's permanents untap. No one gets priority.
            let number = turn.number;
            cx.st().update_player(active, |p| {
                p.turn_began = number;
                p.lands_played = 0;
            });
            // How many turns a player has taken is asked of the history (`cantrip_mechanics::history::in_your_first_turns`).
            cx.record(Event::TurnBegan { player: active });
            // CR 502.1: first, the active player's phased-out permanents phase in.
            crate::phasing::untap_step(cx, active);
            // CR 502.2: the untap step's second turn-based action reads what the previous turn's active player cast,
            // before the counts are cleared for this turn.
            crate::day_night::check_untap_step(cx)?;
            // "Until your next turn" ends as that turn begins (CR 611.2b).
            if cx.s().effects.iter().any(|e| e.duration == Duration::UntilNextTurnOf(active)) {
                cx.st().effects_retain(|e| e.duration != Duration::UntilNextTurnOf(active));
            }
            // The same for a delayed ability that lasts until then (CR 603.7b).
            for at in (0..cx.s().delayed.len()).rev() {
                if cx.s().delayed[at].when == DelayedWhen::UntilNextTurnOf(active) {
                    cx.st().delayed_remove(at);
                }
            }
            untap_step(cx, active)?;
            return Ok(Flow::Done);
        }
        Step::Draw => {
            // CR 103.8a: the player who goes first skips the draw of their first turn (in a two-player game).
            let skip = turn.number == 1 && active == turn.starting && cx.s().players.len() == 2;
            if !skip {
                ops::draw_cards(cx, active, 1);
            }
        }
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
        let card = obj.copied_card().unwrap_or(obj.card);
        let asker = Asker { source: id, controller: player };
        // A scan once a turn of what is on the battlefield, for a card that has such a rule.
        if !cx.card(card).turn_begin.is_empty() && cx.lost_all_abilities(id) {
            continue;
        }
        for rule in cx.card(card).turn_begin {
            if (rule.applies)(&*cx, &asker, player) {
                optional.push((id, rule));
            }
        }
    }
    let Some(at) = pending else {
        for (id, rule) in optional {
            if cx.ask_rule(player, RequestKind::May { source: id, purpose: Purpose::Ask(rule.question) })? == Some(Answer::Yes) {
                // A replacement effect's own doing, its controller's (CR 609.1, 614.10).
                cx.acting_as(player, |cx| (rule.on_skip)(cx, &Asker { source: id, controller: player }))?;
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
            cx.acting_as(player, |cx| (rule.on_skip)(cx, &Asker { source: id, controller: player }))?;
            return Ok(true);
        }
    }
    let keep: Vec<bool> = (0..cx.s().effects.len()).map(|i| i != at).collect();
    cx.st().effects_keep(&keep);
    Ok(true)
}

/// CR 502.3: the active player's permanents untap, all at once. Except an exerted one, for the untap step it sits out
/// (CR 701.43a), and one that doesn't untap during its controller's untap step (CR 502.3); and one with a stun counter
/// has a counter removed from it instead (CR 122.1d). With them, the other players' permanents that a static ability of
/// a permanent another player controls untaps "during each other player's untap step" (`CardDef::untap_in_others_untap_steps`).
pub(crate) fn untap_step(cx: &mut Cx, active: PlayerId) -> R<()> {
    // CR 502.3: the active player determines which permanents will untap, then they untap together. A permanent whose
    // card says "you may choose not to untap this ... during your untap step" is the active player's to keep tapped:
    // asked of each, in battlefield order, before any untaps.
    let mut untapping = Vec::new();
    // The permanents of the other players with an ability that untaps permanents during this untap step: a table lookup
    // per permanent, and nothing more in a game without one.
    let mut others: Vec<(ObjectId, PlayerId, &'static CardDef)> = Vec::new();
    for id in cx.s().battlefield().to_vec() {
        let obj = cx.s().obj(id);
        if obj.controller != active {
            let def = cx.card(cx.cards().current(obj));
            if !def.untap_in_others_untap_steps.is_empty() {
                others.push((id, obj.controller, def));
            }
            continue;
        }
        if !obj.tapped || obj.marks.exerted_by == Some(active) || cx.chars(id).has(Keyword::DoesNotUntap) {
            continue;
        }
        if cx.card(cx.cards().current(obj)).may_not_untap && !cx.lost_all_abilities(id) {
            let ask = RequestKind::May { source: id, purpose: Purpose::Ask(&UNTAP_IT) };
            if cx.ask_rule(active, ask)? != Some(Answer::Yes) {
                continue;
            }
        }
        untapping.push(id);
    }
    // "Untap [these] during each other player's untap step" (CR 611.3a): what each rule is true of now, before anything
    // untaps, untaps with the active player's permanents and is no one's choice (Bender's Waterskin ruling, 2025-10-02).
    // A permanent of a player who isn't active is outside "doesn't untap during its controller's untap step" (CR 502.3,
    // 701.43a); a stun counter still replaces its untap (CR 122.1d, `ops::untap`). A source that has lost all its
    // abilities has no such rule (CR 613.1f).
    others.retain(|&(id, _, _)| !cx.lost_all_abilities(id));
    if !others.is_empty() {
        for id in cx.s().battlefield().to_vec() {
            let obj = cx.s().obj(id);
            if obj.controller == active || !obj.tapped {
                continue;
            }
            if others.iter().any(|&(source, controller, def)| {
                let asker = Asker { source, controller };
                def.untap_in_others_untap_steps.iter().any(|rule| rule(&*cx, &asker, id))
            }) {
                untapping.push(id);
            }
        }
    }
    for id in untapping {
        ops::untap(cx, id);
    }
    // No player gets priority in the untap step (CR 502.4), so no state-based actions are checked before the upkeep begins:
    // an effect "for as long as [it] remains tapped" ends the moment its source untaps (CR 611.2b), and the controller it
    // gave back is the one the upkeep's "at the beginning of your upkeep" abilities see (CR 613.1b).
    ops::settle_lasting_and_control(cx);
    // That untap step has now happened for everything its player exerted, whoever controls it now (CR 701.43a).
    for id in cx.s().battlefield().to_vec() {
        if cx.s().obj(id).marks.exerted_by == Some(active) {
            cx.st().update_obj(id, |o| o.marks.exerted_by = None);
        }
    }
    Ok(())
}

/// "You may choose not to untap this ... during your untap step" (CR 502.3), asked of the active player for each such
/// permanent of theirs that is tapped.
const UNTAP_IT: Question = Question::new("UntapInUntapStep", "untap it during this untap step?", "You may choose to leave it tapped", "Untap it", "Leave it tapped");

/// The battles the active player may attack (CR 310.9b, 508.1b): those whose protector is one of the defending
/// players. A Siege its attacker controls is one of them (CR 310.12a); one the attacker protects never is.
fn attackable_battles(cx: &Cx, defending: &[PlayerId]) -> Vec<ObjectId> {
    if !cx.s().battlefield().iter().any(|&id| cx.s().obj(id).marks.protector.is_some()) {
        return Vec::new();
    }
    cx.s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            cx.s().obj(id).marks.protector.is_some_and(|p| defending.contains(&p)) && {
                let c = cx.chars(id);
                c.is(CardType::Battle) && !c.has(Keyword::CantBeAttacked)
            }
        })
        .collect()
}

/// The planeswalkers the active player may attack (CR 508.1b): those a defending player controls, except one that
/// can't be attacked (CR 508.1c).
fn attackable_planeswalkers(cx: &Cx, defending: &[PlayerId]) -> Vec<ObjectId> {
    cx.s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| defending.contains(&cx.s().obj(id).controller) && {
            let c = cx.chars(id);
            c.is(CardType::Planeswalker) && !c.has(Keyword::CantBeAttacked)
        })
        .collect()
}

fn defending_player(cx: &Cx, attack: Target) -> Option<PlayerId> {
    defending_player_of(cx.s(), attack)
}

fn declare_attackers(cx: &mut Cx) -> R<()> {
    let active = cx.s().turn.active;
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
                // CR 508.1c: an effect saying it can't attack, or a static ability.
                && !cx.s().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::CantAttack)
                && crate::static_rules::attack_rules_allow(&*cx, cx.cards(), id)
                && !summoning_sick(cx.cards(), cx.s(), &cx.g.log, id)
                // CR 702.3b: defender, unless an effect or a static ability lets it attack as though it didn't have it
                // (CR 609.4), which lifts no other restriction: asked last, of a creature that could otherwise attack.
                && (!chars.has(Keyword::Defender) || crate::static_rules::may_attack_despite_defender(&*cx, cx.cards(), id))
        })
        .collect();
    if eligible.is_empty() {
        return Ok(());
    }
    let players: Vec<PlayerId> = cx.s().opponents(active).collect();
    // CR 310.9b, 508.1b: every battle a defending player protects, whoever controls it. Looked for only when a battle
    // is on the battlefield.
    let battles = attackable_battles(cx, &players);
    let planeswalkers = attackable_planeswalkers(cx, &players);
    // CR 508.1d, 508.1h: what each attack costs, where a static ability makes attacking cost something.
    let targets: Vec<Target> = players
        .iter()
        .map(|&p| Target::Player(p))
        .chain(planeswalkers.iter().chain(&battles).map(|&id| Target::Object(id)))
        .collect();
    let costs = crate::static_rules::attack_costs(cx, &eligible, &targets);
    // CR 508.1d: a cost to attack is never a requirement, so a creature that attacks each combat if able need not attack
    // where every attack open to it costs something.
    let requirements: Vec<ObjectId> = eligible
        .iter()
        .copied()
        .filter(|&id| cx.chars(id).must_attack && targets.iter().any(|&t| AttackCost::free(&costs, id, t)))
        .collect();
    let can_pay = if costs.is_empty() { 0 } else { actions::available(cx, active).most_unrestricted() };
    let kind = RequestKind::DeclareAttackers { eligible, players, planeswalkers, battles, requirements, costs, can_pay };
    let attacks = loop {
        // A declaration whose costs are not paid is taken back (CR 508.1, 733): the game returns to the moment before it,
        // the mana abilities activated while paying undone, and the attackers are declared again.
        let mark = cx.s().mark();
        let log_mark = cx.g.log.len();
        // An active player who has left declares nothing, and the turn goes on without them (CR 800.4j).
        let Some(Answer::Attack(attacks)) = cx.ask_rule(active, kind.clone())? else { return Ok(()) };
        // CR 508.1e: the bands among them, announced before they tap.
        let bands = declare_bands(cx, active, &attacks)?;
        // CR 508.1f: the chosen creatures become tapped, which is no cost, before any cost is paid (CR 508.1g-j).
        for &(id, _) in &attacks {
            if !cx.chars(id).has(Keyword::Vigilance) {
                ops::tap(cx, id);
            }
        }
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
                    cx.st().revert(mark);
                    cx.g.log.truncate(log_mark);
                    continue;
                }
                Err(stop) => return Err(stop),
            }
        }
        break (attacks, bands);
    };
    let (attacks, bands) = attacks;
    let mut attackers = Vec::new();
    for ((id, target), band) in attacks.into_iter().zip(bands) {
        attackers.push(Attacker { band, ..Attacker::new(id, target) });
    }
    let attacked = !attackers.is_empty();
    cx.st().update_turn(|t| t.attacked = attacked);
    cx.st().set_attackers(attackers.clone());
    // One batch (CR 603.2c): "whenever one or more creatures attack" triggers once; "whenever you attack" after the
    // creatures' own abilities (CR 508.1m).
    let mut events: Vec<Event> = attackers.iter().map(|a| Event::Attacks { attacker: a.id, defender: a.target }).collect();
    if attacked {
        events.push(Event::AttackersDeclared { player: active, count: count_u32(attackers.len()) });
    }
    cx.emit_batch(&events, None);
    Ok(())
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
/// can block only creatures with flying, flying (CR 702.9b), shadow (CR 702.28b), landwalk (CR 702.14c), protection (CR 702.16f). Menace (CR 702.110b) is about the
/// whole declaration, not a pair.
fn can_block(cx: &Cx, blocker: ObjectId, attacker: ObjectId, rule_sources: &[ObjectId]) -> bool {
    let a = cx.chars(attacker);
    let b = cx.chars(blocker);
    if a.has(Keyword::CantBeBlocked) || b.has(Keyword::CantBlock) || b.has(Keyword::Decayed) || (b.has(Keyword::CanBlockOnlyCreaturesWithFlying) && !a.has(Keyword::Flying)) {
        return false;
    }
    if a.has(Keyword::Flying) && !b.has(Keyword::Flying) && !b.has(Keyword::Reach) {
        return false;
    }
    if a.has(Keyword::Shadow) != b.has(Keyword::Shadow) {
        return false;
    }
    // "Can't be blocked by creatures with power 2 or less" (CR 509.1b): a creature with no power can't block it either.
    if a.has(Keyword::CantBeBlockedByPowerTwoOrLess) && b.power.is_none_or(|p| p <= 2) {
        return false;
    }
    // Landwalk (CR 702.14c): not while the defending player - the blocker's controller - controls a land of its type, as the
    // layers have that land now. A creature with landwalk of its own is no exception (CR 702.14d).
    for (walk, land_type) in Keyword::LANDWALKS {
        if a.has(walk) {
            let defending = cx.s().obj(blocker).controller;
            let has_land = cx.s().battlefield().iter().any(|&id| {
                cx.s().obj(id).controller == defending && {
                    let land = cx.chars(id);
                    land.is(CardType::Land) && land.has_subtype(land_type)
                }
            });
            if has_land {
                return false;
            }
        }
    }
    // CR 509.1b: a static ability that limits who may block whom, of the blocker, of the attacker, or of another permanent
    // ("creatures you control with power 2 or less can't be blocked by ...", Delney, Streetwise Lookout).
    for &id in rule_sources {
        let obj = cx.s().obj(id);
        let card = obj.copied_card().unwrap_or(obj.card);
        if !cx.card(card).block_rules.iter().all(|rule| rule(cx, id, blocker, attacker)) {
            return false;
        }
    }
    // CR 611.2a: a resolved restriction is not an ability the source can lose. Its affected reference and
    // duration are maintained by the ordinary effect lifecycle, including phasing and new-object identity.
    for effect in &cx.s().effects {
        if let EffectKind::BlockRestriction { card, index } = effect.kind {
            if !(cx.card(card).resolved_block_rules[index as usize])(cx, effect.affected, blocker, attacker) {
                return false;
            }
        }
    }
    !a.protected_from_by(&b, cx.s().obj(blocker).controller)
}

/// "Can't block" (CR 509.1b): from an effect on this creature, or the turn's rule that no creature can.
fn cant_block(cx: &Cx, id: ObjectId) -> bool {
    let s = cx.s();
    s.turn.no_blocks_turn == s.turn.number
        || s.effects.iter().any(|e| match e.kind {
            EffectKind::CantBlock => e.affected == id,
            // "Creatures target player controls can't block this turn": a rule about the player, reaching later creatures too.
            EffectKind::CreaturesCantBlock(p) => s.obj(id).controller == p,
            _ => false,
        })
}

fn declare_blockers(cx: &mut Cx) -> R<()> {
    let attackers: Vec<ObjectId> = cx.s().attackers.iter().map(|a| a.id).collect();
    if attackers.is_empty() {
        return Ok(());
    }
    let Some(defender) = defending_player(cx, cx.s().attackers[0].target) else { return Ok(()) };
    let eligible: Vec<ObjectId> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| cx.s().obj(id).controller == defender && cx.chars(id).is(CardType::Creature) && !cx.s().obj(id).tapped)
        .filter(|&id| !cant_block(cx, id))
        .collect();
    // The permanents whose static abilities limit who may block whom (CR 509.1b), and which have not lost them (CR 613.1f).
    let rule_sources: Vec<ObjectId> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let obj = cx.s().obj(id);
            cx.cards().block_rules[obj.copied_card().unwrap_or(obj.card).0 as usize] && !cx.lost_all_abilities(id)
        })
        .collect();
    let mut allowed = Vec::new();
    for &b in &eligible {
        for &a in &attackers {
            if can_block(cx, b, a, &rule_sources) {
                allowed.push((b, a));
            }
        }
    }
    // Menace (CR 702.110b) and "except by three or more creatures" (CR 509.1b): an attacker that can't be blocked by
    // fewer than so many creatures is offered only if that many could, and not at all if it can't be blocked by that many
    // either ("can't be blocked by more than one creature" as well, CR 509.1b: the restrictions are cumulative).
    let menace: Vec<ObjectId> = attackers
        .iter()
        .copied()
        .filter(|&a| {
            let c = cx.chars(a);
            allowed.iter().filter(|p| p.1 == a).count() < c.minimum_blockers() || c.minimum_blockers() > c.maximum_blockers()
        })
        .collect();
    allowed.retain(|p| !menace.contains(&p.1));
    if !allowed.is_empty() {
        // CR 509.1c: the blocks effects require if able, and the most of them a declaration can obey.
        let pairs = crate::static_rules::block_requirements(cx, &allowed);
        let requirements = if pairs.is_empty() { BlockRequirements::default() } else { most_requirements(cx, &allowed, pairs) };
        // A defending player who has left blocks with nothing (CR 800.4a): no one else declares their blocks.
        let answer = cx.ask_rule(defender, RequestKind::DeclareBlockers { eligible, attackers, allowed, requirements })?;
        if let Some(Answer::Block(blocks)) = answer {
            // CR 509.1g: the blockers are declared together, a "blocks" event for each attacker each blocks, those of a band it
            // blocks through banding marked as not declared, so "whenever this creature blocks" triggers once (CR 509.3a).
            let mut events: Vec<Event> = Vec::with_capacity(blocks.len());
            let mut list = Vec::new();
            let mut attackers = cx.s().attackers.clone();
            for (blocker, attacker) in blocks {
                // CR 702.22h: a creature blocking a member of an attacking band blocks each other member of it too, a block of
                // each (CR 509.3b, 509.3d) though only the declared one is its "blocks" (CR 509.3a).
                let band = attackers.iter().find(|a| a.id == attacker).map_or(0, |a| a.band);
                let mut blocked = vec![attacker];
                blocked.extend(attackers.iter().filter(|a| band != 0 && a.band == band && a.id != attacker).map(|a| a.id));
                events.extend(blocked.iter().map(|&a| Event::Blocks { blocker, attacker: a, declared: a == attacker }));
                for attacker in blocked {
                    list.push(Block { blocker, attacker, first_strike: false });
                    if let Some(a) = attackers.iter_mut().find(|a| a.id == attacker) {
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

/// The most of the required blocks `pairs` (CR 509.1c) any declaration of blockers obeys without disobeying a restriction,
/// with one declaration that does. A declaration is legal in its restrictions where each blocker blocks at most one attacker,
/// each block is among `allowed` (CR 509.1b), and each attacker blocked has at least as many blockers as it requires and no
/// more than it allows (menace, "can't be blocked by more than one creature", CR 509.1b). Only blocks of an attacker some
/// requirement names can obey one or help another be obeyed (by making up a minimum), so only those are searched, the
/// blockers with a requirement first and their required blocks tried first; a branch that cannot beat the best found is cut.
fn most_requirements(cx: &Cx, allowed: &[(ObjectId, ObjectId)], pairs: Vec<(ObjectId, ObjectId)>) -> BlockRequirements {
    // A creature blocking one member of an attacking band blocks every member (CR 702.22h), so blocking any member obeys a
    // requirement to block another, and one block may obey two.
    let bands: Vec<(ObjectId, u32)> = cx.s().attackers.iter().filter(|a| a.band != 0).map(|a| (a.id, a.band)).collect();
    let mut req = BlockRequirements { pairs, most: 0, suggested: Vec::new(), bands };
    let mut targets: Vec<ObjectId> = Vec::new();
    for &(_, a) in &req.pairs {
        for x in std::iter::once(a).chain(req.bands.iter().map(|b| b.0).filter(|&x| req.blocks_through(x, a))) {
            if !targets.contains(&x) {
                targets.push(x);
            }
        }
    }
    let limits: Vec<(usize, usize)> = targets
        .iter()
        .map(|&a| {
            let c = cx.chars(a);
            (c.minimum_blockers(), c.maximum_blockers())
        })
        .collect();
    // Each blocker that could block one of `targets`, with the attackers it may block and how many requirements each block
    // obeys, the most first.
    let mut blockers: Vec<(ObjectId, Vec<(usize, usize)>)> = Vec::new();
    for &(b, a) in allowed {
        let Some(t) = targets.iter().position(|&x| x == a) else { continue };
        let gain = req.pairs.iter().filter(|&&(pb, ra)| pb == b && req.blocks_through(a, ra)).count();
        match blockers.iter_mut().find(|e| e.0 == b) {
            Some(e) => e.1.push((t, gain)),
            None => blockers.push((b, vec![(t, gain)])),
        }
    }
    for (_, options) in &mut blockers {
        options.sort_by_key(|&(_, gain)| std::cmp::Reverse(gain));
    }
    blockers.sort_by_key(|(_, options)| std::cmp::Reverse(options.first().map_or(0, |o| o.1)));
    // The most the blockers from each point on could still obey: each its best block.
    let required_after: Vec<usize> =
        (0..=blockers.len()).map(|i| blockers[i..].iter().map(|(_, options)| options.first().map_or(0, |o| o.1)).sum()).collect();

    struct Search<'a> {
        blockers: &'a [(ObjectId, Vec<(usize, usize)>)],
        limits: &'a [(usize, usize)],
        required_after: &'a [usize],
        counts: Vec<usize>,
        chosen: Vec<Option<usize>>,
        best: Option<(usize, Vec<Option<usize>>)>,
    }
    impl Search<'_> {
        fn go(&mut self, i: usize, obeyed: usize) {
            if self.best.as_ref().is_some_and(|(n, _)| obeyed + self.required_after[i] <= *n) {
                return;
            }
            if i == self.blockers.len() {
                let legal = self.counts.iter().zip(self.limits).all(|(&n, &(least, most))| n == 0 || (least <= n && n <= most));
                if legal {
                    self.best = Some((obeyed, self.chosen.clone()));
                }
                return;
            }
            let options = &self.blockers[i].1;
            for &(t, gain) in options {
                if self.counts[t] == self.limits[t].1 {
                    continue;
                }
                self.counts[t] += 1;
                self.chosen[i] = Some(t);
                self.go(i + 1, obeyed + gain);
                self.counts[t] -= 1;
            }
            self.chosen[i] = None;
            self.go(i + 1, obeyed);
        }
    }
    let mut search = Search {
        blockers: &blockers,
        limits: &limits,
        required_after: &required_after,
        counts: vec![0; targets.len()],
        chosen: vec![None; blockers.len()],
        best: None,
    };
    search.go(0, 0);
    let (most, chosen) = search.best.unwrap_or_default();
    req.suggested = blockers.iter().zip(chosen).filter_map(|((b, _), t)| t.map(|t| (*b, targets[t]))).collect();
    req.most = count_u32(most);
    req
}

/// Combat damage (CR 510): every attacker and blocker deals its damage at once. A creature with a choice in where its damage
/// goes - an attacker blocked by two or more creatures, or a trampler with more than lethal for its blocker (CR 510.1c,
/// 702.19b), a blocker blocking two or more creatures (CR 510.1d) - divides it as the player banding names chooses, else as
/// its controller does (CR 702.22j-k); the active player announces first, then each defending player (CR 510.1).
fn combat_damage(cx: &mut Cx) -> R<()> {
    // CR 510.4: with a first or double striker in combat as the step begins, there is a first-strike damage step before
    // the regular one.
    let in_combat: Vec<ObjectId> =
        cx.s().attackers.iter().map(|a| a.id).chain(cx.s().blocks.iter().map(|b| b.blocker)).collect();
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
        .filter(|a| live(cx.s(), a.id) && deals_now(cx, a.id, a.first_strike, first_strike_step))
        .collect();
    let mut blockers: Vec<ObjectId> = Vec::new();
    for b in &cx.s().blocks {
        if !blockers.contains(&b.blocker) && live(cx.s(), b.blocker) && deals_now(cx, b.blocker, b.first_strike, first_strike_step) {
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
    let mut chosen: Vec<(ObjectId, Option<ObjectId>, u16)> = Vec::new();
    let players: Vec<PlayerId> = cx.s().apnap().collect();
    for player in players {
        let mine: Vec<DamageDivision> = divisions.iter().filter(|(by, _)| *by == player).map(|(_, d)| d.clone()).collect();
        if !mine.is_empty()
            && let Some(Answer::Shares(shares)) = cx.ask_rule(player, RequestKind::AssignCombatDamage { divisions: mine })?
        {
            chosen.extend(shares);
        }
    }

    let mut assignments: Vec<(ObjectId, Target, u16)> = Vec::new();
    for attacker in &attackers {
        if chosen.iter().any(|s| s.0 == attacker.id) {
            for &(source, to, amount) in chosen.iter().filter(|s| s.0 == attacker.id) {
                let target = match to {
                    Some(creature) => Target::Object(creature),
                    None => attacker.target,
                };
                assignments.push((source, target, amount));
            }
            continue;
        }
        let power = crate::static_rules::combat_damage_of(cx, attacker.id);
        if !attacker.blocked {
            assignments.push((attacker.id, attacker.target, power));
            continue;
        }
        match blockers_of(cx, attacker.id).as_slice() {
            // CR 510.1c: a blocked attacker whose blockers are all gone assigns no combat damage - unless it has trample,
            // which then assigns all of it to the player or planeswalker it attacks (CR 702.19d).
            [] => {
                if cx.chars(attacker.id).has(Keyword::Trample) {
                    assignments.push((attacker.id, attacker.target, power));
                }
            }
            // CR 510.1c: blocked by exactly one creature, all of it to that creature.
            &[only] => assignments.push((attacker.id, Target::Object(only), power)),
            // Two or more: divided above, unless it assigns no combat damage at all (CR 510.1a: 0 power).
            _ => {}
        }
    }
    for &blocker in &blockers {
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
    // CR 510.2: all of it is dealt at once, and announced at once (CR 603.2c). A prevention shield two or more sources would
    // deal damage to prevents whose damage its player or controller chooses (CR 615.7).
    let mut batch = ops::DamageBatch::default();
    let events: Vec<Event> = ops::order_for_shields(cx, &assignments, true)
        .into_iter()
        .filter_map(|i| {
            let (source, target, amount) = assignments[i];
            ops::deal_damage_with(cx, source, target, amount, true, &mut batch)
        })
        .collect();
    batch.gain_lifelink(cx);
    cx.emit_batch(&events, None);
    Ok(())
}

/// The creatures still blocking `attacker` (CR 509.1h, 506.4): its blockers that have not left combat.
fn blockers_of(cx: &Cx, attacker: ObjectId) -> Vec<ObjectId> {
    cx.s().blocks.iter().filter(|b| b.attacker == attacker && live(cx.s(), b.blocker)).map(|b| b.blocker).collect()
}

/// The creatures `blocker` is still blocking (CR 509.1g, 506.4): those of its blocks whose attacker has not left combat.
fn blocked_by(cx: &Cx, blocker: ObjectId) -> Vec<ObjectId> {
    cx.s().blocks.iter().filter(|b| b.blocker == blocker && live(cx.s(), b.attacker)).map(|b| b.attacker).collect()
}

/// Lethal damage for each of `creatures` from `source` (CR 702.19b, 120.4a): its toughness less the damage marked on it, or
/// any 1 from a source with deathtouch (CR 702.2c).
fn lethal_for(cx: &Cx, source: ObjectId, creatures: &[ObjectId]) -> Vec<u16> {
    let deathtouch = cx.chars(source).has(Keyword::Deathtouch);
    creatures
        .iter()
        .map(|&c| {
            if deathtouch {
                return 1;
            }
            let chars = cx.chars(c);
            (chars.toughness.unwrap_or(0) - cx.s().obj(c).damage as i16).max(0) as u16
        })
        .collect()
}

/// The suggestion for a division the creature's controller makes: lethal to each in turn, the rest to the last creature, or
/// with `rest_to_defender` (trample) to the player or planeswalker attacked. The amounts, and what goes past the creatures.
fn lethal_in_turn(power: u16, lethal: &[u16], rest_to_defender: bool) -> (Vec<u16>, u16) {
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
fn spared(cx: &Cx, power: u16, creatures: &[ObjectId], lethal: &[u16]) -> Vec<u16> {
    let worth = |c: ObjectId| {
        let chars = cx.chars(c);
        chars.power.unwrap_or(0) + chars.toughness.unwrap_or(0)
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
    let trample = cx.chars(attacker.id).has(Keyword::Trample) && !banding;
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
/// when one of them has banding, as the attacking player chooses (CR 702.22k). A blocker blocks two or more only by blocking
/// a band (CR 702.22h).
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

/// Cleanup (CR 514): discard to hand size, then damage wears off and "until end of turn" effects end.
pub(crate) fn cleanup(cx: &mut Cx) -> R<()> {
    let active = cx.s().turn.active;
    // CR 514.1: "sacrifice it at the beginning of the next cleanup step", for the permanents whose card says so, each by its
    // controller, together.
    let doomed: Vec<ObjectId> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let obj = cx.s().obj(id);
            cx.card(obj.copied_card().unwrap_or(obj.card)).sacrifice_at_cleanup.is_some_and(|rule| rule(&*cx, id))
        })
        .collect();
    if !doomed.is_empty() {
        ops::sacrifice(cx, &doomed);
    }
    let hand = cx.s().zone(Zone::hand(active)).to_vec();
    if hand.len() > 7 && !crate::actions::no_maximum_hand_size(cx, active) {
        let count = count_u32(hand.len() - 7);
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
    // "This turn" delayed abilities end with it (CR 603.7b), and so does a "when you next ... this turn" that never triggered.
    for at in (0..cx.s().delayed.len()).rev() {
        if matches!(cx.s().delayed[at].when, DelayedWhen::ThisTurn | DelayedWhen::NextThisTurn) {
            cx.st().delayed_remove(at);
        }
    }
    if cx.s().effects.iter().any(|e| e.duration == Duration::EndOfTurn) {
        // A permanent that was only a copy until end of turn is what it was again (CR 707.2, 611.2b), as far as the copy
        // effects that are left say: the state undoes a copy effect as it removes it (`GameState::effect_remove`).
        cx.st().effects_retain(|e| e.duration != Duration::EndOfTurn);
        // Control is settled for every permanent: an effect that ended may have decided another's controller too (a copy
        // effect that made a permanent an Aura that gives control), before anything reads it (the untap step's "untap all
        // permanents you control").
        ops::settle_lasting_and_control(cx);
    }
    // Permissions to play cards end with the turn they name (CR 514.2), "this turn" or "your next turn"; one for a
    // card that has since moved is gone with the old object (CR 400.7).
    if !cx.s().permissions.is_empty() {
        let number = cx.s().turn.number;
        let moved: Vec<ObjectId> =
            cx.s().permissions.iter().map(|p| p.card).filter(|&c| cx.s().obj(c).became.is_some()).collect();
        cx.st().permissions_retain(|p| {
            !moved.contains(&p.card)
                && match p.grant.expiry {
                    Expiry::Never => true,
                    Expiry::EndOfTurn => false,
                    Expiry::EndOfYourNextTurn => !(p.player == active && number > p.turn),
                    Expiry::UntilYourNextEndStep | Expiry::WhileControls(_) => true,
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
    // CR 724.1a: abilities that triggered before this began and are not yet on the stack cease to exist. Those that
    // trigger from here on wait for the cleanup step (724.1f).
    while let Some(last) = cx.s().triggers.len().checked_sub(1) {
        cx.st().trigger_remove(last);
    }
    // CR 724.1b: every object on the stack is exiled, the resolving one among them, all at once. An ability on the stack
    // is no card, and is gone; a copy of a spell, exiled, ceases to exist as the state-based actions are checked
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
    // CR 724.1c: state-based actions, with no player given priority and no triggered ability put on the stack.
    crate::settle::state_based_actions_alone(cx)?;
    // CR 724.1d: the step ends once this resolution is over, and the game skips straight to the cleanup step.
    cx.st().update_turn(|t| t.ending = true);
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
    // CR 724.1c-d: an effect has ended the turn. No player gets priority again in this step, and the abilities that
    // triggered since wait for the cleanup step (724.1f).
    if cx.s().turn.ending {
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
            if cx.s().stack.is_empty() {
                return Ok(Flow::Done);
            }
            cx.st().frame_push(Frame::new(FrameKind::Resolve));
            Ok(Flow::Again)
        }
    }
}

#[allow(dead_code)]
fn _unused(cx: &Cx) -> bool {
    layers::live(cx.s(), ObjectId(0))
}
