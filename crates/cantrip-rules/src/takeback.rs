//! Taking back a cast or an activation (CR 733.1) without undoing what may not be undone.
//!
//! An illegal action - a cast or an activation given up part way, or one whose payment cannot be completed - is reversed
//! whole, and so may be the mana abilities its player activated while making it, "unless mana from those abilities or from
//! any triggered mana abilities they caused to trigger was spent on another mana ability that wasn't reversed. Players may
//! not reverse actions that moved cards to a library, moved cards from a library to any zone other than the stack, caused a
//! library to be shuffled, or caused cards from a library to be revealed" (CR 733.1). With the owner's ruling beside it: an
//! undo never reveals information, so a mana ability that made hidden information known - a card drawn, a card leaving a
//! hand, a library or face-down exile (Elvish Spirit Guide's exile, Lion's Eye Diamond's discard, Chromatic Sphere's draw),
//! a reveal, a look, a search - is not reversed either. One that only moved a card into a hidden zone may be.
//!
//! Every write of the action is in the journal, and the journal undoes in order only: it cannot take the action out from
//! under a mana ability that stays. So the action is reverted whole (`GameState::revert`), as before, and each mana ability
//! that stays is activated again, as it was: the same ability of the same source, with the answers its player gave it then
//! and paid with exactly the mana it was paid with (`Step`), while the cards the action itself had moved out of their zones
//! (the spell, from the hand or the top of a library) are set aside, so that a draw draws the card it drew and a discard of
//! the hand discards the cards it discarded. What it did is then the game's again: the events in the history, the triggers
//! waiting, the mana in the pool, all of it journalled as any run's writes are, so the game-log tree moves through it as
//! through anything else.
//!
//! Which mana abilities stay is read from what they did, never from the card: each activation's events in the history
//! (`reveals`), and the mana that paid for the ones that stay (`Step::Spend`).

use std::collections::VecDeque;

use cantrip_api::*;
use cantrip_state::{ManaPick, Mark};

use crate::{Cx, Game};

/// One activation of a mana ability: what `actions::activate_mana` was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Activation {
    pub player: PlayerId,
    pub source: ObjectId,
    pub index: u8,
    pub color: Option<Color>,
    pub colorless: bool,
    pub targets: Vec<Target>,
    /// The player-granting effect as it was selected, not its transient position among a player's grants.
    pub player_grant: Option<ContinuousEffect>,
}

/// One thing a mana ability's payment of its own mana cost did (CR 605.3a, 601.2g-h), in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Another mana ability was activated to pay it.
    Activate(Activation),
    /// Mana of the pool was spent on it: `n` of `mana` that `source` produced, of this restriction or none.
    Spend { restriction: Option<ManaRestriction>, mana: Mana, n: u32, source: ObjectId, snow: bool, rider: u32 },
}

/// A mana ability's activation as the run made it.
#[derive(Clone, Debug)]
struct Run {
    act: Activation,
    /// The activation under way when it began: the mana ability it was activated to pay for.
    parent: Option<usize>,
    /// Its events: the history from its start to its end.
    log: (usize, usize),
    /// The answers it was given (`Trail::asks`).
    asks: (usize, usize),
    /// The payments of its own mana cost and of those of the mana abilities activated within it (`Trail::pays`).
    pays: (usize, usize),
    /// The objects made while it was under way: the ids from the first to the one past the last.
    objects: (usize, usize),
    /// The "when that mana is spent" riders made while it was under way (`Trail::riders`).
    riders: (usize, usize),
    /// The sources of the mana added while it was the innermost activation (CR 106.3): its own, and that of the triggered
    /// mana abilities it triggered (CR 605.1b, 605.4a).
    produced: Vec<ObjectId>,
    /// It was activated; one still under way when the action was given up is part of what is given up.
    done: bool,
}

/// What a frame's run has done with mana abilities, for taking it back: every activation, the answers given while one was
/// under way, and the payments of their own mana costs. Bookkeeping of a run, not of the game: cleared as each run begins,
/// never part of a position.
#[derive(Clone, Debug, Default)]
pub(crate) struct Trail {
    runs: Vec<Run>,
    /// Each answer taken while a mana ability was being activated, and whether a payment's own question took it.
    asks: Vec<(Answer, bool)>,
    /// Each mana ability's payment of its own mana cost, in the order begun.
    pays: Vec<Vec<Step>>,
    /// The number of each "when that mana is spent" delayed ability a mana ability made while one was under way
    /// (`DelayedWhen::ThatMana`, `ManaFrom::rider`), in the order made.
    riders: Vec<u32>,
    /// The activations under way, innermost last.
    open: Vec<usize>,
    /// The payments under way, innermost last.
    paying: Vec<usize>,
}

