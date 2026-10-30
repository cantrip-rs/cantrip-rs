//! Search (docs/ai-player-plan.md, "Search"): PUCT over the factoring's sub-steps, the network's policy as the prior
//! and its value at the leaves, in forks of the game (`Game::fork_for_search`) the search moves through with
//! `Game::answer` and `Game::goto` and never copies.
//!
//! A node is one sub-step of one request's factoring (`cantrip_obs::factor::Machine`) where the acting seat has more
//! than one legal option. Its edges are those options. An edge that finishes the request's answer plays it in the
//! fork; then every request that needs no choice is answered at once and gets no node: one with one legal answer (a
//! payment's `AutoPay` among them: payment is no policy step, `pay`), and one that continues a payment run whose first
//! answer the seat already gave in this line (as `net::Payment` keeps to it). Every request in a line is filtered
//! through its seat's cancel guard (`cancel::CancelGuard`): an action the line gives up is not offered again at the
//! board its undo returns to. The request where a seat next has a
//! choice is a new position, its first choosing sub-step the edge's child; a game that ends is a terminal (its result,
//! or 0 for a draw or a line capped at `Config::turn_cap` turns past the root, or by too many requests with no choice).
//!
//! A leaf is expanded with one evaluation of the acting seat's observation in the search's mode: the policy's legal
//! logits (softmaxed at `Config::prior_temperature`) are the edges' priors, the value is the leaf's. Values are backed
//! up from each node's acting seat's side: a node of the seat the leaf's value is for adds it, any other seat's node
//! subtracts it (two players). Selection takes the edge with the most `Q + c_puct * P * sqrt(N) / (1 + n)`, an edge
//! not yet visited taking its node's own value as `Q`. Leaves are evaluated in batches of `Config::batch`, a virtual
//! loss on each edge on the way to a leaf waiting in the batch, across worlds too.
//!
//! The priority shortcut: at a priority's first sub-step, when every legal option but passing has a prior under
//! `Config::shortcut`, passing is the node's only edge (at the root: the search answers pass once the root is
//! expanded).
//!
//! Branch trimming: after a batch's backups, an edge whose value for the seat choosing there is at `Config::prune_bad`
//! or worse is cut when a visited sibling is better, and an edge still `Config::prune_margin` below the best is cut
//! once it has been descended `Config::prune_after` times and an opponent node below it has been expanded
//! (`Config::prune_after_replies`). The best edge at a node is never cut, so every node keeps an option; an unvisited
//! edge, an edge with no child yet, and an edge with virtual loss are never cut. `Child::Dead` (the engine refused an
//! answer) is not a trim and stays as it was. A cut holds only while the trim does (`Config::prune_reversible`): an
//! edge's value is frozen when it is cut, at a shallow estimate, while its siblings' values keep moving as the search
//! finds the replies to them, so each later path restores the cut edges at its nodes that the rule would no longer cut.
//! Cut for good, an edge measured against a sibling's early, optimistic value was never looked at again once that
//! sibling's value fell below it (seen in the advisor, 2026-10-27).
//!
//! Worlds: in Full mode the search plays in one fork of the game; in Decklist and Hidden modes in `Config::worlds`
//! worlds dealt by `determinize` from the seat's knowledge, the root's visits summed over them by the option's token.
//! Each fork is reseeded from `Config::seed`, so no line knows the real game's coming shuffles, and nothing the search
//! does depends on anything else random: the same position, seed and budget give the same answer, and two positions
//! the seat cannot tell apart give it the same worlds (`determinize`) and so the same answer.
//!
//! The answer is walked in the real game's factoring: each sub-step with visits takes the most visited option (or one
//! sampled with weights `visits^(1/T)`, `Choice::Sample`); a sub-step the search never reached is the network's, as
//! `net` takes it. An answer the engine refuses, a search that ran out of time before a simulation, and a request
//! with one legal answer are the network's alone (`net::answer_by`): a seat never times out.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use cantrip_engine::*;
use cantrip_obs::encode;
use cantrip_obs::factor::{FactoredAnswer, Machine, Tok, Token};
use cantrip_obs::{InfoMode, Observation};

use crate::cancel::{self, CancelGuard};
use crate::pay;
use crate::eval::{EvalError, Evaluator, FeatureBatch, MASKED};
use crate::net::{self, Choice, Decision};
use crate::{Answering, Knowledge, Prior, CardIndex, determinize, observe_indexed};

/// Where the search reads the time from: the wall clock, or a test's.
pub trait Clock {
    fn now(&self) -> Instant;
}

/// The wall clock.
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// `Config::memory` by default: 2 GiB. A 3 s search holds about 160 MB at the median and 630 MB at the most, so the
/// bound catches only a runaway, and leaves self-play's deep searches room.
pub const DEFAULT_MEMORY: usize = 2 << 30;

/// What a search may spend and how it searches.
#[derive(Clone, Debug)]
pub struct Config {
    /// Simulations to run (each a leaf evaluated or a terminal reached). With neither this nor a deadline the request
    /// is the network's alone.
    pub simulations: Option<usize>,
    /// When to stop: checked before each simulation and between the requests a line plays with no choice, so the search
    /// stops soon after it whatever one line costs.
    pub deadline: Option<Instant>,
    /// The most the search's tree may hold, in bytes, the part kept from the seat's last search included, as the
    /// counting allocator counts them (`crate::alloc`; no bound where it is not installed). Reached, the search stops
    /// as at its deadline; a kept tree holding more than half of it is not searched on, so the search has room to grow.
    /// A tree's worlds keep every line they played (`Game::goto` moves through it, never forgets it), so without this
    /// a search, and a kept tree from one request to the next, could grow until the machine pages.
    pub memory: Option<usize>,
    /// The limit for the network's answer when the search makes none (`net::answer_by`'s deadline); the search's
    /// own deadline if none.
    pub network_deadline: Option<Instant>,
    /// Worlds in Decklist and Hidden modes (Full plays in one).
    pub worlds: usize,
    pub c_puct: f32,
    pub prior_temperature: f32,
    /// Leaves evaluated together.
    pub batch: usize,
    /// The priority shortcut's bound (see the module).
    pub shortcut: f32,
    pub seed: u64,
    /// How the answer is taken from the root's visits.
    pub choice: Choice,
    /// Turns past the root a line is played before it is taken as a draw.
    pub turn_cap: u32,
    /// Whether edges that are obviously bad are cut (see the module).
    pub prune: bool,
    /// An edge at or below this value for the seat choosing there is cut at once when a visited sibling is better.
    pub prune_bad: f32,
    /// How far below the best visited sibling an edge must be before it is cut once it has had a look.
    pub prune_margin: f32,
    /// Descendants an edge must have been descended through before it can be cut for being below the best.
    pub prune_after: u32,
    /// Opponent decisions below an edge that must have been expanded before it can be cut for being below the best.
    pub prune_after_replies: u32,
    /// Whether a cut is held only while the trim still holds: a cut edge is restored once its best sibling's value has
    /// fallen so that the edge would no longer be cut (see the module). Off, a cut is for good (the search before
    /// 2026-10-27, kept to measure against).
    pub prune_reversible: bool,
    /// Whether a tree kept from this seat's last search is re-rooted and searched on (`Kept`), and how its budget
    /// counts: `None` searches afresh; `TopUp` runs simulations until the root holds the budget, the kept visits
    /// counted; `Add` runs the whole budget on top of them. `Add` by default, following every answer (`Follow::All`):
    /// a kept world is one exactly consistent with the game observed, so a world that cannot follow falls back to a
    /// fresh deal and reuse can at worst equal searching afresh. `reuse_check` tests that.
    /// The recorded visits and root value of a re-rooted search include the visits carried over (as AlphaZero's do).
    pub reuse: Reuse,
    /// What a re-rooted tree follows of another seat's choices on the way (`Follow`).
    pub reuse_follow: Follow,
    /// Check every re-rooted world against the same root built fresh in it (`ReuseCheck`): off in play, on in tests
    /// and in measurement runs. It changes no answer; the fresh roots it evaluates are left unused in the tree.
    pub reuse_check: bool,
    /// The AI's requests made atomic (`crate::atomic`): no cancelling, no mana ability at priority, a cast whose payment
    /// the autotapper cannot finish a refused line. On by default.
    pub atomic: bool,
    /// With `atomic`, a priority's first step is searched as whole answers (`Flat`): each option with its targets and
    /// colours one edge, its prior the product of the factored policy's steps. At most this many whole answers for one
    /// option; an option with more is searched in steps, as before (nothing is left out). 0 searches every priority in
    /// steps.
    pub flatten: usize,
    /// The autotapper as it was before it was told the plan (`pay::with_classic`): for measuring the new one.
    pub pay_classic: bool,
    /// Where the plan does not settle how a cast is paid - its two best classes of payment (`pay::classes`: what each
    /// leaves untapped) each keep payable a follow-up the other does not, and their weights differ by at most this share
    /// of all the node's follow-ups -
    /// the cast is two edges, one for each class, for the search to measure (still one atomic choice). 0: never.
    pub pay_split: f32,
    /// Whether the search also reports what it saw (`Report`): each world's deal and root options, and the top options
    /// completed into whole answers. Off in play and self-play; the advisor asks for it. It changes no answer.
    pub report: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            simulations: None,
            deadline: None,
            memory: Some(DEFAULT_MEMORY),
            network_deadline: None,
            worlds: 8,
            c_puct: 1.5,
            prior_temperature: 1.0,
            batch: 8,
            shortcut: 0.01,
            seed: 0x5ea2c4,
            choice: Choice::Best,
            turn_cap: 40,
            prune: true,
            prune_bad: -0.80,
            prune_margin: 0.20,
            prune_after: 2,
            prune_after_replies: 1,
            prune_reversible: true,
            reuse: Reuse::Add,
            reuse_follow: Follow::All,
            reuse_check: false,
            atomic: true,
            flatten: 64,
            pay_classic: false,
            pay_split: 0.15,
            report: false,
        }
    }
}

impl Config {
    pub fn simulations(n: usize) -> Config {
        Config {
            simulations: Some(n),
            ..Config::default()
        }
    }

    /// Whether this budget searches at all.
    pub fn searches(&self) -> bool {
        self.simulations.is_some_and(|n| n > 0)
            || (self.deadline.is_some() && self.simulations != Some(0))
    }
}

/// Whether and how a search goes on from the tree kept from the seat's last one (`Config::reuse`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reuse {
    #[default]
    None,
    TopUp,
    Add,
    /// Keep the tree but never re-root it: for measuring a search's own tree (`Kept::windows`).
    KeepOnly,
}

/// What a kept tree follows of another seat's choices on the way to the seat's next request (`Config::reuse_follow`).
/// Wherever a world cannot follow (a card it did not deal, a line it never searched, a seat that now sees the game
/// otherwise), the world is dropped and a fresh one dealt in its place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Follow {
    /// None: the seat's own consecutive requests only, every world or none (the first stage).
    Own,
    /// Another seat's passes (the seat's actions in a turn, the opponent passing between them: stage 1 on atomic
    /// choices).
    #[default]
    Passes,
    /// Every answer (stage 2).
    All,
}

/// A search's tree, kept for the seat's next request (`Config::reuse`): its worlds, as the forks left them, and its
/// nodes. The next search of the same seat re-roots each world at the node the answers given since reach, if every
/// world reaches the request now asked, as the seat's request, with the observation the seat has of the real game
/// (so nothing hidden was revealed that tells the worlds apart); otherwise it searches afresh. Answers by another seat
/// with a choice in between are not followed (tree reuse stage 1: the seat's own consecutive requests).
pub struct Kept {
    seat: PlayerId,
    mode: InfoMode,
    /// The real game's node at the request the tree was searched for, and the request's id: a world asks its root
    /// request again when it is dealt (`Game::reask`), so its request ids run on from the real game's by a world's own
    /// offset.
    at: tree::NodeId,
    request: u32,
    worlds: Vec<World>,
    nodes: Vec<Node>,
    deals: Vec<WorldReport>,
    /// The request searched and the factored answer the search chose: a way to read the answer given back into a
    /// world's factoring where the factoring of the engine's answer alone does not make it again.
    chosen: Option<(u32, FactoredAnswer)>,
    /// What the tree held when its search ended, by the counting allocator (0 where it is not installed): what
    /// `Config::memory` counts against the next search before it begins.
    bytes: usize,
}

/// Counts for the timing-equivalence question (a measurement, not a search rule), over a kept tree: positions the
/// search reached twice in one world (the same board by `cancel::board_hash`, the same request kind and seat: a
/// transposition), and another seat's priority windows with an empty stack that only steps began since an earlier such
/// window of theirs in the same turn on the same line (the same "window class"); and the evaluations (expanded nodes)
/// each took.
#[derive(Clone, Debug, Default)]
pub struct Windows {
    pub positions: usize,
    pub evaluations: usize,
    pub transposed_positions: usize,
    pub transposed_evaluations: usize,
    pub opponent_windows: usize,
    pub opponent_window_evaluations: usize,
    pub same_class_windows: usize,
    pub same_class_evaluations: usize,
}

