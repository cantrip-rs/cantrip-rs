//! The search a required cast is made by (`CastTerms::if_able`: "the player casts it if able", CR 608.2g, 733.1-2).
//!
//! The player must cast the card if some sequence of choices completes the cast, and casts nothing if none does. While the
//! cast is being made its caster may not give it up (`takeback::require`), but may make any choice that is legal; an attempt
//! that can't be completed is reversed whole (CR 733.1) and made again (CR 733.2). Each attempt is a path through the tree of
//! its answers, and an attempt that fails marks its path dead: the same answers made again would fail again, the game being
//! the same. The remake is offered no dead choice, and a question every answer of which is dead is a dead end itself - the
//! attempt is reversed there, marking the path that led to it. So every remake ends at a path no attempt has taken, the tree
//! is finite (an X has only the answers its payment allows, `narrow`), and the search ends: with a cast, or with the
//! root dead, when no choice is left that could complete it and the card can't be cast. Nothing is capped and nothing is
//! guessed: a path is dead only because it was walked and failed, or every way on from it was. The advice that narrows what
//! a required cast offers first (`actions::cost_choices`) only keeps the search short.
//!
//! The search is bookkeeping of a run (`Takeback::search`), redone with the run's answers on every re-run, and read by
//! `Game::answer` to refuse a dead answer the question could not leave out.

use crate::Game;
use cantrip_api::*;

/// One required cast's search: the frame it is in, the path of the attempt under way, and every dead path.
#[derive(Clone, Debug, Default)]
pub(crate) struct Search {
    frame: usize,
    path: Vec<Answer>,
    dead: Vec<Vec<Answer>>,
}

/// A required cast begins in `frame`.
pub(crate) fn begin(g: &mut Game, frame: usize) {
    g.takeback.search.push(Search { frame, ..Search::default() });
}

/// An attempt begins: its path is empty.
pub(crate) fn attempt(g: &mut Game) {
    if let Some(s) = g.takeback.search.last_mut() {
        s.path.clear();
    }
}

/// The attempt under way failed: whether it asked anything, its path being dead from now on. One that asked nothing fails
/// whatever is answered, and the card can't be cast.
pub(crate) fn failed(g: &mut Game) -> bool {
    let Some(s) = g.takeback.search.last_mut() else { return false };
    if s.path.is_empty() {
        return false;
    }
    let path = std::mem::take(&mut s.path);
    s.dead.push(path);
    true
}

/// The required cast is over.
pub(crate) fn end(g: &mut Game) {
    g.takeback.search.pop();
}

/// The search a question asked in `frame` is part of, if any.
#[inline]
fn of(g: &Game, frame: usize) -> Option<&Search> {
    g.takeback.search.last().filter(|s| s.frame == frame)
}

/// Whether a question asked in `frame` is one of a required cast's.
#[inline]
pub(crate) fn searching(g: &Game, frame: usize) -> bool {
    of(g, frame).is_some() && !crate::takeback::replaying(g)
}

/// The answer as the search compares it: what was chosen, in no order where the choice has none (targets, modes, cards),
/// an X as the number announced however it was reached, an action as what it does. None for a step that chooses nothing yet
/// (raising or lowering an X before it is announced).
fn canonical(kind: &RequestKind, answer: &Answer) -> Option<Answer> {
    let sorted = |v: &mut Vec<Target>| v.sort_by_key(|t| format!("{t:?}"));
    Some(match answer {
        Answer::Targets(t) => {
            let mut t = t.clone();
            sorted(&mut t);
            Answer::Targets(t)
        }
        Answer::Modes(m) => {
            let mut m = m.clone();
            m.sort_unstable();
            Answer::Modes(m)
        }
        Answer::Cards(c) => {
            let mut c = c.clone();
            c.sort_unstable_by_key(|o| o.0);
            Answer::Cards(c)
        }
        Answer::Divided(d) => {
            let mut d = d.clone();
            d.sort_by_key(|(t, _)| format!("{t:?}"));
            Answer::Divided(d)
        }
        Answer::Act(a) => {
            if let RequestKind::PayCosts { announcing: true, x, .. } = kind {
                match a.kind {
                    ActionKind::IncreaseX | ActionKind::DecreaseX => return None,
                    ActionKind::ConfirmCasting => return Some(Answer::Announce(*x)),
                    _ => {}
                }
            }
            let mut targets = a.targets.clone();
            sorted(&mut targets);
            Answer::Act(Action { granted_by: None, paid_by: None, snow: false, targets, ..a.clone() })
        }
        other => other.clone(),
    })
}

