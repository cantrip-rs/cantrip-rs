//! Speculative resolution (CR 608.2): what a spell or ability on the stack would do if it resolved now, as it is and
//! wherever it is on the stack (`Query::speculate_resolution`). Equinox's "counter target spell if it would destroy a land
//! you control" asks it; its rulings ask whether the spell would destroy one "if it resolved right then", whatever choices
//! or chances its resolution meets.
//!
//! The object is resolved, by the engine's own runs (`Game::run`, `Game::answer`), on a hidden copy of the game
//! (`Game::hypothetical`): a frame resolving it is put on top of the copy's agenda, over a sentinel frame that waits on
//! a question no one answers, so the copy's run stops once the resolution is done - before state-based actions and before
//! any triggered ability is put on the stack (CR 704.3, 603.3), which is where the resolution ends - and never goes on to
//! the rest of the game. Every question the resolution asks branches the search: each legal answer
//! (`actions::answers_to_try`, each checked by `validate::check`) is a branch, the answers given so far replayed the way
//! the engine replays a frame's run. Every chance event branches it too: a die, a coin, a random choice gives each outcome
//! in turn (`actions::HypotheticalChance`, which the mana-capability judge shares), and a shuffle gives each order that what
//! the resolution goes on to do could tell apart (`shuffle`). Each branch played to the end of the resolution is an
//! outcome, handed to the visitor; its `Break` ends the search.
//!
//! Nothing of it reaches the game: the copy is the only thing played on and is thrown away, its questions are asked of
//! no one, its records are its own, its chance comes from no generator, and the reads of the past its runs make are
//! counted for the game's run as one (`take_reads`). The cost falls only on a game that asks it: nothing on the engine's
//! paths runs unless a card asks.

use std::ops::ControlFlow;

use cantrip_api::{Event, Speculated, Speculation};
use cantrip_state::*;

use crate::actions::{answers_to_try, HypotheticalChance};
use crate::{Cx, Game};

/// More work than this is not done (KI-164): a run of the resolution and each answer it is given count one, as each answer
/// plays the run again from its start. Past it, the outcomes found stand and the speculation is `Unjudged` unless one broke
/// off; with `CANTRIP_VERIFY` it panics, naming the object, so a scenario reaching it fails.
const MOST_WORK: usize = 1 << 18;

/// A run of a frame, as the chance operations made in it are known by: the frame, where the journal stood as the run began
/// and how many answers it had been given (a run redone after an answer has one more), and whether its frame goes on in
/// stages (a stage goes on from the one before it rather than redoing it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Run {
    frame: usize,
    mark: u32,
    answers: usize,
    staged: bool,
}

impl Run {
    pub(crate) fn of(cx: &Cx) -> Run {
        match cx.g.state.agenda.get(cx.frame) {
            Some(f) => Run { frame: cx.frame, mark: f.mark, answers: f.answers.len(), staged: f.staged },
            None => Run { frame: cx.frame, mark: 0, answers: 0, staged: false },
        }
    }
}

/// A chance operation, or a shuffle, by its place: its frame, its place among the operations of that kind in the frame's run,
/// and whether it is a shuffle.
type Key = (usize, u32, bool);

/// Where a frame's run has got to: the operations it has made so far.
#[derive(Clone, Debug)]
struct Counted {
    run: Run,
    calls: u32,
    shuffles: u32,
}

/// A shuffle made in a branch: the order it left, the history's length then, how many answers the branch had been given and
/// how many chance outcomes chosen, and the places in its order the branch chose (the rest left as they were).
#[derive(Clone, Debug)]
struct Shuffled {
    key: Key,
    order: Vec<ObjectId>,
    log_at: usize,
    fed: usize,
    route_at: usize,
    chosen: Vec<u32>,
}

