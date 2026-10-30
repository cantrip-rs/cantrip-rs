//! Information-set search: a prototype beside the PUCT search (`crate::search`), the
//! network's policy as the prior and its value at the leaves, solved by regret matching over a tree grown as GT-CFR
//! grows one, in the same forks (`search::Forks`).
//!
//! **Worlds are joint deals, on a grid.** The opponent's hidden cards are dealt from the seat's knowledge
//! (`determinize_owned`, the opponent's cards only), `Config::worlds / Config::hands` ways; the seat's own hand and
//! library `Config::hands` ways: the real hand (its library dealt again, which the seat does not know either) and
//! hands dealt from what the opponent knows of the seat's list. Every opponent deal meets every own deal, each player's
//! part dealt by a generator of its own, so the worlds that share an opponent deal differ only in the seat's hidden
//! cards, and the opponent's information sets there span the range of hands the seat may hold; the worlds that hold the
//! real hand differ only in the opponent's, and the seat's span its belief about them. A world is weighted as the root's
//! chance: an own deal by the probability the opponent's view gives it over how often the grid holds it (the real hand
//! is held once in `hands` whatever its probability, so that the seat's own information sets are reached at all).
//!
//! **Information sets are canonical observations.** A node is one sub-step of a request in a world, as in the PUCT
//! search; its key is the acting seat and the bytes the network reads there (`encode::to_compact`: the seat's
//! observation, the step's options and the choices made so far in the answer). The encoding makes two games the seat
//! cannot tell apart byte-identical, so nodes of different worlds with one key share one regret and strategy table,
//! and one evaluation: the network is asked once per key.
//! Where the encoding forgets (the oldest events, a pooled zone) the key is coarser than the information state: the
//! network forgets the same.
//!
//! **The tree is grown, then solved, in turns.** Each expansion batch takes `Config::batch` simulations: from a world's
//! root (the worlds in turn), down the tree by the current strategy mixed with the prior, to an option whose child is
//! not in the tree yet; that child is played in the world (`Forks::play`) and, where it is a decision, evaluated by the
//! network (its value for its seat, its prior over its options; an information set keeps the `Config::top` likeliest
//! options). After each batch, `Config::iterations` iterations of CFR+ (regret matching with regrets held at zero or
//! above, the average strategy weighted by iteration) run over every world's tree, each world weighted as above. An
//! option whose child is not in the tree yet is valued at its node's own value; a line that ends is worth its result
//! (1 won, -1 lost, 0 a draw the rules make); a capped or stuck line 0 (rare: the horizon below comes long before the
//! cap).
//!
//! **The horizon is one round**: a decision past the seat's next main phase after the opponent's next turn is a
//! boundary leaf, valued by the network and never expanded. With `Config::extend` (the adaptive horizon) a boundary
//! leaf that is hot - something on the stack, a creature attacking, lethal power on the board for either side, or a
//! race (both sides' clocks two turns or less) - is expanded all the same, up to `Config::extra_rounds` further rounds.
//! The hotness test reads the acting seat's observation only.
//!
//! **The answer** is the average strategy at the seat's real information set, walked through the request's sub-steps
//! (most likely, or sampled: `Choice`); a sub-step the tree never reached is the network's, as `net` takes it. The root
//! value is the expectation over the worlds under the current strategies; the range view is each world's root value
//! beside the opponent's hand it dealt.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::time::{Duration, Instant};

use cantrip_engine::*;
use cantrip_obs::encode::{self, to_compact};
use cantrip_obs::factor::{FactoredAnswer, Machine, Tok, Token};
use cantrip_obs::{InfoMode, ObjectObs, Observation, ZoneObs};

use crate::cancel::CancelGuard;
use crate::eval::{EvalError, Evaluator, FeatureBatch, MASKED};
use crate::net::{self, Choice, Decision};
use crate::search::{Forks, Forward, Reached, World, forward, splitmix};
use crate::{Answering, CardIndex, Knowledge, Prior, determinize_owned, observe_indexed};