/// Each choice of `least..=most` of `n` items by index, as a set (`again` false) or a multiset, handed to `f` until it says
/// to stop; whether it stopped. Lazy: no list of them is made.
fn each_choice(n: usize, least: usize, most: usize, again: bool, f: &mut dyn FnMut(&[usize]) -> bool) -> bool {
    fn visit(n: usize, least: usize, most: usize, again: bool, from: usize, at: &mut Vec<usize>, f: &mut dyn FnMut(&[usize]) -> bool) -> bool {
        if at.len() >= least && f(at) {
            return true;
        }
        if at.len() < most {
            for i in from..n {
                // A branch with too few items left to reach `least` hands nothing on: not walked.
                if !again && at.len() + (n - i) < least {
                    break;
                }
                at.push(i);
                if visit(n, least, most, again, if again { i } else { i + 1 }, at, f) {
                    return true;
                }
                at.pop();
            }
        }
        false
    }
    let most = if again { most } else { most.min(n) };
    least <= most && visit(n, least, most, again, 0, &mut Vec::new(), f)
}

/// Each subset of the items by index whose `weights` add up to `minimum` or more, handed to `f` until it says to stop; whether
/// it stopped. A branch that could no longer reach `minimum` with every positive weight left is not walked, so each subset
/// walked leads to one handed on: the time is in what it hands on, not in all the subsets there are.
fn each_reaching(weights: &[i64], minimum: i64, f: &mut dyn FnMut(&[usize]) -> bool) -> bool {
    let mut rest = vec![0i64; weights.len() + 1];
    for i in (0..weights.len()).rev() {
        rest[i] = rest[i + 1] + weights[i].max(0);
    }
    fn visit(weights: &[i64], rest: &[i64], minimum: i64, from: usize, sum: i64, at: &mut Vec<usize>, f: &mut dyn FnMut(&[usize]) -> bool) -> bool {
        if sum >= minimum && f(at) {
            return true;
        }
        for i in from..weights.len() {
            if sum + weights[i] + rest[i + 1] < minimum {
                continue;
            }
            at.push(i);
            if visit(weights, rest, minimum, i + 1, sum + weights[i], at, f) {
                return true;
            }
            at.pop();
        }
        false
    }
    visit(weights, &rest, minimum, 0, 0, &mut Vec::new(), f)
}

/// Each way of dividing `total` into `parts` shares of at least 1, in order, handed to `f` until it says to stop.
fn each_division(total: u32, parts: usize, f: &mut dyn FnMut(&[u32]) -> bool) -> bool {
    fn visit(left: u32, parts: usize, at: &mut Vec<u32>, f: &mut dyn FnMut(&[u32]) -> bool) -> bool {
        if parts == 1 {
            at.push(left);
            let stop = f(at);
            at.pop();
            return stop;
        }
        for n in 1..=left.saturating_sub(parts as u32 - 1) {
            at.push(n);
            if visit(left - n, parts - 1, at, f) {
                return true;
            }
            at.pop();
        }
        false
    }
    parts > 0 && total >= parts as u32 && visit(total, parts, &mut Vec::new(), f)
}

/// Each answer an option of a payment question has (`PayCosts`): the action, with each color it may name and each choice of
/// the objects that come with it (a sacrifice), handed to `f` until it says to stop.
fn each_act(o: &ActionOption, f: &mut dyn FnMut(Answer) -> bool) -> bool {
    let colors: Vec<Option<Color>> = if o.color_choices.is_empty() { vec![None] } else { Color::ALL.into_iter().filter(|&c| o.color_choices.has(c)).map(Some).collect() };
    let least = if o.targets_optional { 0 } else { o.targets_required as usize };
    for color in colors {
        let stopped = each_choice(o.target_candidates.len(), least, o.targets_required as usize, false, &mut |at| {
            f(Answer::Act(Action { color, targets: at.iter().map(|&i| o.target_candidates[i]).collect(), ..Action::of(o) }))
        });
        if stopped {
            return true;
        }
    }
    false
}