/// What a speculative resolution's chance keeps beside its outcomes (`HypotheticalChance::probing`).
#[derive(Clone, Debug, Default)]
pub(crate) struct Probing {
    /// The operations met, in the order first met: the outcome of the nth is the route's nth.
    keys: Vec<Key>,
    counted: Vec<Counted>,
    /// How many answers the branch had been given when the run under way began.
    pub(crate) fed: usize,
    /// The places of each shuffle's order this branch chooses (`shuffle`).
    places: Vec<(Key, Vec<u32>)>,
    shuffled: Vec<Shuffled>,
}

impl Probing {
    fn counted(&mut self, run: Run) -> &mut Counted {
        let at = match self.counted.iter().position(|c| c.run.frame == run.frame) {
            Some(at) => at,
            None => {
                self.counted.push(Counted { run, calls: 0, shuffles: 0 });
                self.counted.len() - 1
            }
        };
        let counted = &mut self.counted[at];
        if counted.run != run {
            // A run redone from its start meets its operations again; a stage goes on from the stage before.
            if !run.staged {
                counted.calls = 0;
                counted.shuffles = 0;
            }
            counted.run = run;
        }
        counted
    }

    /// The next chance operation of `run`.
    pub(crate) fn call_key(&mut self, run: Run) -> Key {
        let counted = self.counted(run);
        let key = (run.frame, counted.calls, false);
        counted.calls += 1;
        key
    }

    /// Where `key` was first met, if it was; otherwise it is met now, the newest.
    pub(crate) fn met(&mut self, key: Key) -> Option<usize> {
        let at = self.keys.iter().position(|&k| k == key);
        if at.is_none() {
            self.keys.push(key);
        }
        at
    }
}

/// A shuffle in a speculative resolution (CR 701.24a): the places of the new order that what the resolution goes on to do
/// tells apart are chosen, each among every card not yet placed (`HypotheticalChance`), and the others keep the cards
/// left in the order they had. Which places those are is found as the branch goes: a record after the shuffle that names a
/// card of it, or the question asked at the end of the branch naming one, reads its place (a draw, a reveal, a look, a
/// card moved out of the library: every way of learning a hidden card is an event, docs/engine-design.md, "Visibility is
/// derived"); a branch that read a place it did not choose is played again choosing it (`unchosen`). A search, shown the
/// whole library at once, reads no place. So the outcomes are every order, each told apart from the others only by what
/// was read of it.
pub(crate) fn shuffle(cx: &mut Cx, zone: Zone) {
    let cards = cx.s().zone(zone).to_vec();
    let run = Run::of(cx);
    let Some(chance) = cx.g.hypothetical_chance.as_mut() else { return };
    let route_at = chance.chosen_len();
    let Some(probing) = chance.probing.as_mut() else { return };
    let counted = probing.counted(run);
    let key = (run.frame, counted.shuffles, true);
    counted.shuffles += 1;
    let fed = probing.fed;
    let mut chosen: Vec<u32> = probing.places.iter().find(|(k, _)| *k == key).map(|(_, p)| p.clone()).unwrap_or_default();
    chosen.retain(|&p| (p as usize) < cards.len());
    let mut left = cards.clone();
    let mut order: Vec<Option<ObjectId>> = vec![None; cards.len()];
    for &place in &chosen {
        let pick = cx.random_below(count_u32(left.len())) as usize;
        if pick < left.len() {
            order[place as usize] = Some(left.remove(pick));
        }
    }
    let mut left = left.into_iter();
    let order: Vec<ObjectId> = order.into_iter().filter_map(|o| o.or_else(|| left.next())).collect();
    for &id in &cards {
        cx.st().zone_remove(zone, id);
    }
    for &id in &order {
        cx.st().zone_push(zone, id);
    }
    let log_at = cx.g.log.len();
    let Some(probing) = cx.g.hypothetical_chance.as_mut().and_then(|c| c.probing.as_mut()) else { return };
    // A run redone makes the shuffle again: where it was first made is where a branch choosing more of it begins.
    match probing.shuffled.iter_mut().find(|s| s.key == key) {
        Some(s) => {
            s.order = order;
            s.log_at = log_at;
        }
        None => probing.shuffled.push(Shuffled { key, order, log_at, fed, route_at, chosen }),
    }
}