/// How the search is spent.
#[derive(Clone, Debug)]
pub struct Config {
    pub worlds: usize,
    /// The seat's own deals: the real hand and `hands - 1` dealt from the opponent's view.
    pub hands: usize,
    /// Network evaluations, all worlds together.
    pub evaluations: usize,
    /// Options an information set keeps, by prior: at most `top`, none under `floor` but the likeliest, and at a
    /// priority's first sub-step only passing where every other option is under `shortcut` (the PUCT search's priority
    /// shortcut).
    pub top: usize,
    pub floor: f32,
    pub shortcut: f32,
    pub batch: usize,
    /// CFR+ iterations after each expansion batch.
    pub iterations: usize,
    /// The adaptive horizon (see the module), and how many rounds past the first it may reach.
    pub extend: bool,
    pub extra_rounds: u32,
    /// How much of a selection follows the current strategy rather than the prior.
    pub follow: f32,
    pub seed: u64,
    pub choice: Choice,
}

impl Default for Config {
    fn default() -> Config {
        Config { worlds: 16, hands: 4, evaluations: 512, top: 4, floor: 0.02, shortcut: 0.01, batch: 16, iterations: 8, extend: false, extra_rounds: 2, follow: 0.5, seed: 0x15ea2c4, choice: Choice::Best }
    }
}

/// What the search made of a request.
#[derive(Clone, Debug, Default)]
pub struct Searched {
    pub evaluations: usize,
    pub nodes: usize,
    pub infosets: usize,
    pub worlds: usize,
    pub own_worlds: usize,
    pub iterations: usize,
    /// Leaves at the horizon, and of those the hot ones the extension expanded.
    pub boundary: usize,
    pub hot: usize,
    pub capped: usize,
    pub refused: usize,
    /// How far the tree reached: the most turns past the root of any node, and of the real root's world along the
    /// likeliest line under the average strategies, its turns and decisions (requests with a choice).
    pub deepest_turns: u32,
    pub line_turns: u32,
    pub line_decisions: usize,
    /// The root's first sub-step: its options' tokens and the average strategy over them.
    pub root: Vec<(Token, f32)>,
    /// The root's value for the seat, its expectation over the worlds; and the network's.
    pub root_value: f32,
    pub net_value: f32,
    /// Each world: its weight, whether it keeps the real hand, its root value for the seat, and the opponent's hand it
    /// dealt (the range view).
    pub range: Vec<(f32, bool, f32, Vec<String>)>,
    pub elapsed: Duration,
}

type Key = u128;

/// Expansion batches a search is grown in at most, whatever its budget: each is followed by `Config::iterations` of
/// CFR+ over the whole tree, so more batches cost iterations in proportion to the tree.
const ROUNDS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Child {
    None,
    Node(usize),
    /// The line ends: its value for the node's seat.
    Value(f32),
    Dead,
}

struct Node {
    world: usize,
    pos: usize,
    machine: Machine,
    seat: PlayerId,
    key: Key,
    /// The network's value for the seat, once evaluated.
    value: Option<f32>,
    /// Options (indices into the step's options), index for index with the information set's.
    children: Vec<Child>,
    /// Past the horizon: evaluated, never expanded.
    boundary: bool,
}

struct InfoSet {
    options: Vec<usize>,
    tokens: Vec<Token>,
    prior: Vec<f32>,
    regret: Vec<f32>,
    total: Vec<f32>,
}

impl InfoSet {
    fn current(&self) -> Vec<f32> {
        let sum: f32 = self.regret.iter().sum();
        if sum > 0.0 { self.regret.iter().map(|r| r / sum).collect() } else { self.prior.clone() }
    }

    fn average(&self) -> Vec<f32> {
        let sum: f32 = self.total.iter().sum();
        if sum > 0.0 { self.total.iter().map(|t| t / sum).collect() } else { self.prior.clone() }
    }
}

struct Search<'a> {
    forks: Forks<'a>,
    config: &'a Config,
    seat: PlayerId,
    nodes: Vec<Node>,
    sets: HashMap<Key, InfoSet>,
    /// The network's outputs by key: its value and the step's logits.
    cache: HashMap<Key, (f32, Vec<f32>)>,
    weights: Vec<f32>,
    own: Vec<bool>,
    /// Each world's boundary turn, and the hard limit the extension may reach.
    boundary: (u32, u32),
    stats: Searched,
    rng: u64,
}

