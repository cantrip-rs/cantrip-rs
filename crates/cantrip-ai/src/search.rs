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
//! sibling's value fell below it.
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
/// bound catches only a runaway: a tree whose worlds' game logs grow past what the machine can hold for every seat.
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
    /// The most the search's tree may hold, in bytes, as the counting allocator counts them (`crate::alloc`; no bound
    /// where it is not installed). Reached, the search stops as at its deadline. Every position a world reaches keeps
    /// that world's game log (`Game::goto` moves through it, never forgets it), so without it a search could grow until
    /// the machine pages.
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
    /// fallen so that the edge would no longer be cut (see the module). Off, a cut is for good.
    pub prune_reversible: bool,
    /// Whether each world's root observation for the seat is compared with the real game's (`Searched::worlds_checked`,
    /// `worlds_mismatched`): a world the seat could tell from the game it is in is one a deal must never make. Off in play;
    /// tests turn it on. It changes no answer.
    pub check_worlds: bool,
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
            check_worlds: false,
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

/// The real game's boards a cancel guard can name that a world can return to - the root's, and the start of each run
/// still waiting there, which a take-back reverts to - each with the same board's hash in `world`, a fork of it dealt
/// anew: the world's hidden cards change the hash. Frames are paired by place, each reverted to its own start.
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

/// Whether a run still waiting in `game` drew on the generator since it began (`GameState::random_since`): re-run in a
/// fork for search, it would draw otherwise than the game did.
pub(crate) fn random_before(game: &Game) -> bool {
    game.state
        .agenda
        .iter()
        .filter(|f| f.pending.is_some() || !f.answers.is_empty())
        .any(|f| !f.staged && game.state.random_since(Mark(f.mark)))
}

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
    /// Whether the search stopped for `Config::memory` before its budget was spent.
    pub memory_bound: bool,
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
    pub elapsed: Duration,
    /// With `Config::check_worlds`: the worlds dealt at the root, and those whose root observation for the seat is not
    /// the real game's.
    pub worlds_checked: usize,
    pub worlds_mismatched: usize,
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
struct Position {
    node: tree::NodeId,
    request: Request,
    /// Every seat's knowledge at this position, the acting seat's up to date.
    knowledge: Vec<Knowledge>,
    obs: Observation,
    answering: Answering,
    /// Each seat's payment run in this line: the run's key and the answer its first request was given.
    runs: Vec<Option<(RunKey, FactoredAnswer)>>,
    /// Each seat's cancel guard in this line: the actions it has given up on, and what it is in the middle of.
    guards: Vec<CancelGuard>,
}

type RunKey = (PlayerId, u8, ObjectId, ObjectId, bool);

struct World {
    game: Game,
    positions: Vec<Position>,
    root: usize,
    dead: bool,
    root_turn: u32,
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
    /// The deadline passed while an edge on the way was being played out: the edge is left as it was, unplayed.
    OutOfTime(Vec<(usize, usize)>),
}

enum Forward {
    Decide(Machine),
    Done(FactoredAnswer),
    Stuck,
}