impl Kept {
    /// See `Windows`. Moves the worlds' games through their trees, which the tree's next use does again anyway.
    pub fn windows(&mut self) -> Windows {
        let mut out = Windows::default();
        let seat = self.seat;
        let mut expanded_at: Vec<BTreeMap<usize, usize>> = vec![BTreeMap::new(); self.worlds.len()];
        let mut parent: Vec<BTreeMap<usize, usize>> = vec![BTreeMap::new(); self.worlds.len()];
        for n in &self.nodes {
            if n.expanded {
                *expanded_at[n.world].entry(n.pos).or_default() += 1;
            }
            for e in &n.edges {
                if let Child::Node(c) = e.child
                    && self.nodes[c].pos != n.pos
                {
                    parent[n.world].entry(self.nodes[c].pos).or_insert(n.pos);
                }
            }
        }
        for (wi, w) in self.worlds.iter_mut().enumerate() {
            if w.dead {
                continue;
            }
            // Each position's board, history length, stack and whether it is another seat's window.
            let mut facts = Vec::with_capacity(w.positions.len());
            for p in &w.positions {
                w.game.goto(p.node);
                let window = p.request.player != seat
                    && matches!(p.request.kind, RequestKind::Priority { .. })
                    && w.game.state.stack.is_empty();
                facts.push((
                    cancel::board_hash(&w.game.state),
                    std::mem::discriminant(&p.request.kind),
                    p.request.player,
                    w.game.log.len(),
                    window,
                    p.turn,
                ));
            }
            let mut seen: std::collections::HashMap<(u64, std::mem::Discriminant<RequestKind>, PlayerId), usize> =
                std::collections::HashMap::new();
            for (i, f) in facts.iter().enumerate() {
                let evals = expanded_at[wi].get(&i).copied().unwrap_or(0);
                out.positions += 1;
                out.evaluations += evals;
                if seen.insert((f.0, f.1, f.2), i).is_some() {
                    out.transposed_positions += 1;
                    out.transposed_evaluations += evals;
                }
                if !f.4 {
                    continue;
                }
                out.opponent_windows += 1;
                out.opponent_window_evaluations += evals;
                // Up the line while only steps began since: an earlier window of theirs in the turn is the same class.
                w.game.goto(w.positions[i].node);
                let mut at = i;
                while let Some(&up) = parent[wi].get(&at) {
                    let quiet = w.game.log[facts[up].3.min(f.3)..f.3]
                        .iter()
                        .all(|r| matches!(r.event, Event::StepBegan { .. }));
                    if !quiet || facts[up].5 != f.5 {
                        break;
                    }
                    if facts[up].4 {
                        out.same_class_windows += 1;
                        out.same_class_evaluations += evals;
                        break;
                    }
                    at = up;
                }
            }
        }
        out
    }
}

/// What a re-rooted search carried over.
#[derive(Clone, Debug, Default)]
pub struct Reused {
    /// Live worlds offered to be re-rooted, and the choices another seat made on the way (`Config::reuse_across`),
    /// those that were not passes among them; and why the worlds lost were lost.
    pub offered: usize,
    pub opponent_choices: usize,
    pub opponent_actions: usize,
    pub lost: BTreeMap<String, usize>,
    /// Fresh worlds dealt in place of the lost ones.
    pub refilled: usize,
    /// Worlds re-rooted, and the visits their roots already held.
    pub worlds: usize,
    pub visits: u32,
    /// The kept tree's most visited option at the new root, before any new simulation: its principal line.
    pub principal: Option<Token>,
    /// What the reuse check found, where it ran (`Config::reuse_check`).
    pub check: Option<ReuseCheck>,
}

/// The reuse check (`Config::reuse_check`): each re-rooted world's root against the same root built fresh in that world,
/// as a fresh search would deal it there (the real game's knowledge and cancel guard for the seat, every other seat's
/// knowledge folded from the world's history, no payment run, no other seat's guard) and evaluated afresh. Any mismatch
/// is a kept tree that differs from a fresh search: a bug.
#[derive(Clone, Debug, Default)]
pub struct ReuseCheck {
    /// Worlds checked.
    pub worlds: usize,
    /// Mismatches by what differed: `request` (the kept root's request and the real one), `seat knowledge`,
    /// `opponent knowledge`, `seat guard`, `opponent guard`, `payment run`, `observation`, `encoding` (the network's
    /// input at the root), `options` (the root's edges), `prior`, `value`, `shortcut`, `stale cut` (a cut edge whose
    /// trim no longer holds), `virtual loss` (left on the root), `fresh root` (none could be built).
    pub mismatches: BTreeMap<String, usize>,
    /// The largest difference seen in a prior, and in the root's value, between a kept root and its fresh evaluation.
    pub prior_diff: f32,
    pub value_diff: f32,
}

/// The real game's boards a cancel guard can name that a world can return to - the root's, and the start of each run
/// still waiting there, which a take-back reverts to - each with the same board's hash in `world`, a fork of it dealt
/// anew (or a kept world at the same request): the world's hidden cards change the hash. Frames are paired by place,
/// each reverted to its own start.
pub(crate) fn world_boards(real: &Game, world: &Game) -> Vec<(u64, u64)> {
    let mut out = vec![(cancel::board_hash(&real.state), cancel::board_hash(&world.state))];
    for (r, w) in real.state.agenda.iter().zip(&world.state.agenda) {
        // Only a run still waiting is held back to its start (`Game::fork_for_search`), in both games.
        if (r.pending.is_none() && r.answers.is_empty()) || (w.pending.is_none() && w.answers.is_empty()) {
            continue;
        }
        // A copy holding the journal back to the run's start (a plain copy holds none), reverted to it.
        let at = |g: &Game, mark: u32| {
            let mut state = g.state.clone_with_journal_from(Mark(mark));
            state.revert(Mark(mark));
            cancel::board_hash(&state)
        };
        out.push((at(real, r.mark), at(world, w.mark)));
    }
    out
}

/// `kind` written out with each object named by its card in `game`: two requests the same but for which of two alike
/// cards is which object read the same (`ReuseCheck`).
fn by_card(game: &Game, kind: &RequestKind) -> String {
    let text = format!("{kind:?}");
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(at) = rest.find("ObjectId(") {
        out.push_str(&rest[..at]);
        let after = &rest[at + "ObjectId(".len()..];
        let digits = after.find(')').unwrap_or(0);
        match after[..digits].parse::<u32>() {
            Ok(n) if game.state.objects.get(n as usize).is_some() => {
                out.push_str(&format!("Card({:?})", game.state.obj(ObjectId(n)).card));
            }
            _ => out.push_str(&rest[at..at + "ObjectId(".len() + digits + 1]),
        }
        rest = &after[digits + 1..];
    }
    out.push_str(rest);
    out
}

/// Whether a run still waiting in `game` drew on the generator since it began (`GameState::random_since`): re-run in a
/// fork for search, it would draw otherwise than the game did.
pub(crate) fn random_before(game: &Game) -> bool {
    game.state
        .agenda
        .iter()
        .filter(|f| f.pending.is_some() || !f.answers.is_empty())
        .any(|f| !f.staged && game.state.random_since(Mark(f.mark)))
}

/// The difference a prior or a value may show between two evaluations of the same input: a remote evaluator batches
/// them differently, and a GPU's sums are not bit-reproducible across batches.
pub const CHECK_TOLERANCE: f32 = 2e-3;

/// Requests answered with no choice in a row before a line is taken as a draw.
pub const MAX_FORCED: usize = 2000;

/// What a search made of a request.
#[derive(Clone, Debug, Default)]
pub struct Searched {
    pub simulations: usize,
    /// Leaves evaluated (the root's included), and the batches they were evaluated in.
    pub evaluations: usize,
    pub batches: usize,
    /// Sub-step nodes, positions (requests reached), and nodes the forks' game trees grew by.
    pub nodes: usize,
    pub positions: usize,
    pub game_nodes: usize,
    /// The peak heap the search held, by the counting allocator (`crate::alloc`) when it is installed (`counted`);
    /// otherwise an estimate of the search's own structures (not the forks' trees).
    pub tree_bytes: usize,
    pub counted: bool,
    /// Whether the search stopped for `Config::memory` before its budget was spent, and whether a kept tree was let go
    /// unsearched for holding more than half of it.
    pub memory_bound: bool,
    pub kept_dropped: bool,
    /// The root's value for the seat: its visits' mean, over every world.
    pub root_value: f32,
    /// The network's value of the root (the first world's).
    pub net_value: f32,
    pub worlds: usize,
    /// Each root sub-step the answer was walked through with visits: its options and theirs, summed over the worlds.
    pub visits: Vec<RootStep>,
    /// Whether the root took the priority shortcut.
    pub shortcut: bool,
    /// Edges that led to an answer the engine refused, and forced answers it refused.
    pub refused: usize,
    /// Edges cut by branch trimming, and how many of those were cut at once or after a look.
    pub pruned: usize,
    pub pruned_immediate: usize,
    pub pruned_shallow: usize,
    /// Cut edges restored (`Config::prune_reversible`).
    pub restored: usize,
    /// How wide and deep the tree reached.
    pub shape: Shape,
    /// Whether the worlds agreed on the root's first sub-step (`Fusion`); `None` with fewer than two live worlds.
    pub fusion: Option<Fusion>,
    pub elapsed: Duration,
    /// What the search saw, when `Config::report` asked for it.
    pub report: Option<Report>,
    /// What a re-rooted search carried over from the seat's last (`Config::reuse`).
    pub reused: Option<Reused>,
    /// Rows the network was asked for, the leaves' own and the inner steps of flattened priorities (`Flat`).
    pub rows: usize,
    /// Priorities searched as whole answers, and the options among them searched in steps for having more whole answers
    /// than `Config::flatten`.
    pub flattened: usize,
    pub flatten_fallbacks: usize,
    /// The root's edges summed over its worlds (its options, as searched).
    pub root_edges: usize,
    /// Leaves evaluated below a cancel or a mana ability activated on its own: the search spent on them.
    pub waste_leaves: usize,
    /// Lines whose cast or activation the autotapper could not pay for (`Config::atomic`).
    pub unpayable: usize,
    /// With `Config::reuse_check`: the worlds dealt fresh at this search's root, and those whose root observation for the
    /// seat is not the real game's - a world the seat could tell from the game it is in, which a deal must never make.
    pub fresh_checked: usize,
    pub fresh_mismatched: usize,
    /// Casts split by class of payment (`Config::pay_split`), and at the root each split edge's whole answer (its tokens
    /// from the root's first step), the sources it leaves untapped and its visits over the worlds: what the seat's payment
    /// keeps open for the answer chosen (`net::Payment`).
    pub pay_splits: usize,
    pub pay_keeps: Vec<(Vec<Token>, Vec<ObjectId>, u32)>,
    /// The lines' time (`Forks::t_play`): all, copying a position's state, moving to it, folding knowledge and observing.
    pub play_time: Duration,
    pub setup_time: Duration,
    pub goto_time: Duration,
    pub observe_time: Duration,
}

/// What a search saw at its root (`Config::report`): for the advisor, which shows a seat's decision and records it.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Each world searched, in the order dealt.
    pub worlds: Vec<WorldReport>,
    /// The factoring's tokens chosen before the root's first sub-step with a choice (steps with one legal option).
    pub prefix: Vec<Token>,
    /// The root's first sub-step's most visited options, each completed into a whole answer: at later sub-steps the
    /// most visited option over the worlds, or the network's best where the search never went.
    pub candidates: Vec<Candidate>,
}

/// One world of a search: what it dealt, and its root's first sub-step.
#[derive(Clone, Debug, Default)]
pub struct WorldReport {
    /// The world's seed (the fork's reseed and the deal's).
    pub seed: u64,
    /// Whether the world could not be searched (no line from its root was answered).
    pub dead: bool,
    /// The network's value of the world's root, for the seat.
    pub value: f32,
    /// What the world dealt each other seat's hand, by card name: the seat (from 1) and its hand.
    pub hands: Vec<(u8, Vec<String>)>,
    /// The top of each other seat's library as dealt, up to three cards, top first: the seat (from 1) and the cards.
    pub library_tops: Vec<(u8, Vec<String>)>,
    /// The root's options, with the network's prior, the visits and the visits' summed value for the seat.
    pub edges: Vec<EdgeReport>,
    /// Whether the root took the priority shortcut (the edges are then pass alone).
    pub shortcut: bool,
}

/// One option at a world's root.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeReport {
    pub token: Token,
    /// The tokens of a whole answer (`Flat`); `token` alone for a step's own option.
    pub path: Vec<Token>,
    pub prior: f32,
    pub visits: u32,
    /// The sum of the values backed up through it, for the seat: `value_sum / visits` is its mean.
    pub value_sum: f32,
    /// Cut by branch trimming.
    pub pruned: bool,
    /// The engine refused the answer it leads to.
    pub dead: bool,
}

/// One of the root's options, completed into a whole answer.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// The option at the root's first sub-step with a choice.
    pub token: Token,
    /// The whole answer's tokens from there (`Flat`), or `token` alone.
    pub path: Vec<Token>,
    pub factored: FactoredAnswer,
    /// The engine's answer it makes, or why the engine refused it.
    pub answer: Result<Answer, String>,
}

/// Strategy fusion at the root, read from the finished tree: each live world's most visited option at the root's first
/// sub-step, compared across the worlds. Each world is searched as if its deal were the game, so where they differ the
/// seat's one real choice is a vote between answers to different games.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fusion {
    /// Worlds whose root was visited.
    pub worlds: usize,
    /// Distinct most-visited options among them.
    pub distinct: usize,
    /// Worlds whose most visited option is the one most worlds chose.
    pub plurality: usize,
}

/// How far a search's tree reached over its live worlds, read from the finished tree: the root sub-step's options, and
/// the lengths of the lines the simulations visited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shape {
    /// The root sub-step's options, and how many of them the simulations visited (the most of any world).
    pub root_options: usize,
    pub root_visited: usize,
    /// The deepest visited line: its sub-steps and its decisions (requests with a choice), the root's counted, and the
    /// turns past the root it reached.
    pub deepest_steps: usize,
    pub deepest_decisions: usize,
    pub deepest_turns: u32,
    /// The same along the most visited line of the first live world.
    pub line_steps: usize,
    pub line_decisions: usize,
    pub line_turns: u32,
}

/// One root sub-step: the tokens chosen before it, its options' tokens, their visits, the one taken.
#[derive(Clone, Debug, PartialEq)]
pub struct RootStep {
    pub prefix: Vec<Token>,
    pub tokens: Vec<Token>,
    pub visits: Vec<u32>,
    pub chosen: Token,
}

/// A request in a fork: where its game node is, and what the acting seat sees there.
pub(crate) struct Position {
    pub(crate) node: tree::NodeId,
    pub(crate) request: Request,
    /// Every seat's knowledge at this position, the acting seat's up to date.
    pub(crate) knowledge: Vec<Knowledge>,
    pub(crate) obs: Observation,
    pub(crate) answering: Answering,
    /// Each seat's payment run in this line: the run's key and the answer its first request was given.
    pub(crate) runs: Vec<Option<(RunKey, FactoredAnswer)>>,
    /// Each seat's cancel guard in this line: the actions it has given up on, and what it is in the middle of.
    pub(crate) guards: Vec<CancelGuard>,
    pub(crate) turn: u32,
}

pub(crate) type RunKey = (PlayerId, u8, ObjectId, ObjectId, bool);

pub(crate) struct World {
    pub(crate) game: Game,
    pub(crate) positions: Vec<Position>,
    pub(crate) root: usize,
    pub(crate) dead: bool,
    pub(crate) root_turn: u32,
    /// The root's position: 0, unless the tree was kept from an earlier request and re-rooted (`Kept`).
    pub(crate) root_pos: usize,
}