fn key_of(seat: PlayerId, bytes: &[u8]) -> Key {
    let hash = |seed: u64| {
        let mut h = foldhash::fast::FixedState::with_seed(seed).build_hasher();
        h.write_u8(seat.0);
        h.write(bytes);
        h.finish()
    };
    (u128::from(hash(1)) << 64) | u128::from(hash(2))
}

/// `ln C(n, k)`.
fn ln_choose(n: u32, k: u32) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    (1..=k).map(|i| ((n - k + i) as f64 / i as f64).ln()).sum()
}

/// The probability the opponent's view gives the seat's real hand: its hidden part a draw without replacement from
/// what the opponent cannot see of the seat's list (its Decklist observation's pool of the seat).
fn hand_probability(game: &Game, seat: PlayerId, opponent: PlayerId) -> f64 {
    let k = Knowledge::of(game, opponent);
    let Some((obs, index)) = observe_indexed(game, &k, opponent, InfoMode::Decklist) else {
        return 0.0;
    };
    let pool: HashMap<&str, u32> = obs.pools.get(seat.index()).map(|p| p.iter().map(|c| (c.name.as_str(), u32::from(c.count))).collect()).unwrap_or_default();
    let n: u32 = pool.values().sum();
    // The seat's hand as the opponent sees it: the hidden cards of it, by what they really are.
    let mut hidden: HashMap<String, u32> = HashMap::new();
    for (i, o) in obs.objects.iter().enumerate() {
        if let ObjectObs::Hidden { place, identity: None, .. } = o
            && place.zone == ZoneObs::Hand
            && place.player == seat.0
        {
            *hidden.entry(game.name(index[i]).to_string()).or_default() += 1;
        }
    }
    let h: u32 = hidden.values().sum();
    let mut lp = -ln_choose(n, h);
    for (name, &c) in &hidden {
        lp += ln_choose(pool.get(name.as_str()).copied().unwrap_or(0), c);
    }
    lp.exp()
}

/// Read from the acting seat's observation (see the module): whether a leaf is hot.
fn hot(obs: &Observation) -> bool {
    let mut power = vec![0i32; obs.players.len()];
    for o in &obs.objects {
        match o {
            ObjectObs::Ability { .. } => return true,
            ObjectObs::Visible(v) => {
                if v.place.zone == ZoneObs::Stack || v.attacking.is_some() {
                    return true;
                }
                if v.place.zone == ZoneObs::Battlefield && v.chars.types & (1 << 1) != 0 {
                    power[v.controller as usize] += i32::from(v.chars.power.unwrap_or(0).max(0));
                }
            }
            ObjectObs::Hidden { .. } => {}
        }
    }
    if obs.players.len() != 2 {
        return false;
    }
    let life = [obs.players[0].life, obs.players[1].life];
    let clock = |p: usize| if power[p] <= 0 { i32::MAX } else { (life[1 - p].max(1) + power[p] - 1) / power[p] };
    clock(0) <= 1 || clock(1) <= 1 || (clock(0) <= 2 && clock(1) <= 2)
}