/// Where the trail stood: what an action begun then has added since.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Since {
    runs: usize,
    asks: usize,
    pays: usize,
    riders: usize,
    /// How many objects the game had.
    objects: usize,
}

impl Since {
    /// A frame's run beginning, with the trail cleared and the game holding this many objects.
    pub(crate) fn frame(objects: usize) -> Since {
        Since { objects, ..Since::default() }
    }
}

/// What a replay of the mana abilities that stay is given: their answers that were not a payment's own, and their payments.
#[derive(Clone, Debug, Default)]
pub(crate) struct Script {
    asks: VecDeque<Answer>,
    pays: VecDeque<Vec<Step>>,
    /// The numbers of the riders they made, made again under the same numbers (`rider`): the mana they paid with is named
    /// by them (`Step::Spend`), as the objects they made keep their ids.
    riders: VecDeque<u32>,
    /// The replay asked something the script has no answer to, or could not spend what it says.
    failed: bool,
}

/// The game's take-back bookkeeping (`Game::takeback`): the trail of the run under way, and the script while a replay runs.
#[derive(Debug, Default)]
pub(crate) struct Takeback {
    trail: Trail,
    script: Option<Script>,
}

/// Bookkeeping of a run: a copy of a game begins with none.
impl Clone for Takeback {
    fn clone(&self) -> Takeback {
        Takeback::default()
    }
}

impl Takeback {
    /// A frame's run begins: nothing of an earlier run is taken back with it.
    pub(crate) fn reset(&mut self) {
        let t = &mut self.trail;
        t.runs.clear();
        t.asks.clear();
        t.pays.clear();
        t.riders.clear();
        t.open.clear();
        t.paying.clear();
    }
}

/// Where the trail stands now.
pub(crate) fn since(g: &Game) -> Since {
    let t = &g.takeback.trail;
    Since { runs: t.runs.len(), asks: t.asks.len(), pays: t.pays.len(), riders: t.riders.len(), objects: g.state.objects.len() }
}

/// A mana ability's activation begins.
pub(crate) fn begin(g: &mut Game, act: Activation) -> usize {
    let (log, objects) = (g.log.len(), g.state.objects.len());
    let t = &mut g.takeback.trail;
    let run = t.runs.len();
    t.runs.push(Run {
        act,
        parent: t.open.last().copied(),
        log: (log, log),
        asks: (t.asks.len(), t.asks.len()),
        pays: (t.pays.len(), t.pays.len()),
        objects: (objects, objects),
        riders: (t.riders.len(), t.riders.len()),
        produced: Vec::new(),
        done: false,
    });
    t.open.push(run);
    run
}

/// It ended: activated, or given up.
pub(crate) fn end(g: &mut Game, run: usize, done: bool) {
    let (log, objects) = (g.log.len(), g.state.objects.len());
    let t = &mut g.takeback.trail;
    if let Some(at) = t.open.iter().rposition(|&r| r == run) {
        t.open.truncate(at);
    }
    let (asks, pays, riders) = (t.asks.len(), t.pays.len(), t.riders.len());
    if let Some(r) = t.runs.get_mut(run) {
        r.log.1 = log;
        r.asks.1 = asks;
        r.pays.1 = pays;
        r.riders.1 = riders;
        r.objects.1 = objects;
        r.done = done;
    }
}

/// An answer was taken: kept while a mana ability is being activated.
pub(crate) fn asked(g: &mut Game, answer: &Answer) {
    let t = &mut g.takeback.trail;
    if !t.open.is_empty() {
        t.asks.push((answer.clone(), false));
    }
}

/// The answer just taken was a mana ability's payment's own question, which the replay does not ask (`Step`).
pub(crate) fn asked_paying(g: &mut Game) {
    let t = &mut g.takeback.trail;
    if !t.open.is_empty()
        && let Some(last) = t.asks.last_mut()
    {
        last.1 = true;
    }
}

/// Mana `source` produced was added (CR 106.3).
pub(crate) fn produced(g: &mut Game, source: ObjectId) {
    let t = &mut g.takeback.trail;
    if let Some(&run) = t.open.last() {
        let produced = &mut t.runs[run].produced;
        if !produced.contains(&source) {
            produced.push(source);
        }
    }
}