/// The forks a search plays in, and how a line moves through one: a request's answer played, then every request after
/// it that needs no choice, to the next request where a seat has one. Shared by the PUCT search here and the
/// information-set search (`crate::issearch`).
pub(crate) struct Forks<'a> {
    pub(crate) mode: InfoMode,
    pub(crate) table: &'a CardIndex,
    /// Turns past a world's root a line is played before it is capped.
    pub(crate) turn_cap: u32,
    pub(crate) worlds: Vec<World>,
    /// Answers the engine refused.
    pub(crate) refused: usize,
    /// Every seat's requests made atomic in the lines (`Config::atomic`).
    pub(crate) atomic: bool,
    /// Lines whose cast or activation the autotapper could not pay for (atomic: the edge refused).
    pub(crate) unpayable: usize,
    /// Where `play` spends its time (a profile, for self-play's budget): all of it, copying the position's state, moving
    /// the world's game to the position (`Game::goto`), and folding knowledge and observing at each position it reaches;
    /// the rest is the engine answering.
    pub(crate) t_play: Duration,
    pub(crate) t_setup: Duration,
    pub(crate) t_goto: Duration,
    pub(crate) t_observe: Duration,
    /// Whether the answer `play` last played was a cancel or a mana ability activated on its own: what the search spent
    /// below such answers is counted (`Searched::waste_leaves`).
    pub(crate) last_waste: bool,
    /// The clock and the deadline a line stops at between the requests it plays with no choice (`Reached::OutOfTime`);
    /// none, a line is played out.
    pub(crate) stop: Option<(&'a dyn Clock, Instant)>,
}

/// Where playing an answer in a world led.
pub(crate) enum Reached {
    /// A request where a seat has a choice: its position, and the factoring past the steps with one option.
    Decide(usize, Machine),
    /// The game ended: the result for the seat that answered.
    Over(f32),
    /// The line reached the turn cap or `MAX_FORCED` requests with no choice.
    Capped,
    /// A request after it could not be answered or observed.
    Stuck,
    /// The engine refused the answer.
    Dead,
    /// The deadline (`Forks::stop`) passed before the line reached a request with a choice: where it leads is not known
    /// yet, and the search stops.
    OutOfTime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Child {
    None,
    Node(usize),
    /// The line ends: the value for the seat of the node the edge is from.
    Terminal(f32),
    /// The engine refused the answer, or every line under it was.
    Dead,
}

struct Edge {
    index: usize,
    /// The choices after `index` a whole answer makes (`Flat`; forced ones included), none for a step's own option.
    rest: Vec<usize>,
    /// The tokens it chooses, from the node's step: `token` alone for a step's own option.
    path: Vec<Token>,
    token: Token,
    prior: f32,
    n: u32,
    w: f32,
    vl: u32,
    /// Opponent decisions expanded below this edge: what a trim for being below the best waits for.
    replies: u16,
    /// Cut by branch trimming. Not `Child::Dead`: that is the engine refusing an answer.
    pruned: bool,
    child: Child,
    /// The sources this edge's cast leaves untapped, where the cast was split by class of payment (`Config::pay_split`).
    keep: Option<std::sync::Arc<Vec<ObjectId>>>,
}

/// Why an edge was cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PruneKind {
    /// At or below `prune_bad`, with a visited sibling better.
    Immediate,
    /// Still `prune_margin` below the best after a look.
    Shallow,
}

/// The trim rule, over an edge's mean `q` for its node's seat and the best visited sibling's `best`.
fn trim(q: f32, best: f32, visits: u32, replies: u16, config: &Config) -> Option<PruneKind> {
    if best <= q {
        return None;
    }
    if q <= config.prune_bad {
        return Some(PruneKind::Immediate);
    }
    if visits >= config.prune_after
        && u32::from(replies) >= config.prune_after_replies
        && best - q >= config.prune_margin
    {
        return Some(PruneKind::Shallow);
    }
    None
}

struct Node {
    /// Below a cancel or a mana ability activated on its own (`Forks::last_waste`).
    waste: bool,
    world: usize,
    pos: usize,
    machine: Machine,
    seat: PlayerId,
    edges: Vec<Edge>,
    expanded: bool,
    queued: bool,
    value: f32,
    shortcut: bool,
}

enum Selected {
    Leaf(usize, Vec<(usize, usize)>),
    Terminal(f32, PlayerId, Vec<(usize, usize)>),
    Collision(Vec<(usize, usize)>),
    Dead(Vec<(usize, usize)>),
    /// The deadline passed while an edge on the way was being played out (`Reached::OutOfTime`): the edge is left as
    /// it was, unplayed.
    OutOfTime(Vec<(usize, usize)>),
}

pub(crate) enum Forward {
    Decide(Machine),
    Done(FactoredAnswer),
    Stuck,
}

/// Takes every sub-step with one legal option.
pub(crate) fn forward(mut m: Machine) -> Forward {
    loop {
        let Some(step) = m.step() else {
            return m.answer().cloned().map_or(Forward::Stuck, Forward::Done);
        };
        let (first, more) = {
            let mut legal = step.legal();
            (legal.next(), legal.next().is_some())
        };
        match (first, more) {
            (Some(i), false) => m.choose(i),
            (Some(_), true) => return Forward::Decide(m),
            (None, _) => return Forward::Stuck,
        }
    }
}

/// `splitmix`, for a caller seeding searches as self-play does.
pub fn splitmix_pub(x: u64) -> u64 {
    splitmix(x)
}

pub(crate) fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn outcome_for(game: &Game, seat: PlayerId) -> f32 {
    match game.state.outcome {
        Outcome::Won(p) if p == seat => 1.0,
        Outcome::Won(_) => -1.0,
        _ => 0.0,
    }
}

struct Search<'a> {
    config: &'a Config,
    forks: Forks<'a>,
    nodes: Vec<Node>,
    pruned: usize,
    pruned_immediate: usize,
    pruned_shallow: usize,
    /// Cut edges restored once their trim no longer held (`Config::prune_reversible`).
    restored: usize,
    flattened: usize,
    flatten_fallbacks: usize,
    /// Each priority's follow-ups as its first step last gave them (`Search::follow_ups`), by world and position.
    follow: std::collections::HashMap<(usize, usize), pay::FollowUps>,
    /// Casts split by class of payment (`Config::pay_split`).
    pay_splits: usize,
}

/// A priority's first step as whole answers (`Config::flatten`): every option completed through the steps after it
/// with its targets and colours, each completion an edge.
struct Flat {
    /// The steps below the first where a whole answer chooses among several options: the machine there.
    inner: Vec<Machine>,
    combos: Vec<Combo>,
    /// Options searched in steps instead, for having more whole answers than the bound.
    fallbacks: usize,
}

/// One whole answer: the first step's option, the choices after it (forced ones included), the inner steps it chooses
/// at (`Flat::inner`, and the option there) for its prior, and its tokens.
struct Combo {
    first: usize,
    rest: Vec<usize>,
    steps: Vec<(usize, usize)>,
    path: Vec<Token>,
}

/// `m`'s step flattened (see `Flat`), each option's whole answers at most `bound`.
fn flatten(m: &Machine, bound: usize) -> Option<Flat> {
    let step = m.step()?;
    let mut flat = Flat { inner: Vec::new(), combos: Vec::new(), fallbacks: 0 };
    for i in step.legal() {
        let mut m1 = m.clone();
        m1.choose(i);
        let mut out = Vec::new();
        let mut inner = Vec::new();
        let complete = whole(m1, Vec::new(), Vec::new(), vec![step.tokens[i]], &mut out, &mut inner, bound);
        if !complete || out.is_empty() {
            if !complete {
                flat.fallbacks += 1;
            }
            flat.combos.push(Combo { first: i, rest: Vec::new(), steps: Vec::new(), path: vec![step.tokens[i]] });
            continue;
        }
        let base = flat.inner.len();
        flat.inner.extend(inner);
        for (rest, steps, path) in out {
            flat.combos.push(Combo { first: i, rest, steps: steps.into_iter().map(|(k, c)| (k + base, c)).collect(), path });
        }
    }
    Some(flat)
}

/// The whole answers below `m` into `out` (choices, inner steps chosen at, tokens), the steps with a choice into
/// `inner`; false once there would be more than `bound`. A branch with nothing legal makes no answer.
#[allow(clippy::type_complexity)]
fn whole(
    mut m: Machine,
    mut rest: Vec<usize>,
    steps: Vec<(usize, usize)>,
    mut path: Vec<Token>,
    out: &mut Vec<(Vec<usize>, Vec<(usize, usize)>, Vec<Token>)>,
    inner: &mut Vec<Machine>,
    bound: usize,
) -> bool {
    loop {
        let Some(step) = m.step() else {
            if m.answer().is_some() {
                out.push((rest, steps, path));
            }
            return out.len() <= bound;
        };
        let legal: Vec<usize> = step.legal().collect();
        match legal.len() {
            0 => return true,
            1 => {
                rest.push(legal[0]);
                path.push(step.tokens[legal[0]]);
                m.choose(legal[0]);
            }
            _ => {
                let k = inner.len();
                inner.push(m.clone());
                for &c in &legal {
                    if out.len() >= bound {
                        return false;
                    }
                    let mut m2 = m.clone();
                    m2.choose(c);
                    let (mut r2, mut s2, mut p2) = (rest.clone(), steps.clone(), path.clone());
                    r2.push(c);
                    s2.push((k, c));
                    p2.push(step.tokens[c]);
                    if !whole(m2, r2, s2, p2, out, inner, bound) {
                        return false;
                    }
                }
                return true;
            }
        }
    }
}

/// The policy's probabilities over a step's legal options at temperature `t`, by option index.
fn softmax_legal(step: &cantrip_obs::factor::Step, logits: &[f32], t: f32) -> BTreeMap<usize, f32> {
    let legal: Vec<usize> = step.legal().collect();
    let logit = |i: usize| logits.get(i).copied().unwrap_or(MASKED);
    let top = legal.iter().map(|&i| logit(i)).fold(f32::MIN, f32::max);
    let w: Vec<f32> = legal.iter().map(|&i| ((logit(i) - top) / t).exp()).collect();
    let sum = w.iter().sum::<f32>().max(1e-30);
    legal.into_iter().zip(w).map(|(i, w)| (i, w / sum)).collect()
}

impl Forks<'_> {
    /// The position for the request `world`'s game waits on, its seat's knowledge brought up to date and the actions
    /// its cancel guard has given up on filtered out.
    pub(crate) fn position(
        &mut self,
        world: usize,
        mut knowledge: Vec<Knowledge>,
        runs: Vec<Option<(RunKey, FactoredAnswer)>>,
        mut guards: Vec<CancelGuard>,
    ) -> Option<usize> {
        let w = &mut self.worlds[world];
        let request = w.game.pending()?.clone();
        let p = request.player.index();
        let request = guards[p].filtered(&w.game, &request);
        let request = if self.atomic { crate::atomic::filter(&w.game, &request) } else { request };
        knowledge[p].update(&w.game);
        let (obs, index) = observe_indexed(&w.game, &knowledge[p], request.player, self.mode)?;
        let answering = Answering::new(&w.game, &request, &obs, &index);
        w.positions.push(Position {
            node: w.game.checkpoint(),
            request,
            knowledge,
            obs,
            answering,
            runs,
            guards,
            turn: w.game.state.turn.number,
        });
        Some(w.positions.len() - 1)
    }

    /// Plays `factored` at position `pos`, then every request after it that needs no choice. Each answer goes through
    /// its seat's cancel guard, so an action a line gives up is not offered again at the board it returns to.
    pub(crate) fn play(
        &mut self,
        world: usize,
        pos: usize,
        factored: FactoredAnswer,
        seat: PlayerId,
    ) -> Reached {
        let t = Instant::now();
        let reached = self.play_timed(world, pos, factored, seat);
        self.t_play += t.elapsed();
        reached
    }

    fn play_timed(
        &mut self,
        world: usize,
        pos: usize,
        factored: FactoredAnswer,
        seat: PlayerId,
    ) -> Reached {
        let started = Instant::now();
        let cap = self.worlds[world].root_turn + self.turn_cap;
        let (node, request, answering, mut runs, mut knowledge, mut guards) = {
            let p = &self.worlds[world].positions[pos];
            (
                p.node,
                p.request.clone(),
                p.answering.clone(),
                p.runs.clone(),
                p.knowledge.clone(),
                p.guards.clone(),
            )
        };
        let copied = Instant::now();
        self.t_setup += copied - started;
        let w = &mut self.worlds[world];
        w.game.goto(node);
        self.t_goto += copied.elapsed();
        let w = &mut self.worlds[world];
        let actor = request.player.index();
        let Ok(answer) = answering.engine(&w.game, &request, &factored) else {
            self.refused += 1;
            return Reached::Dead;
        };
        self.last_waste = crate::atomic::cancels(&answer) || crate::atomic::activates_mana_ability(&w.game, &request, &answer);
        let cancelled = guards[actor].before(&w.game, &request, &answer);
        if w.game
            .answer(request.id, request.player, answer.clone())
            .is_err()
        {
            self.refused += 1;
            return Reached::Dead;
        }
        guards[actor].after(&w.game, cancelled);
        runs[actor] = crate::record::run_key(&request).map(|k| (k, factored));
        for forced in 0usize.. {
            let w = &mut self.worlds[world];
            if w.game.is_over() {
                return Reached::Over(outcome_for(&w.game, seat));
            }
            if forced >= MAX_FORCED || w.game.state.turn.number > cap {
                return Reached::Capped;
            }
            // A line can run to the turn cap with no choice on the way, each request observed: the deadline is read
            // between them, not only between simulations.
            if self.stop.is_some_and(|(clock, d)| clock.now() >= d) {
                return Reached::OutOfTime;
            }
            let Some(request) = w.game.pending().cloned() else {
                return Reached::Stuck;
            };
            let i = request.player.index();
            let request = guards[i].filtered(&w.game, &request);
            // Atomic: a payment the autotapper cannot finish is a cast that could not be made, never a cancel to explore.
            if self.atomic && crate::atomic::unpayable(&w.game, &request) {
                if std::env::var_os("CANTRIP_UNPAYABLE_DEBUG").is_some()
                    && let RequestKind::PayCosts { remaining, options, .. } = &request.kind
                {
                    let names: Vec<String> = options
                        .iter()
                        .map(|o| format!("{:?}:{}:{}", o.kind, o.source.map_or("", |x| w.game.name(x)), o.index))
                        .collect();
                    eprintln!("unpayable: owed {remaining:?} options {names:?} pool {:?}", w.game.state.player(request.player).pool);
                }
                self.unpayable += 1;
                return Reached::Dead;
            }
            let request = if self.atomic { crate::atomic::filter(&w.game, &request) } else { request };
            let observing = Instant::now();
            knowledge[i].update(&w.game);
            let Some((obs, index)) =
                observe_indexed(&w.game, &knowledge[i], request.player, self.mode)
            else {
                return Reached::Stuck;
            };
            let answering = Answering::new(&w.game, &request, &obs, &index);
            self.t_observe += observing.elapsed();
            let run = crate::record::run_key(&request);
            if let (Some(key), Some((k, f))) = (run, &runs[i])
                && key == *k
                && let Ok(a) = answering.engine(&w.game, &request, f)
            {
                let saved = guards[i].clone();
                let cancelled = guards[i].before(&w.game, &request, &a);
                if w.game.answer(request.id, request.player, a.clone()).is_ok() {
                    guards[i].after(&w.game, cancelled);
                    continue;
                }
                guards[i] = saved;
            }
            match forward(Machine::new(&answering.ask, &[])) {
                Forward::Done(f) => {
                    let Ok(a) = answering.engine(&w.game, &request, &f) else {
                        self.refused += 1;
                        return Reached::Stuck;
                    };
                    let cancelled = guards[i].before(&w.game, &request, &a);
                    if w.game
                        .answer(request.id, request.player, a.clone())
                        .is_err()
                    {
                        self.refused += 1;
                        return Reached::Stuck;
                    }
                    guards[i].after(&w.game, cancelled);
                    runs[i] = run.map(|k| (k, f));
                }
                Forward::Stuck => {
                    self.refused += 1;
                    return Reached::Stuck;
                }
                Forward::Decide(m) => {
                    w.positions.push(Position {
                        node: w.game.checkpoint(),
                        request,
                        knowledge,
                        obs,
                        answering,
                        runs,
                        guards,
                        turn: w.game.state.turn.number,
                    });
                    return Reached::Decide(w.positions.len() - 1, m);
                }
            }
        }
        unreachable!()
    }
}