/// Each answer `kind` could have, a superset of those `validate::check` accepts, handed to `f` until it says to stop; whether
/// it stopped. `None` for a question no step of casting a spell asks (CR 601.2: no priority, combat, naming, searching or
/// distributing is part of it), which the search therefore never records (`walked`).
///
/// Lazy for every kind and every size: the search asks only whether one live answer is left (`alive`), and stops at the
/// first, which comes after no more dead ones than attempts have failed - so it never lists a question's answers to decide it.
fn each_answer(kind: &RequestKind, f: &mut dyn FnMut(Answer) -> bool) -> Option<bool> {
    Some(match kind {
        RequestKind::PayCosts { announcing: true, least, most, .. } | RequestKind::ChooseNumber { least, most, .. } => {
            (*least..=*most).any(|n| f(Answer::Announce(n)))
        }
        RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } => {
            options.iter().filter(|o| o.kind != ActionKind::CancelCasting).any(|o| each_act(o, f))
        }
        RequestKind::ChooseTarget { candidates, least, count, .. } => {
            each_choice(candidates.len(), *least as usize, *count as usize, false, &mut |at| f(Answer::Targets(at.iter().map(|&i| candidates[i]).collect())))
        }
        RequestKind::ChooseCards { cards, rule, .. } => {
            let most = rule.count.most().map_or(cards.len(), |n| (n as usize).min(cards.len()));
            let least = match rule.count {
                Count::Exactly(n) => (n as usize).min(most),
                _ => 0,
            };
            each_choice(cards.len(), least, most, false, &mut |at| f(Answer::Cards(at.iter().map(|&i| cards[i]).collect())))
        }
        // Up to one creature tapped to reduce a cost (harmonize).
        RequestKind::TapForPower { purpose: Purpose::TapToReduce, cards, .. } => {
            each_choice(cards.len(), 0, 1, false, &mut |at| f(Answer::Cards(at.iter().map(|&i| cards[i]).collect())))
        }
        RequestKind::TapForPower { cards, powers, minimum, .. } => {
            let weights: Vec<i64> = powers.iter().map(|&p| i64::from(p)).collect();
            each_reaching(&weights, i64::from(*minimum), &mut |at| f(Answer::Cards(at.iter().map(|&i| cards[i]).collect())))
        }
        RequestKind::CollectEvidence { cards, values, minimum, .. } => {
            let weights: Vec<i64> = values.iter().map(|&v| i64::from(v)).collect();
            each_reaching(&weights, i64::from(*minimum), &mut |at| f(Answer::Cards(at.iter().map(|&i| cards[i]).collect())))
        }
        RequestKind::Craft { cards, minimum, maximum, .. } => {
            each_choice(cards.len(), *minimum as usize, *maximum as usize, false, &mut |at| f(Answer::Cards(at.iter().map(|&i| cards[i]).collect())))
        }
        RequestKind::ChooseModes { modes, count, at_most, repeat, .. } => {
            // Any set of the modes - a multiset where a mode may be chosen again, of up to as many as may be chosen.
            let most = if *repeat { usize::from((*count).max(*at_most)) } else { modes.len() };
            each_choice(modes.len(), 0, most, *repeat, &mut |at| f(Answer::Modes(at.iter().map(|&i| i as u8).collect())))
        }
        RequestKind::ChooseColors { count, .. } => {
            each_choice(Color::ALL.len(), *count as usize, *count as usize, true, &mut |at| f(Answer::Colors(at.iter().map(|&i| Color::ALL[i]).collect())))
        }
        RequestKind::May { .. } => f(Answer::Yes) || f(Answer::No),
        RequestKind::ChoosePlayer { candidates, .. } => candidates.iter().any(|&p| f(Answer::Player(p))),
        RequestKind::ChooseCostPayment { choices, .. } => (0..choices.len()).any(|i| f(Answer::CostPayment(i))),
        RequestKind::ChooseSubtype { choices, .. } => choices.iter().any(|&t| f(Answer::Subtype(t))),
        RequestKind::ChooseReplacement { sources, .. } => (0..sources.len()).any(|i| f(Answer::Index(i as u8))),
        RequestKind::OrderReplacementOccurrences { occurrences } => (0..occurrences.len()).any(|i| f(Answer::CostPayment(i))),
        RequestKind::RemoveCounters { pools, total, up_to, .. } => {
            // Each way of taking up to each pool's count from each pool, adding up to `total` (or less, for "up to").
            fn visit(pools: &[CounterPool], at: usize, left: u32, up_to: bool, taken: &mut Vec<(ObjectId, CounterKind, u32)>, f: &mut dyn FnMut(Answer) -> bool) -> bool {
                if at == pools.len() {
                    return (left == 0 || up_to) && f(Answer::Counters(taken.clone()));
                }
                let pool = pools[at];
                for n in 0..=pool.count.min(left) {
                    if n > 0 {
                        taken.push((pool.object, pool.kind, n));
                    }
                    let stop = visit(pools, at + 1, left - n, up_to, taken, f);
                    if n > 0 {
                        taken.pop();
                    }
                    if stop {
                        return true;
                    }
                }
                false
            }
            visit(pools, 0, u32::from(*total), *up_to, &mut Vec::new(), f)
        }
        RequestKind::ChooseDividedTargets { candidates, total, max_targets, optional, .. } => {
            // The targets (no more than there is to divide, each taking at least 1), then each way of dividing the total
            // among them, every share at least 1 (CR 601.2d): nothing generated that the game would not accept.
            let most = if *max_targets == 0 { candidates.len() } else { (*max_targets as usize).min(candidates.len()) }.min(*total as usize);
            let least = if *optional || *total == 0 { 0 } else { 1 };
            each_choice(candidates.len(), least, most, false, &mut |at| {
                if at.is_empty() {
                    return f(Answer::Divided(Vec::new()));
                }
                each_division(*total, at.len(), &mut |shares| f(Answer::Divided(at.iter().zip(shares).map(|(&i, &n)| (candidates[i], n)).collect())))
            })
        }
        _ => return None,
    })
}