/// Whether a speculative resolution is under way in which nothing can read the order of a graveyard (CR 404.3): the rules
/// let the owner of cards put into a graveyard together arrange them only so that what reads that order can be served
/// (`ops::arrange_graveyards`), and what reads it - Corpse Dance's "top creature card", Nether Shadow's "three or more
/// creature cards above it" - reads it as its own spell resolves or as its own triggered ability does, from the graveyard.
/// A speculation runs no trigger and resolves only its object, so unless that object reads the order, every arrangement
/// goes the same way to the end of the resolution: they are one outcome, its cards in the order they arrived, as the
/// shuffles the resolution goes on without reading are (`shuffle`).
pub(crate) fn graveyard_order_unread(cx: &Cx) -> bool {
    if !cx.g.hypothetical_chance.as_ref().is_some_and(|c| c.probing.is_some()) {
        return false;
    }
    let cards = cx.cards();
    let reads = |o: &Obj| cards.get(o.card).reads_graveyard_order || o.copy.is_some_and(|c| cards.get(c.card).reads_graveyard_order);
    let resolving = cx.g.state.agenda.iter().filter_map(|f| match f.kind { FrameKind::Resolve { serial } => Some(serial), _ => None }).last();
    let object = resolving.and_then(|serial| cx.s().stack.iter().chain(&cx.s().left_stack).find(|i| i.serial == serial)).map(|i| i.object);
    !object.is_some_and(|id| reads(cx.s().obj(id)))
}

/// The first shuffle of the branch whose order was read at a place it did not choose (`shuffle`): how many answers and
/// chance outcomes the branch had then, the shuffle, and every place it is to choose.
fn unchosen(copy: &Game, probing: &Probing) -> Option<(usize, usize, Key, Vec<u32>)> {
    let mut asked: Vec<ObjectId> = Vec::new();
    if let Some(request) = copy.state.agenda.last().and_then(|f| f.pending.as_ref()) {
        request.kind.objects(&mut |id| asked.push(id));
    }
    for s in &probing.shuffled {
        if s.log_at > copy.log.len() {
            continue;
        }
        let mut read: Vec<u32> = Vec::new();
        let records = &copy.log[s.log_at..];
        let mut i = 0;
        while i < records.len() {
            // What a search shows is the whole library at once: no place of it.
            if let Event::Shown { player, .. } = records[i].event {
                let shown: Vec<ObjectId> = records[i..].iter()
                    .map_while(|r| match r.event { Event::Shown { player: p, card } if p == player => Some(card), _ => None })
                    .collect();
                if s.order.iter().all(|c| shown.contains(c)) {
                    i += shown.len();
                    continue;
                }
            }
            for (place, &card) in s.order.iter().enumerate() {
                if records[i].event.is_about(card) {
                    read.push(count_u32(place));
                }
            }
            i += 1;
        }
        if !s.order.iter().all(|c| asked.contains(c)) {
            for (place, card) in s.order.iter().enumerate() {
                if asked.contains(card) {
                    read.push(count_u32(place));
                }
            }
        }
        let mut places = s.chosen.clone();
        let before = places.len();
        for place in read {
            if !places.contains(&place) {
                places.push(place);
            }
        }
        if places.len() > before {
            places.sort_unstable();
            return Some((s.fed, s.route_at, s.key, places));
        }
    }
    None
}

/// One way of playing the resolution: the answers given in order, the chance outcomes in the order their operations are
/// met, and the places of each shuffle chosen.
#[derive(Clone, Debug, Default)]
struct Branch {
    answers: Vec<(PlayerId, Answer)>,
    route: Vec<u32>,
    places: Vec<(Key, Vec<u32>)>,
}