impl Search<'_> {
    fn node(&mut self, world: usize, pos: usize, machine: Machine) -> usize {
        let seat = self.forks.worlds[world].positions[pos].request.player;
        self.nodes.push(Node {
            waste: false,
            world,
            pos,
            machine,
            seat,
            edges: Vec::new(),
            expanded: false,
            queued: false,
            value: 0.0,
            shortcut: false,
        });
        self.nodes.len() - 1
    }

    /// What taking edge `e` of node `at` leads to; none if the deadline passed while its line was played out.
    fn materialize(&mut self, at: usize, e: usize) -> Option<Child> {
        let (world, pos) = (self.nodes[at].world, self.nodes[at].pos);
        let mut m = self.nodes[at].machine.clone();
        m.choose(self.nodes[at].edges[e].index);
        for &c in &self.nodes[at].edges[e].rest {
            m.choose(c);
        }
        let waste = self.nodes[at].waste;
        Some(match forward(m) {
            Forward::Decide(m) => {
                let c = self.node(world, pos, m);
                self.nodes[c].waste = waste;
                Child::Node(c)
            }
            Forward::Stuck => {
                self.forks.refused += 1;
                Child::Dead
            }
            Forward::Done(factored) => {
                let seat = self.nodes[at].seat;
                let follow = self.follow_ups(at, e);
                let keep = self.nodes[at].edges[e].keep.clone();
                let classic = self.config.pay_classic;
                let forks = &mut self.forks;
                let played = pay::with_classic(classic, || {
                    pay::with_payment_keep(seat, keep.as_deref().map_or(&[][..], |k| &k[..]), || {
                        pay::with_follow_ups(seat, &follow, || forks.play(world, pos, factored, seat))
                    })
                });
                match played {
                    Reached::Decide(pos, m) => {
                        let c = self.node(world, pos, m);
                        self.nodes[c].waste = waste || self.forks.last_waste;
                        Child::Node(c)
                    }
                    Reached::Over(v) => Child::Terminal(v),
                    // A capped or stuck line is a draw to this search.
                    Reached::Capped | Reached::Stuck => Child::Terminal(0.0),
                    Reached::Dead => Child::Dead,
                    Reached::OutOfTime => return None,
                }
            }
        })
    }

    /// A priority's whole answers (`Flat`) with each cast the plan does not settle the payment of made two edges, one for
    /// each of its two best classes of payment (`Config::pay_split`, `pay::classes`), each with half its prior; the
    /// follow-ups are the node's other options that cost mana, by their priors.
    fn split_by_payment(&mut self, at: usize, edges: Vec<Edge>) -> Vec<Edge> {
        let (wi, pi) = (self.nodes[at].world, self.nodes[at].pos);
        let node_at = self.forks.worlds[wi].positions[pi].node;
        self.forks.worlds[wi].game.goto(node_at);
        let w = &self.forks.worlds[wi];
        let position = &w.positions[pi];
        // The mana abilities are the engine's own request's (the AI's copy has them taken out: `atomic`).
        let Some(options) = w.game.pending().and_then(|r| match &r.kind {
            RequestKind::Priority { options } => Some(options.clone()),
            _ => None,
        }) else {
            return edges;
        };
        let costs: Vec<Option<(u16, ManaCost)>> = edges
            .iter()
            .map(|e| {
                (e.token.kind == Tok::Option)
                    .then(|| position.answering.priority_option(&position.request, e.token.n))
                    .flatten()
                    .and_then(|o| pay::option_cost(&w.game, o))
                    .map(|c| (e.token.n, c))
            })
            .collect();
        let priors: Vec<f32> = edges.iter().map(|e| e.prior).collect();
        let mut out = Vec::with_capacity(edges.len());
        let mut split = 0;
        for (k, e) in edges.into_iter().enumerate() {
            let Some((n, cost)) = &costs[k] else {
                out.push(e);
                continue;
            };
            let follow: pay::FollowUps = costs
                .iter()
                .zip(&priors)
                .filter_map(|(c, &p)| c.as_ref().filter(|(m, _)| m != n).map(|(_, c)| (*c, p)))
                .collect();
            let total: f32 = follow.iter().map(|(_, w)| w).sum();
            let classes = pay::classes(&w.game, &options, cost, &follow);
            // Split only where the plan's follow-ups disagree - each of the two best classes keeps payable one the other
            // does not - and their weights are close; a class that keeps all the other does is simply better.
            let disagree = |a: u64, b: u64| a & !b != 0 && b & !a != 0;
            if classes.len() < 2
                || total <= 0.0
                || !disagree(classes[0].2, classes[1].2)
                || classes[0].1 - classes[1].1 > self.config.pay_split * total
            {
                out.push(e);
                continue;
            }
            split += 1;
            let half = e.prior / 2.0;
            let twin = Edge {
                index: e.index,
                rest: e.rest.clone(),
                path: e.path.clone(),
                token: e.token,
                prior: half,
                n: 0,
                w: 0.0,
                vl: 0,
                replies: 0,
                pruned: false,
                child: Child::None,
                keep: Some(std::sync::Arc::new(classes[1].0.clone())),
            };
            out.push(Edge { prior: half, keep: Some(std::sync::Arc::new(classes[0].0.clone())), ..e });
            out.push(twin);
        }
        self.pay_splits += split;
        out
    }

    /// What the seat wants to do next if it takes edge `e` of node `at`, for the autotapper to keep payable
    /// (`pay::FollowUps`): at a priority's first step, the node's other options that cost mana, each weighted by its
    /// share of the node's visits, or by its prior while the node has none; at a later step of the priority (a target),
    /// what its first step said (`Search::follow`). Nothing elsewhere.
    fn follow_ups(&mut self, at: usize, e: usize) -> pay::FollowUps {
        let node = &self.nodes[at];
        let key = (node.world, node.pos);
        let position = &self.forks.worlds[node.world].positions[node.pos];
        if !matches!(position.request.kind, RequestKind::Priority { .. }) {
            return Vec::new();
        }
        if !node.machine.chosen().is_empty() {
            return self.follow.get(&key).cloned().unwrap_or_default();
        }
        // The options' sources are read where the position is (the world's game may be elsewhere in its tree).
        let at_node = position.node;
        self.forks.worlds[key.0].game.goto(at_node);
        let node = &self.nodes[at];
        let position = &self.forks.worlds[node.world].positions[node.pos];
        let game = &self.forks.worlds[node.world].game;
        let visits: u32 = node.edges.iter().map(|e| e.n).sum();
        let mut out: pay::FollowUps = Vec::new();
        for (k, edge) in node.edges.iter().enumerate() {
            if k == e || edge.token.kind != Tok::Option || edge.pruned {
                continue;
            }
            let Some(option) = position.answering.priority_option(&position.request, edge.token.n) else { continue };
            let Some(cost) = pay::option_cost(game, option) else { continue };
            let weight = if visits > 0 { edge.n as f32 / visits as f32 } else { edge.prior };
            if weight > 0.0 {
                out.push((cost, weight));
            }
        }
        self.follow.insert(key, out.clone());
        out
    }

    /// The tree's shape (`Shape`), walked over the visited edges once the simulations are done.
    fn shape(&self) -> Shape {
        let mut shape = Shape::default();
        let mut line = true;
        for w in self.forks.worlds.iter().filter(|w| !w.dead) {
            let turns = |n: usize| w.positions[self.nodes[n].pos].turn - w.root_turn;
            let below = |n: usize, e: &Edge| match e.child {
                Child::Node(c) if e.n > 0 => Some((c, usize::from(self.nodes[c].pos != self.nodes[n].pos))),
                _ => None,
            };
            let root = &self.nodes[w.root];
            shape.root_options = shape.root_options.max(root.edges.len());
            shape.root_visited = shape.root_visited.max(root.edges.iter().filter(|e| e.n > 0).count());
            let mut stack = vec![(w.root, 1, 1)];
            while let Some((n, steps, decisions)) = stack.pop() {
                shape.deepest_steps = shape.deepest_steps.max(steps);
                shape.deepest_decisions = shape.deepest_decisions.max(decisions);
                shape.deepest_turns = shape.deepest_turns.max(turns(n));
                for e in &self.nodes[n].edges {
                    if let Some((c, new)) = below(n, e) {
                        stack.push((c, steps + 1, decisions + new));
                    }
                }
            }
            if line {
                line = false;
                let (mut n, mut steps, mut decisions) = (w.root, 1, 1);
                while let Some((c, new)) = self.nodes[n].edges.iter().max_by_key(|e| e.n).and_then(|e| below(n, e)) {
                    (n, steps, decisions) = (c, steps + 1, decisions + new);
                }
                (shape.line_steps, shape.line_decisions, shape.line_turns) = (steps, decisions, turns(n));
            }
        }
        shape
    }

    /// The worlds' agreement at the root (`Fusion`): ties within a world go to the first option, as the answer's do.
    fn fusion(&self) -> Option<Fusion> {
        let mut votes: BTreeMap<Token, usize> = BTreeMap::new();
        let mut worlds = 0;
        for w in self.forks.worlds.iter().filter(|w| !w.dead) {
            let edges = &self.nodes[w.root].edges;
            let Some(best) = (0..edges.len()).filter(|&k| edges[k].n > 0).max_by(|&a, &b| edges[a].n.cmp(&edges[b].n).then(b.cmp(&a))) else {
                continue;
            };
            worlds += 1;
            *votes.entry(edges[best].token).or_default() += 1;
        }
        (worlds >= 2).then(|| Fusion { worlds, distinct: votes.len(), plurality: votes.values().copied().max().unwrap_or(0) })
    }

    /// The edge of `at` to take: the most `Q + U`, none if every edge is dead.
    fn best(&self, at: usize) -> Option<usize> {
        let node = &self.nodes[at];
        let total: u32 = node.edges.iter().map(|e| e.n + e.vl).sum();
        let sqrt = (total.max(1) as f32).sqrt();
        let c = self.config.c_puct;
        let mut best: Option<(usize, f32)> = None;
        for (k, e) in node.edges.iter().enumerate() {
            if e.pruned || e.child == Child::Dead {
                continue;
            }
            let visits = e.n + e.vl;
            let q = if visits == 0 {
                node.value
            } else {
                (e.w - e.vl as f32) / visits as f32
            };
            let score = q + c * e.prior * sqrt / (1.0 + visits as f32);
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((k, score));
            }
        }
        best.map(|b| b.0)
    }

    fn select(&mut self, world: usize) -> Selected {
        let mut path = Vec::new();
        let mut at = self.forks.worlds[world].root;
        loop {
            if !self.nodes[at].expanded {
                if self.nodes[at].queued {
                    return Selected::Collision(path);
                }
                self.nodes[at].queued = true;
                return Selected::Leaf(at, path);
            }
            let Some(e) = self.best(at) else {
                return Selected::Dead(path);
            };
            self.nodes[at].edges[e].vl += 1;
            path.push((at, e));
            if self.nodes[at].edges[e].child == Child::None {
                let Some(child) = self.materialize(at, e) else {
                    return Selected::OutOfTime(path);
                };
                self.nodes[at].edges[e].child = child;
            }
            match self.nodes[at].edges[e].child {
                Child::Node(c) => at = c,
                Child::Terminal(v) => return Selected::Terminal(v, self.nodes[at].seat, path),
                Child::Dead => {
                    // Refused: take the virtual loss back and choose again here.
                    self.nodes[at].edges[e].vl -= 1;
                    path.pop();
                }
                Child::None => unreachable!(),
            }
        }
    }

    fn unwind(&mut self, path: &[(usize, usize)]) {
        for &(n, e) in path {
            self.nodes[n].edges[e].vl -= 1;
        }
    }

    fn backup(&mut self, path: &[(usize, usize)], seat: PlayerId, v: f32) {
        for &(n, e) in path {
            let same = self.nodes[n].seat == seat;
            let child_seat = match self.nodes[n].edges[e].child {
                Child::Node(c) => Some(self.nodes[c].seat),
                _ => None,
            };
            let opponent_below = !same || child_seat.is_some_and(|s| s != self.nodes[n].seat);
            let x = if same { v } else { -v };
            let edge = &mut self.nodes[n].edges[e];
            edge.vl -= 1;
            edge.n += 1;
            edge.w += x;
            if opponent_below {
                edge.replies = edge.replies.saturating_add(1);
            }
        }
    }

    /// Cuts edges of one searched path that the search has already seen enough of, from the root down. Once an edge
    /// is cut, nothing below it is reachable, so the rest of the path is left alone. With `prune_reversible`, first
    /// restores each cut edge at the path's nodes whose trim no longer holds against its siblings as they stand now.
    fn prune(&mut self, path: &[(usize, usize)]) {
        if !self.config.prune {
            return;
        }
        if self.config.prune_reversible {
            for &(n, _) in path {
                for k in 0..self.nodes[n].edges.len() {
                    if self.nodes[n].edges[k].pruned && self.trim_holds(n, k).is_none() {
                        self.nodes[n].edges[k].pruned = false;
                        self.restored += 1;
                    }
                }
            }
        }
        for &(n, e) in path {
            if self.nodes[n].edges[e].pruned {
                break;
            }
            let Some(kind) = self.prune_kind(n, e) else {
                continue;
            };
            self.nodes[n].edges[e].pruned = true;
            self.pruned += 1;
            match kind {
                PruneKind::Immediate => self.pruned_immediate += 1,
                PruneKind::Shallow => self.pruned_shallow += 1,
            }
            break;
        }
    }

    /// Whether the edge is to be cut, and why. The value is the edge's mean for the seat choosing at the node, so a
    /// node of either seat trims the moves bad for itself; the best visited edge at a node is never cut.
    fn prune_kind(&self, n: usize, e: usize) -> Option<PruneKind> {
        if self.nodes[n].edges[e].pruned {
            return None;
        }
        self.trim_holds(n, e)
    }

    /// Whether the trim rule holds of edge `e` of node `n` against its uncut visited siblings now, cut or not.
    fn trim_holds(&self, n: usize, e: usize) -> Option<PruneKind> {
        let node = &self.nodes[n];
        let edge = &node.edges[e];
        if edge.vl > 0
            || edge.n == 0
            || matches!(edge.child, Child::None | Child::Dead)
        {
            return None;
        }

        let mut best: Option<f32> = None;
        for (k, other) in node.edges.iter().enumerate() {
            if k == e || other.pruned || other.n == 0 || other.child == Child::Dead {
                continue;
            }
            let q = other.w / other.n as f32;
            best = Some(best.map_or(q, |b: f32| b.max(q)));
        }

        let best = best?;
        let q = edge.w / edge.n as f32;
        trim(q, best, edge.n, edge.replies, self.config)
    }

    /// Whether node `at` is a priority's first step to search as whole answers, and them (`Flat`).
    fn flat_plan(&self, at: usize) -> Option<Flat> {
        let node = &self.nodes[at];
        let pos = &self.forks.worlds[node.world].positions[node.pos];
        if !self.config.atomic
            || self.config.flatten == 0
            || !node.machine.chosen().is_empty()
            || !matches!(pos.request.kind, RequestKind::Priority { .. })
        {
            return None;
        }
        flatten(&node.machine, self.config.flatten).filter(|f| !f.inner.is_empty() || f.fallbacks > 0)
    }

    /// The encoding of node `at`'s position at the step `m` is at.
    fn encoded_at(&self, at: usize, m: &Machine) -> encode::Encoded {
        let node = &self.nodes[at];
        let pos = &self.forks.worlds[node.world].positions[node.pos];
        encode::encode(&pos.obs, m.step().expect("an inner step is at a step"), m.chosen(), &m.names(), self.forks.table)
    }

    fn expand(&mut self, at: usize, value: f32, logits: &[f32], flat: Option<Flat>, inner: &[Vec<f32>]) {
        let t = self.config.prior_temperature.max(1e-3);
        if let Some(flat) = flat {
            self.flattened += 1;
            self.flatten_fallbacks += flat.fallbacks;
            let node = &self.nodes[at];
            let step = node.machine.step().expect("a node is at a step");
            let first = softmax_legal(&step, logits, t);
            let levels: Vec<BTreeMap<usize, f32>> = flat
                .inner
                .iter()
                .zip(inner)
                .map(|(m, l)| softmax_legal(&m.step().expect("an inner step"), l, t))
                .collect();
            let mut edges: Vec<Edge> = flat
                .combos
                .into_iter()
                .map(|c| {
                    let mut prior = first.get(&c.first).copied().unwrap_or(0.0);
                    for &(k, choice) in &c.steps {
                        prior *= levels[k].get(&choice).copied().unwrap_or(0.0);
                    }
                    Edge {
                        index: c.first,
                        rest: c.rest,
                        token: c.path[0],
                        path: c.path,
                        prior,
                        n: 0,
                        w: 0.0,
                        vl: 0,
                        replies: 0,
                        pruned: false,
                        child: Child::None,
                        keep: None,
                    }
                })
                .collect();
            // The shortcut judges each option as a whole: its whole answers' priors summed.
            let mut shortcut = false;
            if edges.iter().any(|e| e.token.kind == Tok::Pass)
                && first.iter().all(|(&i, &p)| step.tokens[i].kind == Tok::Pass || p < self.config.shortcut)
            {
                edges.retain(|e| e.token.kind == Tok::Pass);
                edges.truncate(1);
                edges[0].prior = 1.0;
                shortcut = true;
            }
            if !shortcut && self.config.pay_split > 0.0 && !self.config.pay_classic {
                edges = self.split_by_payment(at, edges);
            }
            let node = &mut self.nodes[at];
            node.edges = edges;
            node.value = value;
            node.expanded = true;
            node.queued = false;
            node.shortcut = shortcut;
            return;
        }
        let node = &self.nodes[at];
        let step = node.machine.step().expect("a node is at a step");
        let legal: Vec<usize> = step.legal().collect();
        let logit = |i: usize| logits.get(i).copied().unwrap_or(MASKED);
        let top = legal.iter().map(|&i| logit(i)).fold(f32::MIN, f32::max);
        let weights: Vec<f32> = legal
            .iter()
            .map(|&i| ((logit(i) - top) / t).exp())
            .collect();
        let sum: f32 = weights.iter().sum::<f32>().max(1e-30);
        let mut edges: Vec<Edge> = legal
            .iter()
            .zip(&weights)
            .map(|(&i, &w)| Edge {
                index: i,
                rest: Vec::new(),
                path: vec![step.tokens[i]],
                token: step.tokens[i],
                prior: w / sum,
                n: 0,
                w: 0.0,
                vl: 0,
                replies: 0,
                pruned: false,
                child: Child::None,
                keep: None,
            })
            .collect();
        let pos = &self.forks.worlds[node.world].positions[node.pos];
        let mut shortcut = false;
        if node.machine.chosen().is_empty()
            && matches!(pos.request.kind, RequestKind::Priority { .. })
            && edges.iter().any(|e| e.token.kind == Tok::Pass)
            && edges
                .iter()
                .all(|e| e.token.kind == Tok::Pass || e.prior < self.config.shortcut)
        {
            edges.retain(|e| e.token.kind == Tok::Pass);
            edges[0].prior = 1.0;
            shortcut = true;
        }
        let node = &mut self.nodes[at];
        node.edges = edges;
        node.value = value;
        node.expanded = true;
        node.queued = false;
        node.shortcut = shortcut;
    }

    /// `ReuseCheck`: every live world's re-rooted root against the same root built fresh in it.
    #[allow(clippy::too_many_arguments)]
    fn check_reroot(
        &mut self,
        game: &Game,
        request: &Request,
        obs: &Observation,
        knowledge: &Knowledge,
        root_guard: &CancelGuard,
        seat: PlayerId,
        evaluator: &dyn Evaluator,
    ) -> Result<ReuseCheck, EvalError> {
        let mut out = ReuseCheck::default();
        let log = std::env::var_os("CANTRIP_REUSE_CHECK_LOG").is_some();
        let mut pairs: Vec<(usize, usize, usize)> = Vec::new();
        let mut found: Vec<Vec<&'static str>> = Vec::new();
        for wi in 0..self.forks.worlds.len() {
            if self.forks.worlds[wi].dead {
                continue;
            }
            out.worlds += 1;
            let mut miss: Vec<&'static str> = Vec::new();
            let r = self.forks.worlds[wi].root;
            let w = &mut self.forks.worlds[wi];
            let kept = &w.positions[self.nodes[r].pos];
            let (node, kept_request, kept_knowledge, kept_guards, kept_runs, kept_obs) = (
                kept.node,
                kept.request.clone(),
                kept.knowledge.clone(),
                kept.guards.clone(),
                kept.runs.clone(),
                kept.obs.clone(),
            );
            w.game.goto(node);
            let players = w.game.state.players.len();
            // The same question, but for which of two alike cards is which object (`Knowledge::alike`).
            if kept_request.player != request.player || by_card(&w.game, &kept_request.kind) != by_card(game, &request.kind) {
                miss.push("request");
                if log {
                    eprintln!("reuse check request: kept {:?}\n  real {:?}", kept_request.kind, request.kind);
                }
            }
            // The seat's knowledge is the real game's, as a fresh deal takes it: the kept world's must be the same, but
            // for which of the seat's alike cards is which object (its library's order was dealt otherwise). Every other
            // seat's is folded from the world's history, as a fresh deal folds it.
            let mut ks = Vec::with_capacity(players);
            for (o, kept_k) in kept_knowledge.iter().enumerate() {
                if o == seat.index() {
                    let mut k = kept_k.clone();
                    k.update(&w.game);
                    if !k.alike(&w.game, knowledge, game) {
                        miss.push("seat knowledge");
                        if log {
                            eprintln!("reuse check seat knowledge: kept {k:?}\n  real {knowledge:?}");
                        }
                    }
                    ks.push(k);
                    continue;
                }
                let fresh = Knowledge::of(&w.game, PlayerId(o as u8));
                let mut k = kept_k.clone();
                k.update(&w.game);
                if k != fresh && !miss.contains(&"opponent knowledge") {
                    miss.push("opponent knowledge");
                }
                ks.push(fresh);
            }
            // The seat's guard: the real one, its boards the world's (as a fresh deal keys it), against the kept one, each
            // as it acts on the boards its own game can return to.
            let boards = world_boards(game, &w.game);
            let mut guards = vec![CancelGuard::default(); players];
            guards[seat.index()] = root_guard.clone();
            if !root_guard.is_empty() {
                guards[seat.index()].rekey(&boards);
            }
            let real_boards: Vec<u64> = boards.iter().map(|b| b.0).collect();
            let world_side: Vec<u64> = boards.iter().map(|b| b.1).collect();
            if kept_guards[seat.index()].canonical(&world_side) != root_guard.canonical(&real_boards) {
                miss.push("seat guard");
                if log {
                    eprintln!(
                        "reuse check seat guard: kept {:?}\n  real {:?}\n  boards {boards:?}",
                        kept_guards[seat.index()],
                        root_guard
                    );
                }
            }
            if (0..players).any(|o| o != seat.index() && kept_guards[o].canonical(&world_side) != guards[o].canonical(&world_side)) {
                miss.push("opponent guard");
            }
            if kept_runs[seat.index()].is_some() {
                miss.push("seat payment run");
            }
            if kept_runs.iter().enumerate().any(|(o, r)| o != seat.index() && r.is_some()) {
                miss.push("opponent payment run");
                if log {
                    eprintln!("reuse check payment run: {kept_runs:?} at {:?}", request.kind);
                }
            }
            if kept_obs != *obs {
                miss.push("observation");
            }
            let root = &self.nodes[r];
            if root.queued || root.edges.iter().any(|e| e.vl > 0) {
                miss.push("virtual loss");
            }
            if self.config.prune
                && self.config.prune_reversible
                && (0..root.edges.len()).any(|k| root.edges[k].pruned && self.trim_holds(r, k).is_none())
            {
                miss.push("stale cut");
            }
            let fresh = self.forks.position(wi, ks, vec![None; players], guards).and_then(|fp| {
                match forward(Machine::new(&self.forks.worlds[wi].positions[fp].answering.ask, &[])) {
                    Forward::Decide(m) => Some(self.node(wi, fp, m)),
                    _ => None,
                }
            });
            match fresh {
                Some(f) => {
                    let fp = self.nodes[f].pos;
                    if self.forks.worlds[wi].positions[fp].obs != kept_obs {
                        miss.push("observation");
                    }
                    if self.encoded(f) != self.encoded(r) {
                        miss.push("encoding");
                    }
                    pairs.push((r, f, found.len()));
                }
                None => miss.push("fresh root"),
            }
            miss.dedup();
            found.push(miss);
        }
        // The fresh roots evaluated as a leaf is, in one batch.
        let mut flats: Vec<Option<Flat>> = pairs.iter().map(|&(_, f, _)| self.flat_plan(f)).collect();
        let mut encoded: Vec<encode::Encoded> = Vec::new();
        let mut first_row = Vec::with_capacity(pairs.len());
        for (k, &(_, f, _)) in pairs.iter().enumerate() {
            first_row.push(encoded.len());
            encoded.push(self.encoded(f));
            if let Some(flat) = &flats[k] {
                for m in &flat.inner {
                    encoded.push(self.encoded_at(f, m));
                }
            }
        }
        if !encoded.is_empty() {
            let batch = evaluator.evaluate_positions(&encoded.iter().collect::<Vec<_>>())?;
            for (k, &(r, f, at)) in pairs.iter().enumerate() {
                let row = first_row[k];
                let flat = flats[k].take();
                let inner = flat.as_ref().map_or(0, |x| x.inner.len());
                self.expand(f, batch.value[row].clamp(-1.0, 1.0), &batch.policy[row], flat, &batch.policy[row + 1..row + 1 + inner]);
                let (kept, fresh) = (&self.nodes[r], &self.nodes[f]);
                if !kept.expanded {
                    continue;
                }
                let miss = &mut found[at];
                let dv = (kept.value - fresh.value).abs();
                out.value_diff = out.value_diff.max(dv);
                if dv > CHECK_TOLERANCE {
                    miss.push("value");
                }
                if kept.shortcut != fresh.shortcut {
                    miss.push("shortcut");
                }
                if kept.edges.len() != fresh.edges.len()
                    || kept.edges.iter().zip(&fresh.edges).any(|(a, b)| a.index != b.index || a.path != b.path || a.rest != b.rest)
                {
                    miss.push("options");
                } else {
                    let dp = kept.edges.iter().zip(&fresh.edges).map(|(a, b)| (a.prior - b.prior).abs()).fold(0.0f32, f32::max);
                    out.prior_diff = out.prior_diff.max(dp);
                    if dp > CHECK_TOLERANCE {
                        miss.push("prior");
                    }
                }
            }
        }
        for miss in found {
            for what in miss {
                *out.mismatches.entry(what.to_string()).or_default() += 1;
                if log {
                    eprintln!("reuse check: {what} differs at request {} of seat {:?}", request.id, seat);
                }
            }
        }
        Ok(out)
    }

    /// Node `at`'s position as a full state, for a perfect-information value (`search_valued`): the world's game there,
    /// observed by the seat in `InfoMode::Full`, at its request's first step. `None` where it cannot be observed.
    fn encoded_full(&mut self, at: usize) -> Option<encode::Encoded> {
        let (wi, pi, seat) = (self.nodes[at].world, self.nodes[at].pos, self.nodes[at].seat);
        let node = self.forks.worlds[wi].positions[pi].node;
        self.forks.worlds[wi].game.goto(node);
        let w = &self.forks.worlds[wi];
        let p = &w.positions[pi];
        let (obs, index) = observe_indexed(&w.game, &p.knowledge[seat.index()], seat, InfoMode::Full)?;
        let answering = Answering::new(&w.game, &p.request, &obs, &index);
        let m = Machine::new(&answering.ask, &[]);
        let step = m.step()?;
        Some(encode::encode(&obs, step, m.chosen(), &m.names(), self.forks.table))
    }

    fn encoded(&self, at: usize) -> encode::Encoded {
        let node = &self.nodes[at];
        let pos = &self.forks.worlds[node.world].positions[node.pos];
        let m = &node.machine;
        encode::encode(
            &pos.obs,
            m.step().expect("a node is at a step"),
            m.chosen(),
            &m.names(),
            self.forks.table,
        )
    }
}