impl Search<'_> {
    fn next_random(&mut self) -> f32 {
        self.rng = splitmix(self.rng);
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Whether a position is past `limit`: that turn's precombat main phase or later.
    fn past(&self, world: &World, pos: usize, limit: u32) -> bool {
        let turn = world.positions[pos].obs.turn;
        turn.number > limit || (turn.number == limit && turn.step >= Step::PrecombatMain as u8)
    }

    fn node(&mut self, world: usize, pos: usize, machine: Machine) -> usize {
        let w = &self.forks.worlds[world];
        let p = &w.positions[pos];
        let seat = p.request.player;
        let step = machine.step().expect("a node is at a step");
        let e = encode::encode(&p.obs, step, machine.chosen(), &machine.names(), self.forks.table);
        let key = key_of(seat, &to_compact(&e));
        let mut boundary = self.past(w, pos, self.boundary.0);
        if boundary && self.config.extend && !self.past(w, pos, self.boundary.1) && hot(&p.obs) {
            boundary = false;
            self.stats.hot += 1;
        }
        if boundary {
            self.stats.boundary += 1;
        }
        self.nodes.push(Node { world, pos, machine, seat, key, value: None, children: Vec::new(), boundary });
        self.nodes.len() - 1
    }

    fn encoded(&self, n: usize) -> encode::Encoded {
        let node = &self.nodes[n];
        let p = &self.forks.worlds[node.world].positions[node.pos];
        let m = &node.machine;
        encode::encode(&p.obs, m.step().expect("a node is at a step"), m.chosen(), &m.names(), self.forks.table)
    }

    /// The network's outputs for a node: its value, and its information set made (the first time a node of it is).
    fn expand(&mut self, n: usize, value: f32, logits: &[f32]) {
        let node = &self.nodes[n];
        let key = node.key;
        let step = node.machine.step().expect("a node is at a step");
        if !self.sets.contains_key(&key) {
            let legal: Vec<usize> = step.legal().collect();
            let logit = |i: usize| logits.get(i).copied().unwrap_or(MASKED);
            let top = legal.iter().map(|&i| logit(i)).fold(f32::MIN, f32::max);
            let all: f32 = legal.iter().map(|&i| (logit(i) - top).exp()).sum::<f32>().max(1e-30);
            let p = |i: usize| (logit(i) - top).exp() / all;
            let mut order = legal.clone();
            order.sort_by(|&a, &b| logit(b).total_cmp(&logit(a)).then(a.cmp(&b)));
            let priority = node.machine.chosen().is_empty() && matches!(self.forks.worlds[node.world].positions[node.pos].request.kind, RequestKind::Priority { .. });
            let pass = legal.iter().copied().find(|&i| step.tokens[i].kind == Tok::Pass);
            match pass {
                Some(pass) if priority && legal.iter().all(|&i| i == pass || p(i) < self.config.shortcut) => order = vec![pass],
                _ => {
                    let first = order[0];
                    order.retain(|&i| i == first || p(i) >= self.config.floor);
                    order.truncate(self.config.top.max(1));
                }
            }
            let w: Vec<f32> = order.iter().map(|&i| (logit(i) - top).exp()).collect();
            let sum: f32 = w.iter().sum::<f32>().max(1e-30);
            self.sets.insert(
                key,
                InfoSet {
                    tokens: order.iter().map(|&i| step.tokens[i]).collect(),
                    options: order,
                    prior: w.iter().map(|x| x / sum).collect(),
                    regret: vec![0.0; w.len()],
                    total: vec![0.0; w.len()],
                },
            );
        }
        let options = self.sets[&key].options.len();
        let node = &mut self.nodes[n];
        node.value = Some(value);
        node.children = vec![Child::None; options];
    }

    /// The child of node `n` by its information set's option `k`, played in its world if it is not yet.
    fn child(&mut self, n: usize, k: usize) -> Child {
        if self.nodes[n].children[k] != Child::None {
            return self.nodes[n].children[k];
        }
        let (world, pos, seat) = (self.nodes[n].world, self.nodes[n].pos, self.nodes[n].seat);
        let option = self.sets[&self.nodes[n].key].options[k];
        let mut m = self.nodes[n].machine.clone();
        m.choose(option);
        let child = match forward(m) {
            Forward::Decide(m) => Child::Node(self.node(world, pos, m)),
            Forward::Stuck => {
                self.forks.refused += 1;
                Child::Dead
            }
            Forward::Done(factored) => match self.forks.play(world, pos, factored, seat) {
                Reached::Decide(pos, m) => Child::Node(self.node(world, pos, m)),
                Reached::Over(v) => Child::Value(v),
                Reached::Capped | Reached::Stuck => {
                    self.stats.capped += 1;
                    Child::Value(0.0)
                }
                Reached::Dead => Child::Dead,
                Reached::OutOfTime => unreachable!("the information-set search plays its lines out (no `Forks::stop`)"),
            },
        };
        self.nodes[n].children[k] = child;
        child
    }

    /// One simulation from world `w`'s root: the node it reached that wants the network, if any.
    fn simulate(&mut self, world: usize) -> Option<usize> {
        let mut n = self.forks.worlds[world].root;
        for _ in 0..10_000 {
            if self.nodes[n].value.is_none() {
                let Some((v, logits)) = self.cache.get(&self.nodes[n].key).cloned() else {
                    return Some(n);
                };
                self.expand(n, v, &logits);
            }
            if self.nodes[n].boundary {
                return None;
            }
            let set = &self.sets[&self.nodes[n].key];
            let current = set.current();
            let prior = set.prior.clone();
            let live: Vec<usize> = (0..prior.len()).filter(|&k| self.nodes[n].children[k] != Child::Dead).collect();
            if live.is_empty() {
                return None;
            }
            // Mostly the current strategy, partly the prior, with an unvisited option preferred in proportion.
            let follow = self.config.follow;
            let weights: Vec<f32> = live.iter().map(|&k| follow * current[k] + (1.0 - follow) * prior[k] + 1e-6).collect();
            let total: f32 = weights.iter().sum();
            let mut x = self.next_random() * total;
            let mut pick = live[live.len() - 1];
            for (&k, &w) in live.iter().zip(&weights) {
                if x < w {
                    pick = k;
                    break;
                }
                x -= w;
            }
            match self.child(n, pick) {
                Child::Node(c) => n = c,
                _ => return None,
            }
        }
        None
    }

    /// One CFR+ iteration over world `w`'s tree; returns the root's value for the seat under the current strategies.
    fn cfr(&mut self, n: usize, reach: [f32; 2], weight: f32, t: f32) -> f32 {
        let node = &self.nodes[n];
        let value = node.value.unwrap_or(0.0);
        let mine = node.seat == self.seat;
        let for_seat = |v: f32| if mine { v } else { -v };
        if node.boundary || node.children.is_empty() {
            return for_seat(value);
        }
        let key = node.key;
        let current = self.sets[&key].current();
        let children = node.children.clone();
        let live: Vec<usize> = (0..children.len()).filter(|&k| children[k] != Child::Dead).collect();
        if live.is_empty() {
            return for_seat(value);
        }
        let mass: f32 = live.iter().map(|&k| current[k]).sum();
        let sigma: Vec<f32> = (0..children.len())
            .map(|k| if children[k] == Child::Dead { 0.0 } else if mass > 0.0 { current[k] / mass } else { 1.0 / live.len() as f32 })
            .collect();
        let me = usize::from(!mine);
        let mut values = vec![0.0f32; children.len()];
        let mut v = 0.0;
        for &k in &live {
            values[k] = match children[k] {
                Child::Node(c) => {
                    let mut r = reach;
                    r[me] *= sigma[k];
                    self.cfr(c, r, weight, t)
                }
                Child::Value(x) => for_seat(x),
                // Not in the tree yet: the node's own value.
                Child::None => for_seat(value),
                Child::Dead => unreachable!(),
            };
            v += sigma[k] * values[k];
        }
        // Regrets from the acting seat's side, weighted by the others' reach and the world's chance.
        let sign = if mine { 1.0 } else { -1.0 };
        let other = reach[1 - me] * weight;
        let own = reach[me] * weight;
        let set = self.sets.get_mut(&key).expect("an expanded node's set");
        for &k in &live {
            set.regret[k] = (set.regret[k] + other * sign * (values[k] - v)).max(0.0);
            set.total[k] += t * own * sigma[k];
        }
        v
    }

    fn iterate(&mut self, t: usize) {
        for w in 0..self.forks.worlds.len() {
            if self.forks.worlds[w].dead {
                continue;
            }
            let root = self.forks.worlds[w].root;
            self.cfr(root, [1.0, 1.0], self.weights[w], t as f32);
        }
        self.stats.iterations += 1;
    }

    /// Each world's root value for the seat under the current strategies, without updating anything.
    fn value(&self, n: usize) -> f32 {
        let node = &self.nodes[n];
        let mine = node.seat == self.seat;
        let for_seat = |v: f32| if mine { v } else { -v };
        let own = for_seat(node.value.unwrap_or(0.0));
        if node.boundary || node.children.is_empty() {
            return own;
        }
        let current = self.sets[&node.key].current();
        let live: Vec<usize> = (0..node.children.len()).filter(|&k| node.children[k] != Child::Dead).collect();
        let mass: f32 = live.iter().map(|&k| current[k]).sum();
        live.iter()
            .map(|&k| {
                let p = if mass > 0.0 { current[k] / mass } else { 1.0 / live.len() as f32 };
                p * match node.children[k] {
                    Child::Node(c) => self.value(c),
                    Child::Value(x) => for_seat(x),
                    _ => own,
                }
            })
            .sum()
    }
}