/// Whether `kind`, asked of `player`, has an answer the game accepts (`validate::check`) that is not one of `dead`. A question
/// no step of casting asks (`each_answer` None) is never recorded, so no answer of it is ever dead: it has its own.
fn alive(g: &Game, player: PlayerId, kind: &RequestKind, dead: &[&Answer]) -> bool {
    let request = Request { id: 0, player, kind: kind.clone(), may_cancel: false };
    let mut found = false;
    let decided = each_answer(kind, &mut |a| {
        if crate::validate::check(g, &request, &a).is_err() {
            return false;
        }
        if canonical(kind, &a).is_some_and(|c| dead.contains(&&c)) {
            return false;
        }
        found = true;
        true
    });
    found || decided.is_none()
}

/// The dead answers to the question at the end of `s`'s path.
fn dead_here(s: &Search) -> Vec<&Answer> {
    let depth = s.path.len();
    s.dead.iter().filter(|d| d.len() == depth + 1 && d[..depth] == s.path[..]).map(|d| &d[depth]).collect()
}

/// The question `kind`, asked of `player` in `frame`, as a required cast's search lets it be asked: without the answers that
/// are dead where they can be left out, and `Err` where no answer the game would accept is alive - a dead end, which the
/// attempt is reversed at without asking.
///
/// **Why no seat is stranded and the search ends.** Every question a required cast asks goes through here
/// (`Cx::ask_core`). It is asked only if it has an answer the game accepts and that is not dead (`alive`, which tries the
/// answers lazily - for every kind a cast asks: payments with their colors and the objects that come with an option,
/// announcements of X, targets, cards, modes, colors, a "may", players, cost orders, subtypes, counters, replacements,
/// divided targets - and stops at the first live one): so a seat always has an answer that will not be refused. One it can't
/// see is dead (a set of targets, a middle X: answers no list leaves out) is refused (`refuses`) without changing anything,
/// and a live one is still there to give. A question with none left is never asked: the attempt is reversed there and its
/// path marked dead, so the question above it loses that answer - and so on up to the first question, when the card can't be
/// cast. Every reversed attempt marks a path no attempt took before, the answers of every question are finitely many, so the
/// tree is finite and the search ends. An X has as many answers as the rules let it be announced, never fewer: one paid in
/// life up to the life there is, one of permanents up to those there are (the announcement's own bounds, `actions::cast`),
/// one paid in mana up to all that could pay for it (`actions::x_bound`: every player's mana, the permanents and cards that
/// could pay instead, the creatures' power, and what the cost changes take off), and one no payment bounds - a power read as
/// the cost is paid, or a yield that can't be told - up to the most its count holds.
///
/// **What deciding costs.** `each_answer` hands on, for every kind, only candidates of the question's own shape - targets of
/// the slot's counts, cards of the selection's count, subsets of cards whose total reaches the minimum (a branch that can't
/// is not walked: `each_reaching`), divisions with every share at least 1 (`each_division`), counters adding up to the total,
/// X within its bounds - so nearly every one handed on is an answer the game accepts. A live one is found after at most as many
/// dead ones as attempts have failed through this question, and all being dead can only be true when there are no more
/// answers than those failed attempts, which is how many the walk then hands on: the time is that of the attempts already made,
/// never that of all the answers there could be (a division of 16 among 15 creatures, an X of thousands). The validator is
/// the only judge, so a question whose shape the walk can't follow exactly (targets "together", a selection of qualities)
/// may pass candidates it then refuses on the way.
pub(crate) fn narrow(g: &Game, frame: usize, player: PlayerId, kind: RequestKind) -> Result<RequestKind, ()> {
    let Some(s) = of(g, frame) else { return Ok(kind) };
    let dead = dead_here(s);
    if dead.is_empty() {
        return Ok(kind);
    }
    let shape = kind.clone();
    let is_dead = |a: Answer| canonical(&shape, &a).is_some_and(|a| dead.contains(&&a));
    let narrowed = match kind {
        RequestKind::ChooseTarget { replacing, label, slot_player, source, purpose, mut candidates, optional, least, count, any_number, together } if count == 1 => {
            candidates.retain(|&t| !is_dead(Answer::Targets(vec![t])));
            RequestKind::ChooseTarget { replacing, label, slot_player, source, purpose, candidates, optional, least, count, any_number, together }
        }
        RequestKind::ChooseModes { source, modes, mut eligible, count, at_most, affordable, additional_cost, repeat, pawprints } if count == 1 && at_most == 0 => {
            eligible.retain(|&m| !is_dead(Answer::Modes(vec![m])));
            RequestKind::ChooseModes { source, modes, eligible, count, at_most, affordable, additional_cost, repeat, pawprints }
        }
        RequestKind::PayCosts { source, total, remaining, mut options, mana_abilities, announcing, x, most, least, loyalty, attackers, blockers, life, governs, assist, x_cost } => {
            for o in &mut options {
                if o.color_choices.is_empty() {
                    continue;
                }
                for c in Color::ALL {
                    if o.color_choices.has(c) && !each_act(&ActionOption { color_choices: ColorSet::of(c), ..o.clone() }, &mut |a| !is_dead(a)) {
                        o.color_choices = ColorSet(o.color_choices.0 & !ColorSet::of(c).0);
                    }
                }
            }
            options.retain(|o| {
                if o.kind == ActionKind::CancelCasting {
                    return true;
                }
                if announcing {
                    return !(o.kind == ActionKind::ConfirmCasting && is_dead(Answer::Announce(x)));
                }
                // An option every answer of which is dead - each color, each choice of the objects that come with it.
                each_act(o, &mut |a| !is_dead(a))
            });
            RequestKind::PayCosts { source, total, remaining, options, mana_abilities, announcing, x, most, least, loyalty, attackers, blockers, life, governs, assist, x_cost }
        }
        other => other,
    };
    if alive(g, player, &narrowed, &dead) { Ok(narrowed) } else { Err(()) }
}

/// The answer just taken to `kind` in `frame` is the next step of the attempt's path.
pub(crate) fn walked(g: &mut Game, frame: usize, kind: &RequestKind, answer: &Answer) {
    if each_answer(kind, &mut |_| true).is_none() {
        return;
    }
    if let Some(s) = g.takeback.search.last_mut().filter(|s| s.frame == frame)
        && let Some(a) = canonical(kind, answer)
    {
        s.path.push(a);
    }
}

/// Whether `answer` to `kind`, the question waiting in `frame`, is dead: the attempt would fail as one has.
#[inline]
pub(crate) fn refuses(g: &Game, frame: usize, kind: &RequestKind, answer: &Answer) -> bool {
    of(g, frame).is_some_and(|s| refuses_in(s, kind, answer))
}

#[cold]
#[inline(never)]
fn refuses_in(s: &Search, kind: &RequestKind, answer: &Answer) -> bool {
    canonical(kind, answer).is_some_and(|a| dead_here(s).contains(&&a))
}