/// Takes every sub-step with one legal option.
fn forward(mut m: Machine) -> Forward {
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

fn splitmix(mut x: u64) -> u64 {
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
    mode: InfoMode,
    table: &'a CardIndex,
    config: &'a Config,
    worlds: Vec<World>,
    nodes: Vec<Node>,
    refused: usize,
    pruned: usize,
    pruned_immediate: usize,
    pruned_shallow: usize,
    /// Cut edges restored once their trim no longer held (`Config::prune_reversible`).
    restored: usize,
    /// The clock and the deadline a line stops at between the requests it plays with no choice; none, a line is played
    /// out.
    stop: Option<(&'a dyn Clock, Instant)>,
}

impl Search<'_> {
    /// The position for the request `world`'s game waits on, its seat's knowledge brought up to date and the actions
    /// its cancel guard has given up on filtered out.
    fn position(
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
        });
        Some(w.positions.len() - 1)
    }

    fn node(&mut self, world: usize, pos: usize, machine: Machine) -> usize {
        let seat = self.worlds[world].positions[pos].request.player;
        self.nodes.push(Node {
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
        match forward(m) {
            Forward::Decide(m) => Some(Child::Node(self.node(world, pos, m))),
            Forward::Stuck => {
                self.refused += 1;
                Some(Child::Dead)
            }
            Forward::Done(factored) => {
                let seat = self.nodes[at].seat;
                self.play(world, pos, factored, seat)
            }
        }
    }

    /// Plays `factored` at position `pos`, then every request after it that needs no choice. Each answer goes through
    /// its seat's cancel guard, so an action a line gives up is not offered again at the board it returns to. None if the
    /// deadline passed before the line reached a request with a choice: where it leads is not known yet.
    fn play(
        &mut self,
        world: usize,
        pos: usize,
        factored: FactoredAnswer,
        seat: PlayerId,
    ) -> Option<Child> {
        let cap = self.worlds[world].root_turn + self.config.turn_cap;
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
        let w = &mut self.worlds[world];
        w.game.goto(node);
        let actor = request.player.index();
        let Ok(answer) = answering.engine(&w.game, &request, &factored) else {
            self.refused += 1;
            return Some(Child::Dead);
        };
        let cancelled = guards[actor].before(&w.game, &request, &answer);
        if w.game
            .answer(request.id, request.player, answer.clone())
            .is_err()
        {
            self.refused += 1;
            return Some(Child::Dead);
        }
        guards[actor].after(&w.game, cancelled);
        runs[actor] = crate::record::run_key(&request).map(|k| (k, factored));
        for forced in 0usize.. {
            let w = &mut self.worlds[world];
            if w.game.is_over() {
                return Some(Child::Terminal(outcome_for(&w.game, seat)));
            }
            if forced >= MAX_FORCED || w.game.state.turn.number > cap {
                return Some(Child::Terminal(0.0));
            }
            // A line can run to the turn cap with no choice on the way, each request observed: the deadline is read
            // between them, not only between simulations.
            if self.stop.is_some_and(|(clock, d)| clock.now() >= d) {
                return None;
            }
            let Some(request) = w.game.pending().cloned() else {
                return Some(Child::Terminal(0.0));
            };
            let i = request.player.index();
            let request = guards[i].filtered(&w.game, &request);
            knowledge[i].update(&w.game);
            let Some((obs, index)) =
                observe_indexed(&w.game, &knowledge[i], request.player, self.mode)
            else {
                return Some(Child::Terminal(0.0));
            };
            let answering = Answering::new(&w.game, &request, &obs, &index);
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
                        return Some(Child::Terminal(0.0));
                    };
                    let cancelled = guards[i].before(&w.game, &request, &a);
                    if w.game
                        .answer(request.id, request.player, a.clone())
                        .is_err()
                    {
                        self.refused += 1;
                        return Some(Child::Terminal(0.0));
                    }
                    guards[i].after(&w.game, cancelled);
                    runs[i] = run.map(|k| (k, f));
                }
                Forward::Stuck => {
                    self.refused += 1;
                    return Some(Child::Terminal(0.0));
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
                    });
                    let pos = w.positions.len() - 1;
                    return Some(Child::Node(self.node(world, pos, m)));
                }
            }
        }
        unreachable!()
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
        let mut at = self.worlds[world].root;
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

    fn expand(&mut self, at: usize, value: f32, logits: &[f32]) {
        let t = self.config.prior_temperature.max(1e-3);
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
                token: step.tokens[i],
                prior: w / sum,
                n: 0,
                w: 0.0,
                vl: 0,
                replies: 0,
                pruned: false,
                child: Child::None,
            })
            .collect();
        let pos = &self.worlds[node.world].positions[node.pos];
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

    fn encoded(&self, at: usize) -> encode::Encoded {
        let node = &self.nodes[at];
        let pos = &self.worlds[node.world].positions[node.pos];
        let m = &node.machine;
        encode::encode(
            &pos.obs,
            m.step().expect("a node is at a step"),
            m.chosen(),
            &m.names(),
            self.table,
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
    let deadline = config.network_deadline.or(config.deadline);
    let mut root_guard = guard.clone();
    let request = &root_guard.filtered(game, request);
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
    // game the seat saw: the network answers alone.
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

    let mut s = Search {
        mode,
        table,
        config,
        worlds: Vec::new(),
        nodes: Vec::new(),
        refused: 0,
        pruned: 0,
        pruned_immediate: 0,
        pruned_shallow: 0,
        restored: 0,
        stop: config.deadline.map(|d| (clock, d)),
    };
    let count = if mode == InfoMode::Full {
        1
    } else {
        config.worlds.max(1)
    };
    let players = game.state.players.len();
    let (mut worlds_checked, mut worlds_mismatched) = (0usize, 0usize);
    for w in 0..count {
        let seed = splitmix(config.seed ^ splitmix(w as u64 + 1));
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
        s.worlds.push(World {
            game: g,
            positions: Vec::new(),
            root: 0,
            dead: false,
            root_turn,
        });
        let world = s.worlds.len() - 1;
        let pos = s.position(world, ks, vec![None; players], guards);
        if config.check_worlds
            && let Some(p) = pos
        {
            worlds_checked += 1;
            if s.worlds[world].positions[p].obs != obs {
                worlds_mismatched += 1;
            }
        }
        let root = pos.and_then(|pos| {
            match forward(Machine::new(
                &s.worlds[world].positions[pos].answering.ask,
                &[],
            )) {
                Forward::Decide(m) => Some(s.node(world, pos, m)),
                _ => None,
            }
        });
        match root {
            Some(r) => s.worlds[world].root = r,
            None => s.worlds[world].dead = true,
        }
    }

    // The simulations.
    let target = config.simulations.filter(|&n| n > 0).unwrap_or(usize::MAX);
    let batch = config.batch.max(1);
    let (mut sims, mut evaluations, mut batches) = (0usize, 0usize, 0usize);
    let (mut encoding, mut evaluating) = (Duration::ZERO, Duration::ZERO);
    let mut next = 0usize;
    let past = |clock: &dyn Clock| config.deadline.is_some_and(|d| clock.now() >= d);
    let bound = config.memory.filter(|_| counted);
    let full = || bound.is_some_and(|m| crate::alloc::live() - base >= m as isize);
    let mut memory_bound = false;
    while sims < target {
        if past(clock) {
            break;
        }
        if full() {
            memory_bound = true;
            break;
        }
        let alive: Vec<usize> = (0..s.worlds.len()).filter(|&w| !s.worlds[w].dead).collect();
        if alive.is_empty() {
            break;
        }
        // Every root took the shortcut: nothing to search.
        if alive.iter().all(|&w| s.nodes[s.worlds[w].root].shortcut) {
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
            let alive: Vec<usize> = (0..s.worlds.len()).filter(|&w| !s.worlds[w].dead).collect();
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
                        None => s.worlds[world].dead = true,
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
        let encoded: Vec<encode::Encoded> = leaves.iter().map(|(n, _)| s.encoded(*n)).collect();
        let t1 = Instant::now();
        let out = evaluator.evaluate(&FeatureBatch::new(&encoded.iter().collect::<Vec<_>>()))?;
        encoding += t1 - t0;
        evaluating += t1.elapsed();
        batches += 1;
        evaluations += leaves.len();
        for (k, (n, path)) in leaves.iter().enumerate() {
            let v = out.value[k].clamp(-1.0, 1.0);
            s.expand(*n, v, &out.policy[k]);
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
    let (mut root_w, mut root_n) = (0.0f64, 0u64);
    for node in &s.nodes {
        if node.pos != 0 || !node.expanded {
            continue;
        }
        let at = visits.entry(node.machine.chosen().to_vec()).or_default();
        for e in &node.edges {
            if e.pruned {
                continue;
            }
            *at.entry(e.token).or_default() += e.n;
        }
    }
    for w in &s.worlds {
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
        .worlds
        .iter()
        .find(|w| !w.dead)
        .map(|w| s.nodes[w.root].value);
    let shortcut = s
        .worlds
        .iter()
        .filter(|w| !w.dead)
        .all(|w| s.nodes[w.root].shortcut)
        && net_value.is_some();
    let mut searched = Searched {
        simulations: sims,
        evaluations,
        batches,
        nodes: s.nodes.len(),
        positions: s.worlds.iter().map(|w| w.positions.len()).sum(),
        game_nodes: s
            .worlds
            .iter()
            .map(|w| w.game.tree.len().saturating_sub(1))
            .sum(),
        tree_bytes: 0,
        counted,
        memory_bound,
        root_value: if root_n > 0 {
            (root_w / root_n as f64) as f32
        } else {
            net_value.unwrap_or(0.0)
        },
        net_value: net_value.unwrap_or(0.0),
        worlds: s.worlds.len(),
        visits: Vec::new(),
        shortcut,
        refused: s.refused,
        pruned: s.pruned,
        pruned_immediate: s.pruned_immediate,
        pruned_shallow: s.pruned_shallow,
        restored: s.restored,
        elapsed: Duration::ZERO,
        worlds_checked,
        worlds_mismatched,
    };
    searched.tree_bytes = if counted {
        (crate::alloc::peak() - base).max(0) as usize
    } else {
        s.nodes
            .iter()
            .map(|n| std::mem::size_of::<Node>() + n.edges.len() * std::mem::size_of::<Edge>())
            .sum::<usize>()
            + searched.positions * std::mem::size_of::<Position>()
    };
    drop(s);

    if visits.values().all(|v| v.values().all(|&n| n == 0)) {
        let mut d = network()?;
        searched.elapsed = clock.now().saturating_duration_since(started);
        d.search = Some(searched);
        return Ok(d);
    }

    // The answer, walked in the real game's factoring.
    let mut random = match config.choice {
        Choice::Sample { seed, .. } => seed | 1,
        Choice::Best => 1,
    };
    let mut m = Machine::new(&answering.ask, &[]);
    let mut steps = 0;
    let mut extra = 0;
    while let Some(step) = m.step() {
        let legal: Vec<usize> = step.legal().collect();
        if legal.is_empty() {
            break;
        }
        steps += 1;
        let prefix = m.chosen().to_vec();
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