/// The seat's answer to `request` by searching (see the module), within `config`'s budget; the network's alone
/// (`net::answer_by`) where the module says. `knowledge` is the seat's of the real game; `prior` deals Hidden mode's
/// worlds; `guard` is the seat's cancel guard in the real game, carried into every world; `clock` is what the deadline
/// is read from.
#[allow(clippy::too_many_arguments)]
pub fn search(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    config: &Config,
    prior: &Prior,
    guard: &CancelGuard,
    clock: &dyn Clock,
) -> Result<Decision, EvalError> {
    search_keeping(game, request, knowledge, mode, table, evaluator, config, prior, guard, clock, &mut None)
}

/// Re-roots a kept tree at the request `game` now asks `seat` (see `Kept`): each live world's root followed down the
/// answers the real game has been given since, to the node of this request. None, and the tree is let go, unless every
/// live world gets there with the seat's observation of the real game; and none across another seat's choice.
#[allow(clippy::type_complexity)]
fn reroot(
    k: Kept,
    game: &Game,
    request: &Request,
    obs: &Observation,
    seat: PlayerId,
    mode: InfoMode,
    count: usize,
    follow_mode: Follow,
) -> Option<(Vec<World>, Vec<Node>, Vec<WorldReport>, Reused)> {
    if k.seat != seat || k.mode != mode || k.worlds.iter().filter(|w| !w.dead).count() > count {
        return None;
    }
    // The answers since: the real game's path from the kept request to now.
    let path = game.tree.path(game.at);
    let from = path.iter().position(|&n| n == k.at)?;
    let answers: Vec<&(u32, PlayerId, Answer)> = path[from + 1..].iter().filter_map(|&n| game.tree.answer(n)).collect();
    if answers.is_empty() {
        return None;
    }
    let Kept { mut worlds, nodes, deals, chosen, request: root_request, .. } = k;
    let mut info = Reused::default();
    let mut roots = Vec::with_capacity(worlds.len());
    for w in worlds.iter_mut() {
        if w.dead {
            roots.push(None);
            continue;
        }
        info.offered += 1;
        match follow(w, &nodes, &answers, root_request, chosen.as_ref(), request, obs, seat, follow_mode, &mut info) {
            Ok(at) if fresh_for_others(w, &nodes, at, game, seat) => roots.push(Some(at)),
            Ok(_) => {
                *info.lost.entry(format!("{:?}", Lost::OtherSeatState)).or_default() += 1;
                roots.push(None);
            }
            // What another seat did is the same in every world: none follows it.
            Err(Lost::OpponentChoice | Lost::OpponentAction) => return None,
            Err(lost) => {
                if follow_mode == Follow::Own {
                    return None;
                }
                *info.lost.entry(format!("{lost:?}")).or_default() += 1;
                roots.push(None);
            }
        }
    }
    let mut principal: BTreeMap<Token, u32> = BTreeMap::new();
    for (w, root) in worlds.iter_mut().zip(roots) {
        let Some(r) = root else {
            w.dead = true;
            continue;
        };
        w.root = r;
        w.root_pos = nodes[r].pos;
        w.root_turn = w.positions[w.root_pos].turn;
        info.worlds += 1;
        for e in nodes[r].edges.iter().filter(|e| !e.pruned) {
            info.visits += e.n;
            *principal.entry(e.token).or_default() += e.n;
        }
    }
    if info.worlds == 0 {
        return None;
    }
    info.principal = principal
        .into_iter()
        .filter(|e| e.1 > 0)
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|e| e.0);
    let nodes = compact(&mut worlds, nodes);
    Some((worlds, nodes, deals, info))
}