/// The number of the "when that mana is spent" delayed ability a mana ability being activated makes (`DelayedWhen::ThatMana`,
/// `ManaFrom::rider`): one no waiting delayed ability or mana in a pool has (`GameState::new_mana_rider`), kept for a take-back
/// of the action it is activated during. While a replay runs, the number the activation it replays made, so that what the
/// replay spends names the same mana (`Step::Spend`); one the script lacks, or that is taken, means the replay went off it.
pub(crate) fn rider(g: &mut Game) -> u32 {
    let scripted = g.takeback.script.as_mut().map(|s| s.riders.pop_front());
    let rider = match scripted {
        Some(Some(n)) if n != 0 && !g.state.mana_rider_used(n) => n,
        Some(_) => {
            script_failed(g);
            g.state.new_mana_rider()
        }
        None => g.state.new_mana_rider(),
    };
    let t = &mut g.takeback.trail;
    if !t.open.is_empty() {
        t.riders.push(rider);
    }
    rider
}

/// A mana ability's payment of its own mana cost begins.
pub(crate) fn begin_paying(g: &mut Game) {
    let t = &mut g.takeback.trail;
    t.paying.push(t.pays.len());
    t.pays.push(Vec::new());
}

pub(crate) fn end_paying(g: &mut Game) {
    g.takeback.trail.paying.pop();
}

/// The payment under way did this.
pub(crate) fn step(g: &mut Game, step: Step) {
    let t = &mut g.takeback.trail;
    if let Some(&pay) = t.paying.last() {
        t.pays[pay].push(step);
    }
}

/// While a replay runs: the answer to the next question it asks, or None where the script has none (the replay has gone
/// off what it replays). Outside one, `Err(())`.
pub(crate) fn scripted_answer(g: &mut Game) -> Result<Option<Answer>, ()> {
    let Some(script) = g.takeback.script.as_mut() else { return Err(()) };
    let next = script.asks.pop_front();
    if next.is_none() {
        script.failed = true;
    }
    Ok(next)
}

/// While a replay runs: the steps of the next mana ability payment, or None where the script has none. Outside one,
/// `Err(())`.
pub(crate) fn scripted_payment(g: &mut Game) -> Result<Option<Vec<Step>>, ()> {
    let Some(script) = g.takeback.script.as_mut() else { return Err(()) };
    let next = script.pays.pop_front();
    if next.is_none() {
        script.failed = true;
    }
    Ok(next)
}

/// A replay could not do what its script says.
pub(crate) fn script_failed(g: &mut Game) {
    if let Some(script) = g.takeback.script.as_mut() {
        script.failed = true;
    }
}

/// Whether a replay is running.
pub(crate) fn replaying(g: &Game) -> bool {
    g.takeback.script.is_some()
}

/// Whether an event made hidden information known, which no undo may hide again (the owner's ruling beside CR 733.1): a card
/// left a hidden zone - a library, a hand, outside the game - or left anywhere face down (CR 708.9), a card was moved into a
/// library (CR 733.1), drawn, revealed, looked at, searched for, shown by a question, shuffled into or put at random into a
/// library, scried or surveilled, or turned face up.
fn reveals(g: &Game, event: &Event) -> bool {
    match *event {
        Event::ZoneChange { old, from, to, .. } => {
            from.kind().is_hidden() || to.is(ZoneKind::Library) || (old.index() < g.state.objects.len() && g.state.obj(old).face_down)
        }
        Event::Drew { .. }
        | Event::Revealed { .. }
        | Event::Looked { .. }
        | Event::Searched { .. }
        | Event::Shown { .. }
        | Event::Shuffled { .. }
        | Event::Randomized { .. }
        | Event::Scried { .. }
        | Event::Surveilled { .. }
        | Event::TurnedFaceUp { .. } => true,
        _ => false,
    }
}