/// The seat's answer to `request` by the information-set search (see the module), Decklist mode. `knowledge` is the
/// seat's of the real game; `guard` its cancel guard there.
#[allow(clippy::too_many_arguments)]
pub fn search(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    config: &Config,
    guard: &CancelGuard,
) -> Result<(Decision, Searched), EvalError> {
    let started = Instant::now();
    let mode = InfoMode::Decklist;
    let mut root_guard = guard.clone();
    let request = &root_guard.filtered(game, request);
    let seat = request.player;
    let players = game.state.players.len();
    let opponent = PlayerId(((seat.index() + 1) % players) as u8);
    let network = || net::answer(game, request, knowledge, mode, table, evaluator, config.choice);
    let (obs, index) = observe_indexed(game, knowledge, seat, mode).ok_or_else(|| EvalError("no observation of the request".into()))?;
    let answering = Answering::new(game, request, &obs, &index);
    // A waiting run that drew on the generator would draw otherwise in every world (`search::random_before`).
    if !matches!(forward(Machine::new(&answering.ask, &[])), Forward::Decide(_)) || crate::search::random_before(game) {
        return network().map(|d| (d, Searched::default()));
    }
    let root_turn = game.state.turn.number;
    let first = if game.state.turn.active == seat { root_turn + 2 } else { root_turn + 3 };
    let mut s = Search {
        forks: Forks { mode, table, turn_cap: 40, worlds: Vec::new(), refused: 0, atomic: false, last_waste: false, unpayable: 0, t_play: Duration::ZERO, t_setup: Duration::ZERO, t_goto: Duration::ZERO, t_observe: Duration::ZERO, stop: None },
        config,
        seat,
        nodes: Vec::new(),
        sets: HashMap::new(),
        cache: HashMap::new(),
        weights: Vec::new(),
        own: Vec::new(),
        boundary: (first, first + 2 * config.extra_rounds),
        stats: Searched::default(),
        rng: splitmix(config.seed ^ 0x1505),
    };
    let prior = Prior::new();
    let hands = config.hands.clamp(1, config.worlds.max(1));
    let deals = config.worlds.max(1).div_ceil(hands);
    let mut ranges = Vec::new();
    for w in 0..deals * hands {
        let (deal, hand) = (w / hands, w % hands);
        let theirs_seed = splitmix(config.seed ^ splitmix(deal as u64 + 1));
        let own_seed = splitmix(config.seed ^ 0x0e1f ^ splitmix(hand as u64 + 1));
        let mut g = game.fork_for_search();
        g.reseed(splitmix(theirs_seed ^ own_seed));
        determinize_owned(&mut g, knowledge, seat, mode, &prior, &mut Rng::new(theirs_seed), Some(opponent));
        let own = hand == 0;
        if own {
            // The real hand; the seat's library, which it does not know either, dealt again.
            determinize_owned(&mut g, knowledge, seat, mode, &prior, &mut Rng::new(own_seed), Some(seat));
        } else {
            // The seat's hand and library as the opponent believes them.
            let theirs = Knowledge::of(&g, opponent);
            determinize_owned(&mut g, &theirs, opponent, mode, &prior, &mut Rng::new(own_seed), Some(seat));
        }
        if g.pending().is_none_or(|r| r.player != seat) {
            continue;
        }
        // The real hand is held in one deal of `hands`; a dealt one is drawn from the opponent's view in the rest.
        let weight = if own { (hand_probability(&g, seat, opponent) * hands as f64) as f32 } else { hands as f32 / (hands - 1) as f32 };
        let mut guards = vec![CancelGuard::default(); players];
        guards[seat.index()] = root_guard.clone();
        if !root_guard.is_empty() {
            guards[seat.index()].rekey(&crate::search::world_boards(game, &g));
        }
        let ks: Vec<Knowledge> = (0..players).map(|p| if p == seat.index() && own { knowledge.clone() } else { Knowledge::of(&g, PlayerId(p as u8)) }).collect();
        let hand: Vec<String> = g.state.zone(Zone::hand(opponent)).iter().map(|&id| g.name(id).to_string()).collect();
        let root_turn = g.state.turn.number;
        s.forks.worlds.push(World { game: g, positions: Vec::new(), root: 0, dead: false, root_turn, root_pos: 0 });
        let world = s.forks.worlds.len() - 1;
        let root = s.forks.position(world, ks, vec![None; players], guards).and_then(|pos| match forward(Machine::new(&s.forks.worlds[world].positions[pos].answering.ask, &[])) {
            Forward::Decide(m) => Some(s.node(world, pos, m)),
            _ => None,
        });
        match root {
            Some(r) => s.forks.worlds[world].root = r,
            None => s.forks.worlds[world].dead = true,
        }
        s.weights.push(weight);
        s.own.push(own);
        ranges.push(hand);
    }
    let total: f32 = s.weights.iter().zip(&s.forks.worlds).filter(|(_, w)| !w.dead).map(|(x, _)| *x).sum();
    if total > 0.0 {
        for x in &mut s.weights {
            *x /= total;
        }
    }
    let alive: Vec<usize> = (0..s.forks.worlds.len()).filter(|&w| !s.forks.worlds[w].dead).collect();
    let real_root = alive.iter().copied().find(|&w| s.own[w]).map(|w| s.forks.worlds[w].root);
    let Some(real_root) = real_root else {
        return network().map(|d| (d, Searched::default()));
    };
    // The roots first, then the batches.
    let mut pending: Vec<usize> = alive.iter().map(|&w| s.forks.worlds[w].root).collect();
    let mut next = 0usize;
    let mut t = 0usize;
    loop {
        if !pending.is_empty() {
            // One evaluation a key: nodes of different worlds the acting seat cannot tell apart are one input.
            let mut asked: Vec<usize> = Vec::new();
            for &n in &pending {
                if !s.cache.contains_key(&s.nodes[n].key) && !asked.iter().any(|&a| s.nodes[a].key == s.nodes[n].key) {
                    asked.push(n);
                }
            }
            if !asked.is_empty() {
                let encoded: Vec<encode::Encoded> = asked.iter().map(|&n| s.encoded(n)).collect();
                let out = evaluator.evaluate_positions(&encoded.iter().collect::<Vec<_>>())?;
                s.stats.evaluations += asked.len();
                for (k, &n) in asked.iter().enumerate() {
                    s.cache.insert(s.nodes[n].key, (out.value[k].clamp(-1.0, 1.0), out.policy[k].clone()));
                }
            }
            for &n in &pending {
                let (value, logits) = s.cache[&s.nodes[n].key].clone();
                s.expand(n, value, &logits);
            }
            for _ in 0..config.iterations.max(1) {
                t += 1;
                s.iterate(t);
            }
        }
        if s.stats.evaluations >= config.evaluations || alive.is_empty() {
            break;
        }
        pending.clear();
        let mut tries = 0;
        let batch = config.batch.max(config.evaluations / ROUNDS).max(1);
        while pending.len() < batch && tries < 4 * batch && s.stats.evaluations + pending.len() < config.evaluations {
            tries += 1;
            let world = alive[next % alive.len()];
            next += 1;
            if let Some(n) = s.simulate(world)
                && !pending.contains(&n)
            {
                pending.push(n);
            }
        }
        if pending.is_empty() {
            break;
        }
    }

    // The answer: the real information set's average strategy, walked through the request's sub-steps.
    let root_set = &s.sets[&s.nodes[real_root].key];
    let average = root_set.average();
    s.stats.root = root_set.tokens.iter().copied().zip(average.iter().copied()).collect();
    // The seat's own root value: over the worlds that keep its real hand (one weight among them).
    let own: Vec<usize> = alive.iter().copied().filter(|&w| s.own[w]).collect();
    s.stats.root_value = own.iter().map(|&w| s.value(s.forks.worlds[w].root)).sum::<f32>() / own.len().max(1) as f32;
    s.stats.net_value = s.nodes[real_root].value.unwrap_or(0.0);
    s.stats.range = alive.iter().map(|&w| (s.weights[w], s.own[w], s.value(s.forks.worlds[w].root), ranges[w].clone())).collect();
    s.stats.worlds = alive.len();
    s.stats.own_worlds = alive.iter().filter(|&&w| s.own[w]).count();
    s.stats.nodes = s.nodes.len();
    s.stats.infosets = s.sets.len();
    let turn_of = |n: &Node| s.forks.worlds[n.world].positions[n.pos].turn;
    s.stats.deepest_turns = s.nodes.iter().map(|n| turn_of(n).saturating_sub(root_turn)).max().unwrap_or(0);
    let mut n = real_root;
    loop {
        let node = &s.nodes[n];
        let Some(set) = s.sets.get(&node.key).filter(|_| node.value.is_some() && !node.boundary && !node.children.is_empty()) else {
            break;
        };
        let avg = set.average();
        let k = (0..avg.len()).max_by(|&a, &b| avg[a].total_cmp(&avg[b])).unwrap_or(0);
        match node.children[k] {
            Child::Node(c) => {
                if s.nodes[c].pos != node.pos {
                    s.stats.line_decisions += 1;
                }
                n = c;
            }
            _ => break,
        }
    }
    s.stats.line_turns = turn_of(&s.nodes[n]).saturating_sub(root_turn);
    s.stats.refused = s.forks.refused;
    let mut random = match config.choice {
        Choice::Sample { seed, .. } => seed | 1,
        Choice::Best => 1,
    };
    let pick = |p: &[f32], random: &mut u64| -> usize {
        match config.choice {
            Choice::Best => (0..p.len()).max_by(|&a, &b| p[a].total_cmp(&p[b]).then(b.cmp(&a))).unwrap_or(0),
            Choice::Sample { .. } => {
                *random = splitmix(*random);
                let mut x = (*random >> 40) as f32 / (1u64 << 24) as f32 * p.iter().sum::<f32>();
                for (k, &w) in p.iter().enumerate() {
                    if x < w {
                        return k;
                    }
                    x -= w;
                }
                p.len() - 1
            }
        }
    };
    let mut m = Machine::new(&answering.ask, &[]);
    // The node of the real root's world the walk is at, while the tree follows it.
    let mut at = Some(real_root);
    let mut evaluations = s.stats.evaluations;
    while let Some(step) = m.step() {
        let legal: Vec<usize> = step.legal().collect();
        if legal.is_empty() {
            break;
        }
        let i = if legal.len() == 1 {
            legal[0]
        } else if let Some(n) = at.filter(|&n| s.nodes[n].value.is_some() && !s.nodes[n].children.is_empty() && s.nodes[n].machine.chosen() == m.chosen()) {
            let set = &s.sets[&s.nodes[n].key];
            let k = pick(&set.average(), &mut random);
            set.options[k]
        } else {
            let e = encode::encode(&obs, step, m.chosen(), &m.names(), table);
            let out = evaluator.evaluate(&FeatureBatch::new(&[&e]))?;
            evaluations += 1;
            net::pick(&out.policy[0], &legal, config.choice, &mut random)
        };
        // Follow the tree to the next sub-step of this request, where it went.
        at = at.and_then(|n| {
            let set = s.sets.get(&s.nodes[n].key)?;
            let k = set.options.iter().position(|&o| o == i)?;
            match s.nodes[n].children.get(k)? {
                Child::Node(c) if s.nodes[*c].pos == s.nodes[n].pos => Some(*c),
                _ => None,
            }
        });
        m.choose(i);
    }
    s.stats.elapsed = started.elapsed();
    let factored: FactoredAnswer = m.answer().cloned().ok_or_else(|| EvalError("the machine finished with no answer".into()))?;
    let stats = s.stats.clone();
    drop(s);
    match answering.engine(game, request, &factored) {
        Ok(answer) => Ok((
            Decision {
                answer,
                factored,
                value: stats.root_value,
                evaluations,
                rejected: Vec::new(),
                fallback: false,
                late: false,
                steps: 0,
                encode: Duration::ZERO,
                evaluate: Duration::ZERO,
                search: None,
            },
            stats,
        )),
        Err(r) => {
            let mut d = network()?;
            d.rejected.insert(0, r);
            Ok((d, stats))
        }
    }
}