/// A re-rooted tree with only what its new roots reach: the nodes below each live world's root, and the positions those
/// nodes are at, renumbered; a dead world keeps nothing. Without it a kept tree carries every node and position it ever
/// held from one request to the next, its memory growing through a game (a position holds every seat's knowledge and
/// the seat's observation). What is dropped no root could reach again: the search walks only down from the roots.
fn compact(worlds: &mut [World], nodes: Vec<Node>) -> Vec<Node> {
    let mut keep = vec![false; nodes.len()];
    let mut stack: Vec<usize> = worlds.iter().filter(|w| !w.dead).map(|w| w.root).collect();
    while let Some(n) = stack.pop() {
        if std::mem::replace(&mut keep[n], true) {
            continue;
        }
        for e in &nodes[n].edges {
            if let Child::Node(c) = e.child
                && !keep[c]
            {
                stack.push(c);
            }
        }
    }
    // Each world's positions in use, renumbered in order.
    let mut used: Vec<Vec<bool>> = worlds.iter().map(|w| vec![false; w.positions.len()]).collect();
    for (n, node) in nodes.iter().enumerate() {
        if keep[n] {
            used[node.world][node.pos] = true;
        }
    }
    let mut pos_map: Vec<Vec<usize>> = Vec::with_capacity(worlds.len());
    for (w, world) in worlds.iter_mut().enumerate() {
        if world.dead {
            world.positions = Vec::new();
            pos_map.push(Vec::new());
            continue;
        }
        used[w][world.root_pos] = true;
        let mut map = vec![usize::MAX; world.positions.len()];
        let mut kept = Vec::new();
        for (i, p) in std::mem::take(&mut world.positions).into_iter().enumerate() {
            if used[w][i] {
                map[i] = kept.len();
                kept.push(p);
            }
        }
        world.positions = kept;
        world.root_pos = map[world.root_pos];
        pos_map.push(map);
    }
    let mut node_map = vec![usize::MAX; nodes.len()];
    let mut out: Vec<Node> = Vec::with_capacity(keep.iter().filter(|k| **k).count());
    for (n, node) in nodes.into_iter().enumerate() {
        if keep[n] {
            node_map[n] = out.len();
            out.push(node);
        }
    }
    for node in &mut out {
        node.pos = pos_map[node.world][node.pos];
        for e in &mut node.edges {
            if let Child::Node(c) = e.child {
                e.child = Child::Node(node_map[c]);
            }
        }
    }
    for world in worlds.iter_mut().filter(|w| !w.dead) {
        world.root = node_map[world.root];
    }
    out
}

/// Whether every other seat's state at the re-rooted node `at` of world `w` is what a fresh deal of the world gives it:
/// no payment run, and a cancel guard that acts as an empty one on the boards the world can return to.
fn fresh_for_others(w: &mut World, nodes: &[Node], at: usize, game: &Game, seat: PlayerId) -> bool {
    let position = &w.positions[nodes[at].pos];
    let others = || (0..position.runs.len()).filter(|&o| o != seat.index());
    if others().any(|o| position.runs[o].is_some()) {
        return false;
    }
    if others().all(|o| position.guards[o].is_empty()) {
        return true;
    }
    let node = position.node;
    w.game.goto(node);
    let world_side: Vec<u64> = world_boards(game, &w.game).into_iter().map(|b| b.1).collect();
    let empty = CancelGuard::default().canonical(&world_side);
    let position = &w.positions[nodes[at].pos];
    others().all(|o| position.guards[o].canonical(&world_side) == empty)
}

/// Why a world could not be followed to the request now asked.
#[derive(Debug)]
enum Lost {
    /// Another seat had a choice on the way (`Follow::Own` does not follow it).
    OpponentChoice,
    /// Another seat did something other than pass on the way (`Follow::Passes` does not follow it).
    OpponentAction,
    /// An answer given is not one the world's factoring makes (a card the world did not deal, say).
    NotInWorld,
    /// The line of an answer given was never searched in the world (or ended or was refused there).
    Unsearched,
    /// The world reached another request, or the seat sees the real game otherwise than the world.
    Diverged,
    /// Another seat's state at the request is not what a fresh deal gives it: a payment run its line left open, or a
    /// cancel guard that is not empty on the boards the world can return to. A fresh deal knows neither (`ReuseCheck`).
    OtherSeatState,
}

/// The node of the request now asked, reached from world `w`'s root down the answers given since (see `reroot`).
#[allow(clippy::too_many_arguments)]
fn follow(
    w: &mut World,
    nodes: &[Node],
    answers: &[&(u32, PlayerId, Answer)],
    root_request: u32,
    chosen: Option<&(u32, FactoredAnswer)>,
    request: &Request,
    obs: &Observation,
    seat: PlayerId,
    follow_mode: Follow,
    info: &mut Reused,
) -> Result<usize, Lost> {
    let mut at = w.root;
    let offset = w.positions[w.root_pos].request.id.wrapping_sub(root_request);
    for &&(id, player, ref answer) in answers {
        let pos = nodes[at].pos;
        if w.positions[pos].request.id != id.wrapping_add(offset) {
            // Answered with no choice in this world: inside the edge already taken.
            continue;
        }
        if player != seat {
            let pass = matches!(answer, Answer::Pass);
            match follow_mode {
                Follow::Own => return Err(Lost::OpponentChoice),
                Follow::Passes if !pass => return Err(Lost::OpponentAction),
                _ => {}
            }
            info.opponent_choices += 1;
            if !pass {
                info.opponent_actions += 1;
            }
        }
        // The answer in the world's factoring, the world's game at its position: the engine's answer factored, or the
        // search's own choice.
        let node = w.positions[pos].node;
        w.game.goto(node);
        let position = &w.positions[pos];
        let factored = [
            position.answering.factored(&position.request, answer, None),
            chosen.filter(|(r, _)| *r == id).map(|(_, f)| f.clone()),
            Some(FactoredAnswer::Pass),
            Some(FactoredAnswer::Cancel),
        ]
        .into_iter()
        .flatten()
        .find(|f| position.answering.engine(&w.game, &position.request, f).is_ok_and(|a| a == *answer))
        .ok_or(Lost::NotInWorld)?;
        let names = Machine::new(&position.answering.ask, &[]).names();
        let tokens = position.answering.decompose(&names, &factored).map_err(|_| Lost::NotInWorld)?;
        // Down the request's sub-steps to the edge that finishes it.
        loop {
            let node = &nodes[at];
            let from = node.machine.chosen().len();
            let ahead = tokens.get(from..).ok_or(Lost::NotInWorld)?;
            let edge = node.edges.iter().find(|e| ahead.starts_with(&e.path)).ok_or(Lost::NotInWorld)?;
            match edge.child {
                Child::Node(c) if nodes[c].pos == pos => at = c,
                Child::Node(c) => {
                    at = c;
                    break;
                }
                _ => return Err(Lost::Unsearched),
            }
        }
    }
    let position = &w.positions[nodes[at].pos];
    if nodes[at].pos == w.root_pos
        || position.request.id != request.id.wrapping_add(offset)
        || position.request.player != seat
        || position.obs != *obs
    {
        return Err(Lost::Diverged);
    }
    Ok(at)
}

/// `search`, going on from the tree `kept` holds where `Config::reuse` asks and it can (`Kept`), and leaving this
/// search's tree in `kept` for the seat's next request.
#[allow(clippy::too_many_arguments)]
pub fn search_keeping(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    config: &Config,
    prior: &Prior,
    guard: &CancelGuard,
    clock: &dyn Clock,
    kept: &mut Option<Kept>,
) -> Result<Decision, EvalError> {
    search_valued(game, request, knowledge, mode, table, evaluator, None, config, prior, guard, clock, kept)
}