/// What a take-back keeps (CR 733.1): the mana abilities activated since `since` that are not reversed, the outermost of them
/// in the order activated, and the cards the action had moved out of their zones before the last of them, to set aside while
/// they are activated again. Read before the action is reverted.
fn kept(g: &Game, log_mark: usize, since: Since) -> (Vec<usize>, Vec<ObjectId>) {
    let t = &g.takeback.trail;
    let runs = &t.runs;
    let first = since.runs;
    if runs.len() <= first {
        return (Vec::new(), Vec::new());
    }
    let end = g.log.len();
    // Each event's innermost activation among the action's: the one begun last whose span holds it. Activations nest, and
    // are in the order begun.
    let mut owners: Vec<Option<usize>> = vec![None; end.saturating_sub(log_mark)];
    for (r, run) in runs.iter().enumerate().skip(first) {
        let (from, to) = run.log;
        for slot in owners.iter_mut().take(to.min(end).saturating_sub(log_mark)).skip(from.saturating_sub(log_mark)) {
            *slot = Some(r);
        }
    }
    let owner = &owners;
    let own = |r: usize| (log_mark..end).filter(move |&i| owner[i - log_mark] == Some(r));
    let mut keep: Vec<bool> = (0..runs.len()).map(|r| r >= first && runs[r].done && own(r).any(|i| reveals(g, &g.log[i].event))).collect();
    if !keep.iter().any(|&k| k) {
        return (Vec::new(), Vec::new());
    }
    loop {
        let mut changed = false;
        for r in first..runs.len() {
            if !keep[r] {
                continue;
            }
            // What was activated within it is part of it, activated again with it: its spans hold theirs. The mana abilities
            // whose mana paid for it, or for one activated within it, are not reversed either.
            for steps in &t.pays[runs[r].pays.0..runs[r].pays.1] {
                for step in steps {
                    let Step::Spend { source, .. } = *step else { continue };
                    for x in first..r {
                        if !keep[x] && runs[x].done && (runs[x].act.source == source || runs[x].produced.contains(&source)) {
                            keep[x] = true;
                            changed = true;
                        }
                    }
                }
            }
            // Nor one that made an object it, or one within it, went on to move: what it moved is that ability's doing.
            for i in runs[r].log.0.max(log_mark)..runs[r].log.1.min(end) {
                let Event::ZoneChange { old, .. } = g.log[i].event else { continue };
                for x in first..r {
                    if keep[x] || !runs[x].done {
                        continue;
                    }
                    if own(x).any(|j| matches!(g.log[j].event, Event::ZoneChange { new, .. } if new == old)) {
                        keep[x] = true;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    // The kept ones nothing kept is around: one within a kept one, however deep and whatever is between, is activated again
    // as part of it (its steps hold every activation within it), so never again on its own.
    let under_kept = |r: usize| {
        let mut at = runs[r].parent;
        while let Some(p) = at.filter(|&p| p >= first) {
            if keep[p] {
                return true;
            }
            at = runs[p].parent;
        }
        false
    };
    let outermost: Vec<usize> = (first..runs.len()).filter(|&r| keep[r] && !under_kept(r)).collect();
    // The cards the action itself moved before the last mana ability kept began: the spell from where it was cast, say.
    let last = outermost.last().map_or(end, |&r| runs[r].log.0);
    let aside: Vec<ObjectId> = (log_mark..last)
        .filter(|&i| owner[i - log_mark].is_none_or(|r| !runs[r].done))
        .filter_map(|i| match g.log[i].event {
            Event::ZoneChange { old, .. } => Some(old),
            _ => None,
        })
        .collect();
    (outermost, aside)
}

/// The answers and payments the outermost kept activations replay, theirs and those within them, in order.
fn script(t: &Trail, outermost: &[usize]) -> Script {
    let mut script = Script::default();
    for &r in outermost {
        let run = &t.runs[r];
        script.asks.extend(t.asks[run.asks.0..run.asks.1].iter().filter(|(_, paying)| !paying).map(|(a, _)| a.clone()));
        script.pays.extend(t.pays[run.pays.0..run.pays.1].iter().cloned());
        script.riders.extend(t.riders[run.riders.0..run.riders.1].iter().copied());
    }
    script
}

/// Takes back the action begun at `mark` (the journal), `log_mark` (the history) and `since` (the trail): reverts everything
/// it did (CR 733.1), then activates again each mana ability activated during it that may not be reversed, with what it was
/// given and paid then. Every write is journalled.
pub(crate) fn take_back(cx: &mut Cx, mark: Mark, log_mark: usize, since: Since) {
    let (outermost, aside) = kept(cx.g, log_mark, since);
    let script = if outermost.is_empty() { None } else { Some(script(&cx.g.takeback.trail, &outermost)) };
    let runs: Vec<(Activation, (usize, usize))> = outermost.iter().map(|&r| (cx.g.takeback.trail.runs[r].act.clone(), cx.g.takeback.trail.runs[r].objects)).collect();
    // What each did, event by event, for the replay to be held to.
    let original_receipts = if outermost.is_empty() { Vec::new() } else { cx.s().cost_receipts.clone() };
    let did: Vec<Vec<Event>> = outermost
        .iter()
        .map(|&r| {
            let (from, to) = cx.g.takeback.trail.runs[r].log;
            cx.g.log[from.min(cx.g.log.len())..to.min(cx.g.log.len())].iter().map(|rec| rec.event).collect()
        })
        .collect();
    // The objects the action made before each kept ability did, by card and owner: they stand in, gone, for what the action
    // made, so that the replay makes each object under the id it had (what a later kept ability names - a card drawn, then
    // exiled from the hand for its own mana - is the same object).
    let made: Vec<(CardId, PlayerId)> = cx.s().objects[since.objects.min(cx.s().objects.len())..].iter().map(|o| (o.card, o.owner)).collect();
    cx.st().revert(mark);
    cx.g.log.truncate(log_mark);
    truncate(cx.g, since);
    let Some(script) = script else { return };
    let replay_mark = cx.s().mark();
    // The cards the action moved out of their zones are out of them again while the kept abilities are activated, each where
    // it was: the spell cast from the top of a library is not the card a draw then draws, nor a card in the hand discarded.
    let mut set_aside: Vec<(Zone, usize, ObjectId)> = Vec::new();
    for id in aside {
        if id.index() >= cx.s().objects.len() || set_aside.iter().any(|&(_, _, o)| o == id) {
            continue;
        }
        let zone = cx.s().obj(id).zone;
        if let Some(at) = cx.s().zone(zone).iter().position(|&o| o == id) {
            cx.st().zone_remove(zone, id);
            set_aside.push((zone, at, id));
        }
    }
    cx.g.takeback.script = Some(script);
    let mut ok = true;
    for ((act, objects), did) in runs.iter().zip(&did) {
        while cx.s().objects.len() < objects.0 {
            let Some(&(card, owner)) = made.get(cx.s().objects.len() - since.objects) else { break };
            cx.st().push_obj(Obj::new(card, owner, Zone::GONE));
        }
        let from = cx.g.log.len();
        ok = cx.s().objects.len() == objects.0
            && crate::actions::activate_recorded_mana(cx, act).is_ok()
            && cx.s().objects.len() == objects.1
            && same_events(&cx.g.log[from..], did, &cx.s().cost_receipts, &original_receipts);
        if !ok {
            break;
        }
    }
    let script = cx.g.takeback.script.take().unwrap_or_default();
    ok &= !script.failed && script.asks.is_empty() && script.pays.is_empty() && script.riders.is_empty();
    for (zone, at, id) in set_aside.into_iter().rev() {
        let at = at.min(cx.s().zone(zone).len());
        cx.st().zone_insert(zone, at, id);
    }
    // The replay goes off what it replays - another card drawn, another card moved, or nothing done - only where a kept
    // mana ability reads something of the action other than where the cards it moved were (the stack, a choice made for the
    // spell), and no printed mana ability that reveals anything reads such a thing: each draws, discards or exiles from a
    // hidden zone (Chromatic Sphere, the Spirit Guides) whatever the stack holds. Were one to, no outcome would reveal
    // nothing: what the replay did differs from what was seen, and reverting whole puts seen cards back. The action is then
    // reverted whole, as the rules have it with no mana ability kept, rather than left with a replay that did something no
    // one did; the assertion makes the tests and the corpus fail on it.
    debug_assert!(ok, "a mana ability kept through a take-back (CR 733.1) did not activate again as it had");
    if !ok {
        cx.st().revert(replay_mark);
        cx.g.log.truncate(log_mark);
        truncate(cx.g, since);
    }
}

/// Whether a replay did what the activation did, event by event: the same objects (the ids are the same, `take_back`'s
/// stand-ins see to it), the same cards drawn, moved, revealed. A last-known row's place (`Entered::known`,
/// `Attacks::known`) is not what happened, and the action's own rows came before it.
fn same_events(replayed: &[Record], did: &[Event], new_receipts: &[cantrip_state::CostReceiptDetails], old_receipts: &[cantrip_state::CostReceiptDetails]) -> bool {
    fn plain(e: Event) -> Event {
        match e {
            Event::Entered { permanent, controller, types, .. } => Event::Entered { permanent, controller, types, known: 0 },
            Event::Attacks { attacker, defender, defending, .. } => Event::Attacks { attacker, defender, defending, known: 0 },
            e => e,
        }
    }
    if replayed.len() != did.len() { return false; }
    // Resolution payment ids are journal positions: removing an earlier reversible activation shifts them on replay.
    // Match each new id to its original and check all of its receipts against the same mapping.
    let mut payments = Vec::new();
    replayed.iter().zip(did).all(|(r, &e)| match (r.event, e) {
        (Event::CombatPaymentBegan { payment: new, payer: a }, Event::CombatPaymentBegan { payment: old, payer: b }) => {
            if a != b || payments.iter().any(|&(n, o)| n == new || o == old) { return false; }
            payments.push((new, old));
            true
        }
        (Event::CombatCostObjectPaid { payment: new, source: a, attacker: b, payer: c, object: d, receipt: e },
         Event::CombatCostObjectPaid { payment: old, source: f, attacker: g, payer: h, object: i, receipt: j }) => {
            payments.contains(&(new, old)) && (a, b, c, d) == (f, g, h, i)
                && new_receipts.get(e as usize).zip(old_receipts.get(j as usize)).is_some_and(|(x, y)| x == y)
        }
        (Event::ResolutionPaymentBegan { payment: new, source: a, payer: b, resolving: ra }, Event::ResolutionPaymentBegan { payment: old, source: c, payer: d, resolving: rb }) => {
            if a != c || b != d || ra != rb || payments.iter().any(|&(n, o)| n == new || o == old) { return false; }
            payments.push((new, old));
            true
        }
        (Event::ResolutionCostObjectPaid { payment: new, source: a, payer: b, object: c, receipt: d },
         Event::ResolutionCostObjectPaid { payment: old, source: f, payer: g, object: h, receipt: i }) => {
            payments.contains(&(new, old)) && (a, b, c) == (f, g, h)
                && new_receipts.get(d as usize).zip(old_receipts.get(i as usize)).is_some_and(|(x, y)| x == y)
        }
        (Event::CostObjectPaid { spell: a, object: b, receipt: c }, Event::CostObjectPaid { spell: d, object: e, receipt: f }) => {
            (a, b) == (d, e) && new_receipts.get(c as usize).zip(old_receipts.get(f as usize)).is_some_and(|(x, y)| x == y)
        }
        (Event::AbilityCostObjectPaid { serial: a, object: b, receipt: c }, Event::AbilityCostObjectPaid { serial: d, object: e, receipt: f }) => {
            (a, b) == (d, e) && new_receipts.get(c as usize).zip(old_receipts.get(f as usize)).is_some_and(|(x, y)| x == y)
        }
        (new, old) => plain(new) == plain(old),
    })
}

#[cfg(test)]
mod receipt_rebinding_tests {
    use super::*;

    #[test]
    fn replay_compares_receipt_values_when_arena_indices_change() {
        let detail = cantrip_state::CostReceiptDetails { origin: Some(CostOrigin::AdditionalChoice(700)), component: 900, part: CostPart::Discard(1000) };
        let different = cantrip_state::CostReceiptDetails { part: CostPart::Discard(1001), ..detail };
        let event = Event::CostObjectPaid { spell: ObjectId(1), object: ObjectId(2), receipt: 0 };
        let record = Record { serial: 0, turn: 1, step: cantrip_state::Step::Upkeep, active: PlayerId(0), cause: cantrip_api::Cause::Rules,
            event: Event::CostObjectPaid { spell: ObjectId(1), object: ObjectId(2), receipt: 1 } };
        assert!(same_events(&[record], &[event], &[different, detail], &[detail]));
        assert!(!same_events(&[record], &[event], &[detail, different], &[detail]));
    }
}

fn truncate(g: &mut Game, since: Since) {
    let t = &mut g.takeback.trail;
    t.runs.truncate(since.runs);
    t.asks.truncate(since.asks);
    t.pays.truncate(since.pays);
    t.riders.truncate(since.riders);
    t.open.retain(|&r| r < since.runs);
    t.paying.retain(|&p| p < since.pays);
}

/// Spends mana from `player`'s pool for a mana ability's payment of its own mana cost, as `GameState::spend_mana` does,
/// recording what it took (`Step::Spend`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spend(
    cx: &mut Cx,
    player: PlayerId,
    restriction: Option<ManaRestriction>,
    mana: Mana,
    n: u32,
    pick: ManaPick,
    taken: &mut Vec<(ObjectId, Mana, u32, bool, u32)>,
    record: bool,
) -> u32 {
    let from = taken.len();
    let got = cx.st().spend_mana(player, restriction, mana, n, pick, taken);
    if record {
        for &(source, mana, n, snow, rider) in &taken[from..] {
            step(cx.g, Step::Spend { restriction, mana, n, source, snow, rider });
        }
    }
    got
}