/// `Query::speculate_resolution` of a game: see the module.
pub(crate) fn speculate(game: &Game, item: Target, settled: &dyn Fn(&Speculated<'_>) -> bool, visit: &mut dyn FnMut(&Speculated<'_>) -> ControlFlow<()>) -> Speculation<()> {
    let serial = match item {
        Target::Object(id) => game.state.stack.iter().find(|i| i.kind == StackKind::Spell && i.object == id).map(|i| i.serial),
        Target::Ability(serial) => game.state.stack.iter().find(|i| i.serial == serial && i.kind != StackKind::Spell).map(|i| i.serial),
        _ => None,
    };
    let Some(serial) = serial else { return Speculation::NotOnStack };
    // The copy's runs count their reads of the past afresh (`Game::run`); the game's run under way keeps its own, and the
    // speculation is one more: what it answers may have asked the past.
    let reads = cantrip_api::history::take_reads();
    let came = explore(game, serial, settled, visit);
    cantrip_api::history::take_reads();
    for _ in 0..=reads {
        cantrip_api::history::note_read();
    }
    came
}

fn explore(game: &Game, serial: u32, settled: &dyn Fn(&Speculated<'_>) -> bool, visit: &mut dyn FnMut(&Speculated<'_>) -> ControlFlow<()>) -> Speculation<()> {
    let mut copy = game.hypothetical();
    // The copy is played back to here before each branch: nothing it does is forgotten from its journal.
    copy.state.journal.holds += 1;
    let base = copy.state.mark();
    let base_log = copy.log.len();
    let sentinel = copy.state.agenda.len();
    let gap = |why: String| {
        if crate::layers::verifying() {
            panic!("what the stack object {serial} would do if it resolved (CR 608.2) can't be judged: {why}");
        }
    };
    // Payments already met: the answers given before the payment began, and the position, with the question it waits on.
    let mut met: std::collections::HashSet<(u64, u64)> = std::collections::HashSet::new();
    let mut waiting = vec![Branch::default()];
    let mut work = 0usize;
    while let Some(branch) = waiting.pop() {
        work += 1 + branch.answers.len();
        if work > MOST_WORK {
            gap(format!("more than {MOST_WORK} runs and answers"));
            return Speculation::Unjudged;
        }
        copy.state.rollback(base);
        copy.log.truncate(base_log);
        copy.tree = crate::tree::Tree::new();
        copy.at = crate::tree::NodeId::ROOT;
        // The sentinel waits on a question no one answers: the copy's run stops there once the resolution is done.
        let mut stop = Frame::new(FrameKind::Settle);
        let id = copy.state.request_id();
        stop.pending = Some(Request { id, player: PlayerId(0), kind: RequestKind::May { source: ObjectId::NONE, purpose: Purpose::Band }, may_cancel: false });
        copy.state.frame_push(stop);
        copy.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
        copy.hypothetical_chance = Some(HypotheticalChance::probing(branch.route.clone(), Probing { places: branch.places.clone(), ..Probing::default() }));
        copy.run();
        let mut given: Vec<(PlayerId, Answer, Option<RequestKind>)> = Vec::new();
        let mut lost = false;
        for (player, answer) in &branch.answers {
            let Some(request) = copy.state.agenda.last().and_then(|f| f.pending.clone()) else { lost = true; break };
            if copy.state.agenda.len() <= sentinel + 1 || request.player != *player {
                lost = true;
                break;
            }
            if let Some(probing) = copy.hypothetical_chance.as_mut().and_then(|c| c.probing.as_mut()) {
                probing.fed = given.len() + 1;
            }
            if copy.answer(request.id, *player, answer.clone()).is_err() {
                lost = true;
                break;
            }
            given.push((*player, answer.clone(), Some(request.kind)));
        }
        if lost {
            // A replay that does not meet the questions it met before can't be: the engine's runs are deterministic.
            gap("a branch replayed differently".to_string());
            return Speculation::Unjudged;
        }
        let Some(chance) = copy.hypothetical_chance.take() else { continue };
        let Some(probing) = chance.probing.as_deref() else { continue };
        // A shuffle read where the branch chose nothing: the branch is played again choosing it, and what it found since
        // the shuffle is dropped.
        if let Some((fed, route_at, key, places)) = unchosen(&copy, probing) {
            for (used, route) in chance.alternatives.iter().filter(|(_, r)| r.len() <= route_at) {
                waiting.push(Branch { answers: branch.answers[..(*used).min(branch.answers.len())].to_vec(), route: route.clone(), places: branch.places.clone() });
            }
            let mut again = branch.places.clone();
            again.retain(|(k, _)| *k != key);
            again.push((key, places));
            waiting.push(Branch {
                answers: branch.answers[..fed.min(branch.answers.len())].to_vec(),
                route: chance.chosen()[..route_at.min(chance.chosen_len())].to_vec(),
                places: again,
            });
            continue;
        }
        for (used, route) in &chance.alternatives {
            waiting.push(Branch { answers: branch.answers[..(*used).min(branch.answers.len())].to_vec(), route: route.clone(), places: branch.places.clone() });
        }
        let done = copy.state.is_over() || copy.state.agenda.len() == sentinel + 1;
        let outcome = Speculated { game: &copy, records: &copy.log[base_log..], ended: done };
        if done || settled(&outcome) {
            if visit(&outcome).is_break() {
                return Speculation::Decided(());
            }
            continue;
        }
        let Some(request) = copy.state.agenda.last().and_then(|f| f.pending.clone()) else {
            gap("the resolution stopped without a question".to_string());
            return Speculation::Unjudged;
        };
        // A payment met again in the same position, after the same answers before it began, goes the same way from here:
        // the order of mana abilities activated, an X raised and lowered again, an activation taken back (CR 605.3a, 733.1).
        if let RequestKind::UnlessPays { source, subject, question, .. } = &request.kind {
            let same = |kind: &Option<RequestKind>| matches!(kind, Some(RequestKind::UnlessPays { source: s, subject: t, question: q, .. })
                if s == source && t == subject && std::ptr::eq(*q, *question));
            let began = given.iter().rposition(|(_, _, kind)| !same(kind)).map_or(0, |at| at + 1);
            // The chance outcomes met so far are replayed by every run from here, as the answers before the payment are.
            let before = hash_of(&(&branch.answers[..began], chance.chosen()));
            // The position without the answers the payment took to reach it, its pool as the set it is (CR 106.4: a pool
            // has no order; which of its mana pays is the payer's to say wherever the rules tell it apart, `mana_told_apart`).
            let top = copy.state.agenda.len() - 1;
            let answers = std::mem::take(&mut copy.state.agenda[top].answers);
            let pool = std::mem::take(&mut copy.state.mana_from);
            let mut units: Vec<u64> = pool.iter().map(hash_of).collect();
            units.sort_unstable();
            let position = hash_of(&(&copy.state, units));
            copy.state.mana_from = pool;
            copy.state.agenda[top].answers = answers;
            if !met.insert((before, position)) {
                continue;
            }
        }
        let budget = MOST_WORK.saturating_sub(work + waiting.len());
        let mut tries = match answers_to_try(copy.cards, &request.kind, true, budget) {
            Ok((tries, all)) => {
                if !all {
                    gap(format!("more than {MOST_WORK} answers to try"));
                    return Speculation::Unjudged;
                }
                tries
            }
            Err(why) => {
                gap(why);
                return Speculation::Unjudged;
            }
        };
        if request.may_cancel {
            tries.push(Answer::Cancel);
        }
        // Tried first is the first offered: pushed last.
        for answer in tries.into_iter().rev() {
            if crate::validate::check(&copy, &request, &answer).is_err() {
                continue;
            }
            let mut answers = branch.answers.clone();
            answers.push((request.player, answer));
            waiting.push(Branch { answers, route: chance.chosen().to_vec(), places: branch.places.clone() });
        }
    }
    Speculation::Undecided
}

fn hash_of<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