/// `search_keeping`, with each leaf's value from `value` where it is given: a perfect-information value network asked of
/// the leaf's world as a full state (`InfoMode::Full`: both hands, the libraries' order), the policy still `evaluator`'s
/// from what the seat knows. The root's value is the visits' mean over the worlds, which are dealt from the seat's
/// knowledge: the belief-weighted mean of the worlds' values.
#[allow(clippy::too_many_arguments)]
pub fn search_valued(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    value: Option<&dyn Evaluator>,
    config: &Config,
    prior: &Prior,
    guard: &CancelGuard,
    clock: &dyn Clock,
    kept: &mut Option<Kept>,
) -> Result<Decision, EvalError> {

    let deadline = config.network_deadline.or(config.deadline);
    let mut root_guard = guard.clone();
    let request = &root_guard.filtered(game, request);
    let request = &if config.atomic { crate::atomic::filter(game, request) } else { request.clone() };
    let network = || {
        net::answer_by(
            game,
            request,
            knowledge,
            mode,
            table,
            evaluator,
            config.choice,
            deadline,
        )
    };
    if !config.searches() {
        return network();
    }
    // A run still waiting that drew on the generator before this request: any answer runs it again from its start
    // (`Game::answer`), and a fork's generator is another (`Game::reseed`), so in every world it would draw otherwise -
    // a mulligan's shuffle and redraw dealt again, a hand the seat never had. No world can be both dealt anew and the
    // game the seat saw: the network answers alone. (The fix is in the engine: such a run asked in stages.)
    if random_before(game) {
        return network();
    }
    if mode == InfoMode::Hidden && prior.is_empty() {
        // An empty prior deals an opponent's unseen library as it is (`determinize`): the search would read it.
        return Err(EvalError(
            "a Hidden search needs a prior of the metagame's decks".into(),
        ));
    }
    let started = clock.now();
    let seat = request.player;
    let (obs, index) = observe_indexed(game, knowledge, seat, mode)
        .ok_or_else(|| EvalError("no observation of the request (it is deferred)".into()))?;
    let answering = Answering::new(game, request, &obs, &index);
    if !matches!(
        forward(Machine::new(&answering.ask, &[])),
        Forward::Decide(_)
    ) {
        return network();
    }
    let counted = crate::alloc::installed();
    let base = crate::alloc::live();
    crate::alloc::reset_peak();
    // What the tree holds (`Config::memory`): this thread's heap above what it held before the kept tree was grown.
    let floor = base - kept.as_ref().map_or(0, |k| k.bytes as isize);
    let held = || (crate::alloc::live() - floor).max(0) as usize;
    let bound = config.memory.filter(|_| counted);
    // A kept tree that already holds more than half the bound leaves the search too little room: it searches afresh.
    let kept_dropped = bound.is_some_and(|m| kept.as_ref().is_some_and(|k| k.bytes > m / 2));
    if kept_dropped {
        *kept = None;
    }

    let mut s = Search {
        config,
        forks: Forks { mode, table, turn_cap: config.turn_cap, worlds: Vec::new(), refused: 0, atomic: config.atomic, last_waste: false, unpayable: 0, t_play: Duration::ZERO, t_setup: Duration::ZERO, t_goto: Duration::ZERO, t_observe: Duration::ZERO, stop: config.deadline.map(|d| (clock, d)) },
        nodes: Vec::new(),
        pruned: 0,
        pruned_immediate: 0,
        pruned_shallow: 0,
        restored: 0,
        flattened: 0,
        flatten_fallbacks: 0,
        follow: Default::default(),
        pay_splits: 0,
    };
    let count = if mode == InfoMode::Full {
        1
    } else {
        config.worlds.max(1)
    };
    let players = game.state.players.len();
    let mut deals: Vec<WorldReport> = Vec::new();
    let mut reused: Option<Reused> = None;
    if matches!(config.reuse, Reuse::TopUp | Reuse::Add)
        && let Some(k) = kept.take()
        && let Some(r) = reroot(k, game, request, &obs, seat, mode, count, config.reuse_follow)
    {
        let (worlds, nodes, kept_deals, mut info) = r;
        s.forks.worlds = worlds;
        s.nodes = nodes;
        deals = kept_deals;
        // A cut at the new root is held only while its trim holds, as a fresh search's would be: the kept root's cuts
        // were last judged when it was an inner node.
        {
            for w in 0..s.forks.worlds.len() {
                if s.forks.worlds[w].dead {
                    continue;
                }
                let r = s.forks.worlds[w].root;
                // The seat's payment run as its last answer in the line left it: the seat's answer at the root replaces
                // it before anything reads it (`Forks::play`), and a fresh root has none.
                let root_pos = s.forks.worlds[w].root_pos;
                s.forks.worlds[w].positions[root_pos].runs[seat.index()] = None;
                if !(config.prune && config.prune_reversible) {
                    continue;
                }
                for k in 0..s.nodes[r].edges.len() {
                    if s.nodes[r].edges[k].pruned && s.trim_holds(r, k).is_none() {
                        s.nodes[r].edges[k].pruned = false;
                        s.restored += 1;
                    }
                }
            }
        }
        if config.reuse_check {
            info.check = Some(s.check_reroot(game, request, &obs, knowledge, &root_guard, seat, evaluator)?);
        }
        reused = Some(info);
    }
    let (mut fresh_checked, mut fresh_mismatched) = (0usize, 0usize);
    // Fresh worlds: all of them, or as many as a re-rooted tree lost, dealt from seeds of their own.
    let fresh: Vec<u64> = match &reused {
        None => (0..count as u64).collect(),
        // Seeds past every world the tree has held, so no world is dealt twice.
        Some(r) => (0..count.saturating_sub(r.worlds) as u64).map(|k| k + s.forks.worlds.len() as u64).collect(),
    };
    if let Some(r) = reused.as_mut() {
        r.refilled = fresh.len();
    }
    for w in fresh {
        let seed = splitmix(config.seed ^ splitmix(w + 1));
        let mut g = game.fork_for_search();
        g.reseed(seed);
        determinize(&mut g, knowledge, seat, mode, prior, &mut Rng::new(seed));
        if g.pending().is_none_or(|r| r.player != seat) {
            continue;
        }
        // The real guard's boards are this world's: hidden cards the world dealt changed their hashes.
        let mut guards = vec![CancelGuard::default(); players];
        guards[seat.index()] = root_guard.clone();
        if !root_guard.is_empty() {
            guards[seat.index()].rekey(&world_boards(game, &g));
        }
        let ks: Vec<Knowledge> = (0..players)
            .map(|p| {
                if p == seat.index() {
                    knowledge.clone()
                } else {
                    Knowledge::of(&g, PlayerId(p as u8))
                }
            })
            .collect();
        let root_turn = g.state.turn.number;
        if config.report || config.reuse != Reuse::None {
            deals.push(deal_of(&g, seat, seed));
        }
        s.forks.worlds.push(World {
            game: g,
            positions: Vec::new(),
            root: 0,
            dead: false,
            root_turn,
            root_pos: 0,
        });
        let world = s.forks.worlds.len() - 1;
        let pos = s.forks.position(world, ks, vec![None; players], guards);
        if config.reuse_check && let Some(p) = pos {
            fresh_checked += 1;
            if s.forks.worlds[world].positions[p].obs != obs {
                fresh_mismatched += 1;
                if std::env::var_os("CANTRIP_REUSE_CHECK_LOG").is_some() {
                    eprintln!("reuse check: a fresh world's root observation differs at request {} ({:?})", request.id, crate::kind_name(&request.kind));
                }
            }
        }
        let root = pos.and_then(|pos| {
            match forward(Machine::new(
                &s.forks.worlds[world].positions[pos].answering.ask,
                &[],
            )) {
                Forward::Decide(m) => Some(s.node(world, pos, m)),
                _ => None,
            }
        });
        match root {
            Some(r) => s.forks.worlds[world].root = r,
            None => s.forks.worlds[world].dead = true,
        }
    }

    // The simulations.
    let target = config.simulations.filter(|&n| n > 0).unwrap_or(usize::MAX);
    let batch = config.batch.max(1);
    let carried = match (config.reuse, &reused) {
        (Reuse::TopUp, Some(r)) => r.visits as usize,
        _ => 0,
    };
    let (mut sims, mut evaluations, mut batches) = (carried, 0usize, 0usize);
    let mut rows = 0usize;
    let mut waste_leaves = 0usize;
    let (mut encoding, mut evaluating) = (Duration::ZERO, Duration::ZERO);
    let mut next = 0usize;
    let past = |clock: &dyn Clock| config.deadline.is_some_and(|d| clock.now() >= d);
    let full = || bound.is_some_and(|m| held() >= m);
    let mut memory_bound = false;
    while sims < target {
        if past(clock) {
            break;
        }
        if full() {
            memory_bound = true;
            break;
        }
        let alive: Vec<usize> = (0..s.forks.worlds.len()).filter(|&w| !s.forks.worlds[w].dead).collect();
        if alive.is_empty() {
            break;
        }
        // Every root took the shortcut: nothing to search.
        if alive.iter().all(|&w| s.nodes[s.forks.worlds[w].root].shortcut) {
            break;
        }
        let mut leaves: Vec<(usize, Vec<(usize, usize)>)> = Vec::new();
        let mut progress = false;
        let mut tries = 0;
        // Out of time or memory before the batch is made: the leaves chosen so far are let go unevaluated.
        let mut stopped = false;
        while sims + leaves.len() < target && leaves.len() < batch && tries < 4 * batch {
            if past(clock) {
                stopped = true;
                break;
            }
            if full() {
                (stopped, memory_bound) = (true, true);
                break;
            }
            tries += 1;
            let alive: Vec<usize> = (0..s.forks.worlds.len()).filter(|&w| !s.forks.worlds[w].dead).collect();
            let Some(&world) = alive.get(next % alive.len().max(1)) else {
                break;
            };
            next += 1;
            match s.select(world) {
                Selected::Leaf(n, path) => leaves.push((n, path)),
                Selected::Terminal(v, seat, path) => {
                    s.backup(&path, seat, v);
                    s.prune(&path);
                    sims += 1;
                    progress = true;
                }
                Selected::Collision(path) => s.unwind(&path),
                Selected::OutOfTime(path) => {
                    s.unwind(&path);
                    stopped = true;
                    break;
                }
                Selected::Dead(path) => {
                    s.unwind(&path);
                    match path.last() {
                        Some(&(n, e)) => s.nodes[n].edges[e].child = Child::Dead,
                        None => s.forks.worlds[world].dead = true,
                    }
                    progress = true;
                }
            }
        }
        if stopped {
            for (n, path) in &leaves {
                s.unwind(path);
                s.nodes[*n].queued = false;
            }
            break;
        }
        if leaves.is_empty() {
            if progress {
                continue;
            }
            break;
        }
        let t0 = Instant::now();
        // Each leaf's own step, and the inner steps of a priority searched as whole answers (`Flat`), in one batch.
        let mut flats: Vec<Option<Flat>> = leaves.iter().map(|(n, _)| s.flat_plan(*n)).collect();
        let mut encoded: Vec<encode::Encoded> = Vec::with_capacity(leaves.len());
        let mut first_row = Vec::with_capacity(leaves.len());
        for (k, (n, _)) in leaves.iter().enumerate() {
            first_row.push(encoded.len());
            encoded.push(s.encoded(*n));
            if let Some(f) = &flats[k] {
                for m in &f.inner {
                    encoded.push(s.encoded_at(*n, m));
                }
            }
        }
        let t1 = Instant::now();
        let out = evaluator.evaluate_positions(&encoded.iter().collect::<Vec<_>>())?;
        encoding += t1 - t0;
        evaluating += t1.elapsed();
        batches += 1;
        evaluations += leaves.len();
        rows += encoded.len();
        waste_leaves += leaves.iter().filter(|(n, _)| s.nodes[*n].waste).count();
        // The perfect-information value of each leaf's world, where one is given.
        let full_values: Option<Vec<Option<f32>>> = match value {
            Some(v) => {
                let full: Vec<Option<encode::Encoded>> = leaves.iter().map(|(n, _)| s.encoded_full(*n)).collect();
                let present: Vec<&encode::Encoded> = full.iter().flatten().collect();
                if present.is_empty() {
                    None
                } else {
                    let t2 = Instant::now();
                    let vout = v.evaluate(&FeatureBatch::new(&present))?;
                    evaluating += t2.elapsed();
                    rows += present.len();
                    let mut values = vout.value.into_iter();
                    Some(full.iter().map(|e| e.as_ref().and_then(|_| values.next())).collect())
                }
            }
            None => None,
        };
        for (k, (n, path)) in leaves.iter().enumerate() {
            let r = first_row[k];
            let v = full_values
                .as_ref()
                .and_then(|f| f[k])
                .unwrap_or(out.value[r]);
            // A value that is no number would back a NaN up the tree into the root's value (a game's root_values written
            // null): an error to see, never a value to search on. Inputs held to the f16's range (`f16_bits`) keep it so.
            if !v.is_finite() {
                return Err(EvalError(format!("the network valued a leaf {v}: not a number to search on")));
            }
            let v = v.clamp(-1.0, 1.0);
            let flat = flats[k].take();
            let inner = flat.as_ref().map_or(0, |f| f.inner.len());
            s.expand(*n, v, &out.policy[r], flat, &out.policy[r + 1..r + 1 + inner]);
            let seat = s.nodes[*n].seat;
            s.backup(path, seat, v);
        }
        for (_, path) in &leaves {
            s.prune(path);
        }
        sims += leaves.len();
    }

    // The root's visits, summed over the worlds by prefix and token.
    let mut visits: BTreeMap<Vec<Token>, BTreeMap<Token, u32>> = BTreeMap::new();
    // Whole answers' visits at the steps where the root's priority was searched as whole answers (`Flat`).
    let mut wholes: BTreeMap<Vec<Token>, BTreeMap<Vec<Token>, u32>> = BTreeMap::new();
    let (mut root_w, mut root_n) = (0.0f64, 0u64);
    for node in &s.nodes {
        if node.pos != s.forks.worlds[node.world].root_pos || !node.expanded {
            continue;
        }
        let base = node.machine.chosen().to_vec();
        for e in &node.edges {
            if e.pruned {
                continue;
            }
            // A whole answer's visits count at each of its steps, as the recorded visits are kept by step.
            let mut prefix = base.clone();
            for &t in &e.path {
                *visits.entry(prefix.clone()).or_default().entry(t).or_default() += e.n;
                prefix.push(t);
            }
            if e.path.len() > 1 || !e.rest.is_empty() {
                *wholes.entry(base.clone()).or_default().entry(e.path.clone()).or_default() += e.n;
            }
        }
    }
    for w in &s.forks.worlds {
        if w.dead {
            continue;
        }
        for e in &s.nodes[w.root].edges {
            if e.pruned {
                continue;
            }
            root_w += e.w as f64;
            root_n += e.n as u64;
        }
    }
    let net_value = s
        .forks.worlds
        .iter()
        .find(|w| !w.dead)
        .map(|w| s.nodes[w.root].value);
    let shortcut = s
        .forks.worlds
        .iter()
        .filter(|w| !w.dead)
        .all(|w| s.nodes[w.root].shortcut)
        && net_value.is_some();
    let mut searched = Searched {
        simulations: sims - carried,
        evaluations,
        batches,
        nodes: s.nodes.len(),
        positions: s.forks.worlds.iter().map(|w| w.positions.len()).sum(),
        game_nodes: s
            .forks.worlds
            .iter()
            .map(|w| w.game.tree.len().saturating_sub(1))
            .sum(),
        tree_bytes: 0,
        counted,
        memory_bound,
        kept_dropped,
        root_value: if root_n > 0 {
            (root_w / root_n as f64) as f32
        } else {
            net_value.unwrap_or(0.0)
        },
        net_value: net_value.unwrap_or(0.0),
        worlds: s.forks.worlds.len(),
        visits: Vec::new(),
        shortcut,
        refused: s.forks.refused,
        pruned: s.pruned,
        pruned_immediate: s.pruned_immediate,
        pruned_shallow: s.pruned_shallow,
        restored: s.restored,
        shape: s.shape(),
        fusion: s.fusion(),
        elapsed: Duration::ZERO,
        report: None,
        reused: reused.clone(),
        rows,
        flattened: s.flattened,
        flatten_fallbacks: s.flatten_fallbacks,
        waste_leaves,
        unpayable: s.forks.unpayable,
        fresh_checked,
        fresh_mismatched,
        pay_splits: s.pay_splits,
        play_time: s.forks.t_play,
        setup_time: s.forks.t_setup,
        goto_time: s.forks.t_goto,
        observe_time: s.forks.t_observe,
        pay_keeps: {
            let mut keeps: Vec<(Vec<Token>, Vec<ObjectId>, u32)> = Vec::new();
            for w in s.forks.worlds.iter().filter(|w| !w.dead) {
                let root = &s.nodes[w.root];
                if !root.machine.chosen().is_empty() {
                    continue;
                }
                for e in root.edges.iter().filter(|e| !e.pruned) {
                    if let Some(k) = &e.keep {
                        match keeps.iter_mut().find(|(p, kk, _)| *p == e.path && kk[..] == k[..]) {
                            Some(entry) => entry.2 += e.n,
                            None => keeps.push((e.path.clone(), k.to_vec(), e.n)),
                        }
                    }
                }
            }
            keeps
        },
        root_edges: s.forks.worlds.iter().filter(|w| !w.dead).map(|w| s.nodes[w.root].edges.len()).max().unwrap_or(0),
    };
    let kept_deals = if config.reuse != Reuse::None { deals.clone() } else { Vec::new() };
    if config.report {
        let mut worlds = deals;
        let mut prefix = None;
        for (w, report) in s.forks.worlds.iter().zip(worlds.iter_mut()) {
            report.dead = w.dead;
            let root = &s.nodes[w.root];
            if w.dead || !root.expanded {
                continue;
            }
            prefix.get_or_insert_with(|| root.machine.chosen().to_vec());
            report.value = root.value;
            report.shortcut = root.shortcut;
            report.edges = root
                .edges
                .iter()
                .map(|e| EdgeReport {
                    token: e.token,
                    path: e.path.clone(),
                    prior: e.prior,
                    visits: e.n,
                    value_sum: e.w,
                    pruned: e.pruned,
                    dead: e.child == Child::Dead,
                })
                .collect();
        }
        searched.report = Some(Report { worlds, prefix: prefix.unwrap_or_default(), candidates: Vec::new() });
    }
    searched.tree_bytes = if counted {
        (crate::alloc::peak() - base).max(0) as usize
    } else {
        s.nodes
            .iter()
            .map(|n| std::mem::size_of::<Node>() + n.edges.len() * std::mem::size_of::<Edge>())
            .sum::<usize>()
            + searched.positions * std::mem::size_of::<Position>()
    };
    let mut keep = (config.reuse != Reuse::None).then(|| Kept {
        seat,
        mode,
        at: game.at,
        request: request.id,
        worlds: std::mem::take(&mut s.forks.worlds),
        nodes: std::mem::take(&mut s.nodes),
        deals: kept_deals,
        chosen: None,
        bytes: 0,
    });
    drop(s);
    if let Some(k) = keep.as_mut() {
        k.bytes = if counted { held() } else { 0 };
    }

    if visits.values().all(|v| v.values().all(|&n| n == 0)) {
        *kept = keep;
        let mut d = network()?;
        searched.elapsed = clock.now().saturating_duration_since(started);
        d.search = Some(searched);
        return Ok(d);
    }

    if let Some(report) = searched.report.as_mut() {
        let t0 = Instant::now();
        let (made, evaluated) = candidates(game, request, &obs, &answering, table, evaluator, &visits, &wholes, &report.prefix)?;
        report.candidates = made;
        evaluating += t0.elapsed();
        evaluations += evaluated;
    }

    // The answer, walked in the real game's factoring.
    let mut random = match config.choice {
        Choice::Sample { seed, .. } => seed | 1,
        Choice::Best => 1,
    };
    let mut m = Machine::new(&answering.ask, &[]);
    let mut steps = 0;
    let mut extra = 0;
    // A whole answer chosen at a step searched as whole answers: its tokens, taken in order.
    let mut planned: std::collections::VecDeque<Token> = std::collections::VecDeque::new();
    while let Some(step) = m.step() {
        let legal: Vec<usize> = step.legal().collect();
        if legal.is_empty() {
            break;
        }
        steps += 1;
        let prefix = m.chosen().to_vec();
        if planned.is_empty()
            && let Some(options) = wholes.get(&prefix)
            && options.values().any(|&n| n > 0)
        {
            // The step's options and their whole answers alike, each with its visits.
            let mut all: Vec<(Vec<Token>, u32)> = options.iter().map(|(p, &n)| (p.clone(), n)).collect();
            for &i in &legal {
                let t = step.tokens[i];
                if !all.iter().any(|(p, _)| p[0] == t) {
                    let n = visits.get(&prefix).and_then(|v| v.get(&t)).copied().unwrap_or(0);
                    all.push((vec![t], n));
                }
            }
            let k = match config.choice {
                Choice::Best => (0..all.len()).max_by(|&a, &b| all[a].1.cmp(&all[b].1).then(b.cmp(&a))).unwrap(),
                Choice::Sample { temperature, .. } => {
                    let logits: Vec<f32> = all.iter().map(|&(_, n)| if n == 0 { MASKED } else { (n as f32).ln() }).collect();
                    let open: Vec<usize> = (0..all.len()).filter(|&k| all[k].1 > 0).collect();
                    net::pick(&logits, &open, Choice::Sample { temperature, seed: 0 }, &mut random)
                }
            };
            planned.extend(all[k].0.iter().copied());
        }
        if let Some(&want) = planned.front()
            && let Some(&i) = legal.iter().find(|&&i| step.tokens[i] == want)
        {
            planned.pop_front();
            if legal.len() > 1 {
                let seen: Vec<u32> = legal
                    .iter()
                    .map(|&j| visits.get(&prefix).and_then(|v| v.get(&step.tokens[j])).copied().unwrap_or(0))
                    .collect();
                searched.visits.push(RootStep {
                    prefix,
                    tokens: legal.iter().map(|&j| step.tokens[j]).collect(),
                    visits: seen,
                    chosen: want,
                });
            }
            m.choose(i);
            continue;
        }
        planned.clear();
        let seen: Vec<u32> = legal
            .iter()
            .map(|&i| {
                visits
                    .get(&prefix)
                    .and_then(|v| v.get(&step.tokens[i]))
                    .copied()
                    .unwrap_or(0)
            })
            .collect();
        let i = if legal.len() == 1 {
            legal[0]
        } else if seen.iter().any(|&n| n > 0) {
            let k = match config.choice {
                Choice::Best => (0..legal.len())
                    .max_by(|&a, &b| seen[a].cmp(&seen[b]).then(b.cmp(&a)))
                    .unwrap(),
                Choice::Sample { temperature, .. } => {
                    // Weights visits^(1/T), as logits of ln(visits)/T.
                    let logits: Vec<f32> = seen
                        .iter()
                        .map(|&n| if n == 0 { MASKED } else { (n as f32).ln() })
                        .collect();
                    let open: Vec<usize> = (0..legal.len()).filter(|&k| seen[k] > 0).collect();
                    net::pick(
                        &logits,
                        &open,
                        Choice::Sample {
                            temperature,
                            seed: 0,
                        },
                        &mut random,
                    )
                }
            };
            searched.visits.push(RootStep {
                prefix,
                tokens: legal.iter().map(|&i| step.tokens[i]).collect(),
                visits: seen,
                chosen: step.tokens[legal[k]],
            });
            legal[k]
        } else {
            let t0 = Instant::now();
            let e = encode::encode(&obs, step, m.chosen(), &m.names(), table);
            let t1 = Instant::now();
            let out = evaluator.evaluate(&FeatureBatch::new(&[&e]))?;
            encoding += t1 - t0;
            evaluating += t1.elapsed();
            extra += 1;
            net::pick(&out.policy[0], &legal, config.choice, &mut random)
        };
        m.choose(i);
    }
    searched.elapsed = clock.now().saturating_duration_since(started);
    let factored = m
        .answer()
        .cloned()
        .ok_or_else(|| EvalError("the machine finished with no answer".into()))?;
    if let Some(k) = keep.as_mut() {
        k.chosen = Some((request.id, factored.clone()));
    }
    *kept = keep;
    match answering.engine(game, request, &factored) {
        Ok(answer) => Ok(Decision {
            answer,
            factored,
            value: searched.root_value,
            evaluations: evaluations + extra,
            rejected: Vec::new(),
            fallback: false,
            late: false,
            steps,
            encode: encoding,
            evaluate: evaluating,
            search: Some(searched),
        }),
        Err(r) => {
            let mut d = network()?;
            d.rejected.insert(0, r);
            d.evaluations += evaluations + extra;
            d.search = Some(searched);
            Ok(d)
        }
    }
}

/// A world's deal as the advisor reports it (`WorldReport`): each other seat's hand and library top, by card name.
fn deal_of(g: &Game, seat: PlayerId, seed: u64) -> WorldReport {
    let s = &g.state;
    let name = |id: &ObjectId| g.cards.get(s.obj(*id).card).name.to_string();
    let others: Vec<PlayerId> = s.seats().filter(|&p| p != seat).collect();
    WorldReport {
        seed,
        hands: others
            .iter()
            .map(|&p| (p.0 + 1, s.zone(Zone::hand(p)).iter().map(name).collect()))
            .collect(),
        library_tops: others
            .iter()
            .map(|&p| (p.0 + 1, s.zone(Zone::library(p)).iter().take(3).map(name).collect()))
            .collect(),
        ..WorldReport::default()
    }
}

/// The report's candidates (`Report::candidates`): the root's first sub-step's visited options, the most visited first
/// (at most five), each completed into a whole answer; and the evaluations that took.
#[allow(clippy::too_many_arguments)]
fn candidates(
    game: &Game,
    request: &Request,
    obs: &Observation,
    answering: &Answering,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    visits: &BTreeMap<Vec<Token>, BTreeMap<Token, u32>>,
    wholes: &BTreeMap<Vec<Token>, BTreeMap<Vec<Token>, u32>>,
    root: &[Token],
) -> Result<(Vec<Candidate>, usize), EvalError> {
    const MOST: usize = 5;
    let Some(first) = visits.get(root) else {
        return Ok((Vec::new(), 0));
    };
    // The root's options as searched: its whole answers (`Flat`), and the options searched in steps.
    let mut ranked: Vec<(Vec<Token>, u32)> = wholes.get(root).map_or_else(Vec::new, |w| w.iter().map(|(p, &n)| (p.clone(), n)).collect());
    for (&t, &n) in first {
        if !ranked.iter().any(|(p, _)| p[0] == t) {
            ranked.push((vec![t], n));
        }
    }
    ranked.retain(|e| e.1 > 0);
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(MOST);
    let mut evaluations = 0;
    let mut out = Vec::new();
    for (path, _) in ranked {
        let token = path[0];
        let mut planned: std::collections::VecDeque<Token> = path.iter().copied().collect();
        let mut m = Machine::new(&answering.ask, &[]);
        let mut random = 1u64;
        let mut taken = false;
        while let Some(step) = m.step() {
            let legal: Vec<usize> = step.legal().collect();
            if legal.is_empty() {
                break;
            }
            let prefix = m.chosen().to_vec();
            let i = if !taken && legal.len() > 1 && prefix == root {
                taken = true;
                planned.pop_front();
                match legal.iter().find(|&&i| step.tokens[i] == token) {
                    Some(&i) => i,
                    None => break,
                }
            } else if taken && let Some(&want) = planned.front() {
                planned.pop_front();
                match legal.iter().find(|&&i| step.tokens[i] == want) {
                    Some(&i) => i,
                    None => break,
                }
            } else if legal.len() == 1 {
                legal[0]
            } else {
                let seen: Vec<u32> = legal
                    .iter()
                    .map(|&i| visits.get(&prefix).and_then(|v| v.get(&step.tokens[i])).copied().unwrap_or(0))
                    .collect();
                if seen.iter().any(|&n| n > 0) {
                    let k = (0..legal.len()).max_by(|&a, &b| seen[a].cmp(&seen[b]).then(b.cmp(&a))).unwrap();
                    legal[k]
                } else {
                    let e = encode::encode(obs, step, m.chosen(), &m.names(), table);
                    let out = evaluator.evaluate(&FeatureBatch::new(&[&e]))?;
                    evaluations += 1;
                    net::pick(&out.policy[0], &legal, Choice::Best, &mut random)
                }
            };
            m.choose(i);
        }
        let Some(factored) = m.answer().cloned().filter(|_| taken) else {
            continue;
        };
        let answer = answering
            .engine(game, request, &factored)
            .map_err(|r| format!("{}: {}", r.kind, r.why));
        out.push(Candidate { token, path, factored, answer });
    }
    Ok((out, evaluations))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_very_bad_edge_is_cut_at_once() {
        let c = Config::default();
        assert_eq!(trim(-0.9, 0.0, 1, 0, &c), Some(PruneKind::Immediate));
    }

    #[test]
    fn the_best_edge_is_never_cut() {
        let c = Config::default();
        assert_eq!(trim(-0.9, -0.9, 8, 8, &c), None);
        assert_eq!(trim(0.8, 0.7, 8, 8, &c), None);
    }

    #[test]
    fn an_edge_below_the_best_waits_for_a_look() {
        let c = Config::default();
        assert_eq!(trim(-0.5, 0.0, 1, 1, &c), None);
        assert_eq!(trim(-0.5, 0.0, 2, 0, &c), None);
        assert_eq!(trim(-0.5, 0.0, 2, 1, &c), Some(PruneKind::Shallow));
    }

    #[test]
    fn an_edge_near_the_best_is_not_cut() {
        let c = Config::default();
        assert_eq!(trim(-0.5, -0.4, 8, 8, &c), None);
    }
}
