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
//! or 0 for a draw). A line capped at `Config::turn_cap` turns past the root, or by too many requests with no choice
//! (`MAX_FORCED`), takes the network's value at the cap (decision 8, `Config::cap_value`, on by default): the request
//! there is observed by its seat and evaluated once, a node with no edges that is then a terminal with that value; a
//! cap that cannot be observed, and a line stuck on a request the engine refuses, is 0. A priority with no option but
//! passing (and not deferred) is passed in a line without being observed (`Config::skip_forced_observe`, on): its one
//! answer is `Pass` with or without the observation, and the seat's knowledge is folded there all the same, so the
//! lines are the same lines.
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
//!
//! # The root (the review of 2026-11-13, item 4; every option off by default)
//!
//! - `Config::shared_root`: every fresh world's root is the same observation, so the roots are evaluated before the
//!   simulations, one evaluation for every root whose network input is the same (`same_input`), counted as one
//!   simulation; each world expands its root from it (its own payment split and prior adjustments).
//! - `Config::root_bandit` (implies `shared_root`): the root is one bandit over the worlds. Each root option - its
//!   tokens and its class of payment (`Slots`) - is one arm, its `n`, `w` and virtual loss summed over the live worlds'
//!   roots (`Agg`); selection at a world's root scores its edges with those sums, and the simulation descends into that
//!   world, the worlds taking turns as before. Branch trimming at the root judges and cuts an option in every world at
//!   once on its sums. Below the root nothing changes (single-observer ISMCTS at the root alone: Cowling, Powley and
//!   Whitehouse 2012).
//! - `Config::gumbel` (implies `root_bandit`): Gumbel root selection with sequential halving (`Gumbel`, after `mctx`).
//!   Each simulation's root option is the one sequential halving asks for - its visits in this search equal to the
//!   halving table's (`halving_sequence`), best by `g + logit + sigma(completed q)` - and its `j`th visit goes to the
//!   `j`th world it can be taken in, so the options are compared in the same worlds. The answer at the root's first
//!   step is the survivor (the most visited in this search, best by the same score), whatever `Config::choice` says:
//!   in self-play the Gumbel draw (`Gumbel::noise`) is the exploration; in play it is off. The root is never trimmed.
//!   Visits carried by a kept tree count in the values, not in the halving.
//!
//! # Throughput (item 7)
//!
//! - `Config::eval_cache`: each row's network input (`input_key`) is looked up before it is sent, and a row whose input
//!   was evaluated before in this search, or is earlier in the same batch, takes those outputs (at most `eval_cache`
//!   rows kept; once full, nothing more is kept). Exact for an evaluator whose outputs are a function of the row's input
//!   alone; on one whose sums move with the batch's shape (tract on the CPU: about 1e-7) a row takes its first batch's
//!   bits. `Config::count_duplicates` counts the repeated inputs without caching (`Searched::duplicate_rows`).
//! - `Config::lazy_inner`: a leaf that is a priority searched as whole answers (`Flat`) is evaluated at its own step
//!   alone; each whole answer has its option's first-step prior shared evenly among the option's whole answers until
//!   the node is selected again, when its inner steps are evaluated (that selection is no simulation: `refine`) and
//!   each whole answer takes the product, as an eager expansion gives it. A world's root is always evaluated whole.
//!
//! # Budget and stopping (item 5)
//!
//! - `Config::snapshots`: the root's visits by option at each count of this search's simulations asked, and at the end
//!   (`Searched::snapshots`), for self-play to measure how often an earlier snapshot names the final choice.
//! - `Config::early_stop`: past that many simulations, a search whose root has one option visited (but for fewer
//!   visits than a batch: virtual loss's detours), no option cut and every other visited option at least
//!   `Config::early_stop_margin` below it stops (`Searched::early_stopped`); its root value is the visits' mean.
//! - `Config::time_use` (`TimeUse`): with a deadline, past a share of its time the search stops on the same settled
//!   root, or on a root whose visits' entropy and value swing (from the network's root value) are small.
//!
//! # Exploration (item 9; off by default)
//!
//! - `Config::root_floor` and `Config::root_noise`: the root's priors with a floor `(1 - e) P + e / options`, and with
//!   Dirichlet noise drawn by each option's tokens from the seed (the same in every world). `RootOption::net_prior` keeps
//!   the network's own. `Config::root_favour` floors one nominated option alone (`cantrip-explore verify`).
//!   `Config::favour_paths` floors whole answers, at every sub-step of the root's decision they pass through, so that
//!   two answers that differ only at a later sub-step are both searched there (`cantrip-explore judge`); with
//!   `Config::report` each world's visits and values through each are reported along the way (`WorldReport::favoured`).
//! - `Config::forced_playouts` (KataGo): at the root an option with fewer than `sqrt(k * P * N)` visits is taken first,
//!   and the recorded target at the root's first step takes back, from every option but the most visited, the forced
//!   visits its PUCT score with fewer visits would still not justify (on the option's sums over the worlds); an option
//!   left with one is taken out (`Target::Pruned`).
//! - `Config::root_shortcut` off: the root never takes the priority shortcut (`Searched::shortcut_would` says whether
//!   it would have). `audit_shortcut` runs a full search so, with a floor, at a root the shortcut would pass.
//!
//! # What the search reports for recording
//!
//! - `Searched::visits` (`RootStep`): each sub-step of the answer the search chose, with the root's visits there over
//!   the worlds. At the root's first step, with Gumbel, they are Gumbel's policy target `softmax(logit + sigma(completed
//!   q))` times `TARGET_SCALE`; with forced playouts, the pruned visits (`Searched::target` says which). A recorder that
//!   normalises them per step needs no change to carry either.
//! - `Searched::steps` (`StepOrigin`): every sub-step of the answer with a choice, and whether the search chose it or
//!   the network did (a sub-step the search never reached): the per-sub-step "came from a search" flag.
//! - `Searched::root_options` (`RootOption`): each root option once, over the worlds, with its prior (and the
//!   network's), visits, mean value `q`, completed value (Gumbel's completion, for a Q head) and target share.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use cantrip_engine::*;
use cantrip_obs::encode;
use cantrip_obs::factor::{FactoredAnswer, Machine, Tok, Token};
use cantrip_obs::{InfoMode, Observation};

use crate::cancel::{self, CancelGuard};
use crate::pay;
use crate::eval::{EvalError, Evaluator, FeatureBatch, MASKED, Outputs};
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
    /// The root evaluated once (see the module, "The root"): the worlds' fresh roots are one observation, so one
    /// evaluation is copied to every world whose root has the same network input, and counts as one simulation. Off
    /// by default (each world's root is a simulation of its own, as before).
    pub shared_root: bool,
    /// One bandit at the root over the worlds (see the module, "The root"): selection at a world's root reads every
    /// live world's visits and values of the same option (its tokens and its payment class), each simulation still
    /// descending into the next world; below the root nothing changes. Implies `shared_root`. Off by default.
    pub root_bandit: bool,
    /// Gumbel root selection with sequential halving and completed-Q policy targets (`Gumbel`; see the module, "The
    /// root"). Implies `root_bandit`. None by default: PUCT at the root, the visits the target.
    pub gumbel: Option<Gumbel>,
    /// Evaluations cached by the network's input (see the module, "Throughput"): at most this many rows kept; 0, no
    /// cache (the default). Exact: the same input is never sent twice, and a row's outputs are the network's for it.
    pub eval_cache: usize,
    /// Count the rows whose network input the search already sent (`Searched::duplicate_rows`), without caching: what
    /// the cache would save. Off by default (it hashes every row).
    pub count_duplicates: bool,
    /// A flattened priority's inner steps (`Flat`) evaluated only when the node is selected again (see the module,
    /// "Throughput"); until then each whole answer has its option's first-step prior shared evenly among the option's
    /// whole answers. A world's root is always evaluated whole. Off by default.
    pub lazy_inner: bool,
    /// Budget snapshots: at each of these counts of this search's simulations (checked between batches), the root's
    /// visits by option are kept (`Searched::snapshots`), and a last one at the end. Empty by default. Changes no answer.
    pub snapshots: Vec<usize>,
    /// An early stop: after this many of the search's simulations, a search whose root has one option visited (but
    /// for fewer visits than a batch: the detours virtual loss makes), no option cut and no close value
    /// (`early_stop_margin`) stops between batches (`Searched::early_stopped`); its root value is still the visits'
    /// mean. 0, never (the default).
    pub early_stop: usize,
    /// How far below the visited option's value any other visited option's must be for the early stop (and the time
    /// use's stop on one option).
    pub early_stop_margin: f32,
    /// A floor under the root's priors: `P' = (1 - e) P + e / options`. 0, none (the default).
    pub root_floor: f32,
    /// A floor on one option of the root's first sub-step with a choice, `(token, e)`: the options through that token
    /// (a whole answer's first token, `Flat`, or the step's option) get `P' = (1 - e) P + e P / P(token)` (the floor
    /// shared among them by their priors, evenly where those are 0), every other `(1 - e) P`; after `root_floor`, before
    /// the noise. For a tool weighing one nominated play (`cantrip-explore verify`); not a setting by name. None, the
    /// default.
    pub root_favour: Option<(Token, f32)>,
    /// Floors on whole answers of the root's request, `(tokens, e)`: each a path of the request's machine from its start
    /// (`cantrip_obs::factor::decompose`'s tokens, the forced ones included). At every node of the root's decision whose
    /// chosen tokens begin the path - the root, and the sub-steps below it in the same request - the edges through the
    /// path get `P' = (1 - e) P + e P / P(through)` (shared by their priors, evenly where those are 0) and every other
    /// `(1 - e) P`, as `root_favour` does at the root alone; each path in turn, after `root_favour`. With `report`, each
    /// world's visits and values through each path, node by node (`WorldReport::favoured`). For a tool comparing given
    /// answers (`cantrip-explore judge`); not a setting by name. Empty, the default.
    pub favour_paths: Vec<(Vec<Token>, f32)>,
    /// Dirichlet noise in the root's priors: `P' = (1 - e) P + e * Dir(root_noise_alpha)`, drawn from `seed` and each
    /// option's tokens (the same in every world). 0, none (the default).
    pub root_noise: f32,
    pub root_noise_alpha: f32,
    /// KataGo's forced playouts at the root, with policy target pruning: an option with fewer than
    /// `sqrt(k * P * N)` visits is taken first, and the recorded target takes back the forced visits the values do not
    /// justify (see the module, "Exploration"). This is `k`; 0, none (the default).
    pub forced_playouts: f32,
    /// Whether the priority shortcut may answer at the root. Off, a root the shortcut would pass is searched with every
    /// option (`Searched::shortcut_would` says so): with `root_floor`, the shortcut audit (`audit_shortcut`). On by
    /// default. Off, a kept tree is not re-rooted (its root may have taken the shortcut).
    pub root_shortcut: bool,
    /// Whether a line capped at `turn_cap` or `MAX_FORCED` takes the network's value at the cap (decision 8), or is a
    /// draw. On by default.
    pub cap_value: bool,
    /// Whether a line observes a request it answers with no choice where the answer needs no observation (a priority
    /// with no option but passing, the request not deferred: passing is its one answer). Off, every request a line
    /// answers is observed, as before; on (the default) the same lines are played, the same answers given, without
    /// the observation (`Searched::forced_unobserved`).
    pub skip_forced_observe: bool,
    /// Time use at play (`TimeUse`): with a deadline, stop early on a settled root. None by default.
    pub time_use: Option<TimeUse>,
}

/// Gumbel root selection (Danihelka, Guez, Schrittwieser and Silver 2022, "Policy improvement by planning with Gumbel",
/// as DeepMind's `mctx` implements it): the top `considered` root options by `g + logit` are visited by sequential
/// halving, each phase keeping the better half by `g + logit + sigma(completed q)`; the answer is the survivor; the
/// policy target is `softmax(logit + sigma(completed q))` over every root option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gumbel {
    /// Root options considered at most (`m`).
    pub considered: usize,
    /// Whether `g` is drawn (a Gumbel(0, 1) per option, from the search's seed): the exploration, for self-play. Off,
    /// `g` is 0 (play).
    pub noise: bool,
    /// `sigma(q) = (c_visit + the most visits of an option) * c_scale * q`, `q` the completed values rescaled to [0, 1]
    /// over the options (`mctx`'s `qtransform_completed_by_mix_value`).
    pub c_visit: f32,
    pub c_scale: f32,
    /// The simulations sequential halving is planned over when the search has a deadline and no count.
    pub budget: usize,
}

impl Default for Gumbel {
    fn default() -> Gumbel {
        Gumbel { considered: 16, noise: false, c_visit: 50.0, c_scale: 0.1, budget: 1024 }
    }
}

/// Time use at play (`Config::time_use`): once `min_share` of the time to the deadline has passed, the search stops
/// between batches when its root has one option visited (as `Config::early_stop` judges it), or when the visits over
/// the root's options have an entropy under `entropy` (nats) and the root's value is within `swing` of the network's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeUse {
    pub min_share: f32,
    pub entropy: f32,
    pub swing: f32,
}

impl Default for TimeUse {
    fn default() -> TimeUse {
        TimeUse { min_share: 0.25, entropy: 0.2, swing: 0.05 }
    }
}

/// What a Gumbel search's (or a forced-playout search's) recorded root visits are scaled to: a root step's
/// `RootStep::visits` are then its policy target times this, rounded (so a share under `1 / TARGET_SCALE` is 0).
pub const TARGET_SCALE: f32 = 1_000_000.0;

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
            shared_root: false,
            root_bandit: false,
            gumbel: None,
            eval_cache: 0,
            count_duplicates: false,
            lazy_inner: false,
            snapshots: Vec::new(),
            early_stop: 0,
            early_stop_margin: 0.1,
            root_floor: 0.0,
            root_favour: None,
            favour_paths: Vec::new(),
            root_noise: 0.0,
            root_noise_alpha: 0.3,
            forced_playouts: 0.0,
            root_shortcut: true,
            cap_value: true,
            skip_forced_observe: true,
            time_use: None,
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

    /// Sets one search setting by its name, as the AI host's `AiStart` names it (BotLab's per-seat `+Key=Value`) and
    /// `cantrip-selfplay`'s `--search Key=Value`: `Worlds`, `CPuct`, `PriorTemperature`, `Batch`, `Shortcut`,
    /// `TurnCap`, `Prune`, `PruneBad`, `PruneMargin`, `PruneAfter`, `PruneAfterReplies`, `PruneReversible`, `Atomic`,
    /// `Flatten`, `PayClassic`, `PaySplit`, `SharedRoot`, `RootBandit`, `Gumbel`, `GumbelConsidered`, `GumbelNoise`,
    /// `GumbelCVisit`, `GumbelCScale`, `GumbelBudget` (a `Gumbel*` setting turns Gumbel on), `EvalCache`,
    /// `CountDuplicates`, `LazyInner`, `Snapshots` (counts joined by `,` or `;`), `EarlyStop`, `EarlyStopMargin`,
    /// `RootFloor`, `RootNoise`, `RootNoiseAlpha`, `ForcedPlayouts`, `RootShortcut`, `CapValue`, `SkipForcedObserve`,
    /// `TimeUse`, `TimeMinShare`, `TimeEntropy`, `TimeSwing` (a `Time*` setting turns time use on). A count may be
    /// written as a whole number with a fraction of 0 (`8.0`), as a JSON number from a double reads.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let value = value.trim();
        let bad = || format!("search setting {key}: no value {value:?}");
        let float = || value.parse::<f32>().map_err(|_| bad()).and_then(|x| if x.is_finite() { Ok(x) } else { Err(bad()) });
        let count = || -> Result<usize, String> {
            match value.parse::<usize>() {
                Ok(n) => Ok(n),
                Err(_) => match value.parse::<f64>() {
                    Ok(x) if x >= 0.0 && x.fract() == 0.0 && x <= u32::MAX as f64 => Ok(x as usize),
                    _ => Err(bad()),
                },
            }
        };
        let flag = || match value.to_ascii_lowercase().as_str() {
            "true" | "1" | "on" | "yes" => Ok(true),
            "false" | "0" | "off" | "no" => Ok(false),
            _ => Err(bad()),
        };
        match key {
            "Worlds" => self.worlds = count()?.max(1),
            "CPuct" => self.c_puct = float()?,
            "PriorTemperature" => self.prior_temperature = float()?,
            "Batch" => self.batch = count()?.max(1),
            "Shortcut" => self.shortcut = float()?,
            "TurnCap" => self.turn_cap = count()? as u32,
            "Prune" => self.prune = flag()?,
            "PruneBad" => self.prune_bad = float()?,
            "PruneMargin" => self.prune_margin = float()?,
            "PruneAfter" => self.prune_after = count()? as u32,
            "PruneAfterReplies" => self.prune_after_replies = count()? as u32,
            "PruneReversible" => self.prune_reversible = flag()?,
            "Atomic" => self.atomic = flag()?,
            "Flatten" => self.flatten = count()?,
            "PayClassic" => self.pay_classic = flag()?,
            "PaySplit" => self.pay_split = float()?,
            "SharedRoot" => self.shared_root = flag()?,
            "RootBandit" => self.root_bandit = flag()?,
            "Gumbel" => self.gumbel = flag()?.then(|| self.gumbel.unwrap_or_default()),
            "GumbelConsidered" => self.gumbel.get_or_insert_with(Gumbel::default).considered = count()?.max(1),
            "GumbelNoise" => self.gumbel.get_or_insert_with(Gumbel::default).noise = flag()?,
            "GumbelCVisit" => self.gumbel.get_or_insert_with(Gumbel::default).c_visit = float()?,
            "GumbelCScale" => self.gumbel.get_or_insert_with(Gumbel::default).c_scale = float()?,
            "GumbelBudget" => self.gumbel.get_or_insert_with(Gumbel::default).budget = count()?.max(1),
            "EvalCache" => self.eval_cache = count()?,
            "CountDuplicates" => self.count_duplicates = flag()?,
            "LazyInner" => self.lazy_inner = flag()?,
            "Snapshots" => {
                self.snapshots = value
                    .split([',', ';'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.parse::<usize>().map_err(|_| bad()))
                    .collect::<Result<_, _>>()?;
            }
            "EarlyStop" => self.early_stop = count()?,
            "EarlyStopMargin" => self.early_stop_margin = float()?,
            "RootFloor" => self.root_floor = float()?.clamp(0.0, 1.0),
            "RootNoise" => self.root_noise = float()?.clamp(0.0, 1.0),
            "RootNoiseAlpha" => self.root_noise_alpha = float()?.max(1e-3),
            "ForcedPlayouts" => self.forced_playouts = float()?.max(0.0),
            "RootShortcut" => self.root_shortcut = flag()?,
            "CapValue" => self.cap_value = flag()?,
            "SkipForcedObserve" => self.skip_forced_observe = flag()?,
            "TimeUse" => self.time_use = flag()?.then(|| self.time_use.unwrap_or_default()),
            "TimeMinShare" => self.time_use.get_or_insert_with(TimeUse::default).min_share = float()?,
            "TimeEntropy" => self.time_use.get_or_insert_with(TimeUse::default).entropy = float()?,
            "TimeSwing" => self.time_use.get_or_insert_with(TimeUse::default).swing = float()?,
            _ => return Err(format!("no search setting {key}")),
        }
        Ok(())
    }

    /// `set` for a `Key=Value` pair.
    pub fn set_pair(&mut self, pair: &str) -> Result<(), String> {
        let (key, value) = pair.split_once('=').ok_or_else(|| format!("search setting {pair:?}: not Key=Value"))?;
        self.set(key.trim(), value)
    }

    /// Whether the roots are evaluated once, before the simulations (`shared_root`, or what implies it).
    fn shares_root(&self) -> bool {
        self.shared_root || self.root_bandit || self.gumbel.is_some()
    }

    /// Whether selection at a root reads every world's statistics (`root_bandit`, or Gumbel).
    fn bandit(&self) -> bool {
        self.root_bandit || self.gumbel.is_some()
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
    /// Lines the kept tree had capped at its old root's horizon (`Config::turn_cap`) that the new root's reaches: each
    /// is played again when it is next taken, to the new horizon.
    pub uncapped: usize,
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
    /// The tokens chosen before the root's first sub-step with a choice (steps with one legal option): what
    /// `root_options`' paths follow.
    pub root_prefix: Vec<Token>,
    /// The root's first sub-step's options as searched, over the live worlds (`RootOption`): each whole answer (`Flat`)
    /// or step option, and each class of payment of a split cast, once, with its prior, visits and values. For a Q
    /// head and for recording. Empty when no root was expanded.
    pub root_options: Vec<RootOption>,
    /// What the recorded root visits (`visits`) are at the root's first sub-step (`Target`).
    pub target: Target,
    /// Every sub-step of the answer with more than one legal option, in order, and whether the search chose it
    /// (`StepOrigin::searched`: the search reached it and the option is the visits', or Gumbel's) or the network did (a
    /// sub-step the search never reached). Empty when the whole answer is the network's (no visits).
    pub steps: Vec<StepOrigin>,
    /// The root's visits at `Config::snapshots`' counts, and at the end (`Snapshot`).
    pub snapshots: Vec<Snapshot>,
    /// Whether `Config::early_stop` stopped the search, and whether `Config::time_use` did.
    pub early_stopped: bool,
    pub time_stopped: bool,
    /// The evaluation cache (`Config::eval_cache`): rows taken from it (or from a row with the same input in the same
    /// batch) instead of being sent.
    pub cache_hits: usize,
    /// Rows whose network input the search had already asked for (`Config::count_duplicates` or `eval_cache`), and the
    /// distinct inputs asked: the cache's worth, measured.
    pub duplicate_rows: usize,
    pub distinct_rows: usize,
    /// Flattened priorities whose inner steps were evaluated when selected again (`Config::lazy_inner`), and the rows
    /// the leaves' inner steps would have cost evaluated at once but were not (left unevaluated at the end).
    pub lazy_refined: usize,
    pub lazy_pending_rows: usize,
    /// Requests the lines answered with no choice without observing them (`Config::skip_forced_observe`).
    pub forced_unobserved: usize,
    /// Lines capped and valued by the network at the cap (`Config::cap_value`).
    pub capped_valued: usize,
    /// Whether the priority shortcut's rule held at the root on the network's own priors (whether or not it answered:
    /// `Config::root_shortcut`).
    pub shortcut_would: bool,
    /// The answer's searched sub-steps (`visits`) at which two legal options have an equal token (interchangeable
    /// options: they make the same answer). Each keeps its own visits (`RootStep::options`).
    pub equal_tokens: usize,
}

/// One option at the root's first sub-step, over the live worlds (`Searched::root_options`).
#[derive(Clone, Debug, PartialEq)]
pub struct RootOption {
    /// Its tokens from the root's first sub-step: a whole answer's (`Flat`), or the step's option alone.
    pub path: Vec<Token>,
    /// The sources its cast leaves untapped, where the cast was split by class of payment (`Config::pay_split`).
    pub keep: Option<Vec<ObjectId>>,
    /// The prior the search used (the first live world's: every world's root is the same observation), and the
    /// network's own before any floor or noise (`Config::root_floor`, `root_noise`).
    pub prior: f32,
    pub net_prior: f32,
    /// Visits summed over the worlds, cut ones included, and the sum of the values backed up through it, for the seat.
    pub visits: u32,
    pub value_sum: f32,
    /// `value_sum / visits`, for the seat: none if unvisited.
    pub q: Option<f32>,
    /// `q`, or for an unvisited option the root's mixed value (Gumbel's completion: the network's root value and the
    /// visited options' values weighted by their priors).
    pub completed_q: f32,
    /// Its share of the recorded target where the target is not the visits (`Target`).
    pub target: Option<f32>,
    /// Cut by branch trimming in some world; refused by the engine in every world.
    pub pruned: bool,
    pub dead: bool,
}

/// What a search records at its root's first sub-step (`Searched::target`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    /// The visits.
    #[default]
    Visits,
    /// The Gumbel policy target, `softmax(logit + sigma(completed q))`, scaled by `TARGET_SCALE` (`Config::gumbel`).
    Gumbel,
    /// The visits with KataGo's policy target pruning: forced visits the values do not justify taken back
    /// (`Config::forced_playouts`).
    Pruned,
}

/// A sub-step of the answer and who chose it (`Searched::steps`).
#[derive(Clone, Debug, PartialEq)]
pub struct StepOrigin {
    pub prefix: Vec<Token>,
    pub chosen: Token,
    /// The search chose it (the step has a `RootStep`); false, the network (the search never reached the step).
    pub searched: bool,
}

/// The root's visits at a count of the search's simulations (`Config::snapshots`).
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// This search's simulations when it was taken (at the first batch boundary at or past the count asked).
    pub simulations: usize,
    /// Each root option's visits over the worlds, cut ones left out, by its tokens from the root's first sub-step (the
    /// classes of payment of a split cast summed), in the root's order.
    pub options: Vec<(Vec<Token>, u32)>,
    /// The most visited (the first of a tie), none with no visits.
    pub best: Option<Vec<Token>>,
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
    /// Beside each of `Config::favour_paths`, the nodes of the root's decision along it from the world's root: at each,
    /// the tokens chosen before it, and the visits and summed value (for the seat) of its edges through the path. The walk
    /// follows the one edge through the path while it leads to a node of the same request, and stops where none or
    /// several do (a whole answer's edge, a cast split by class of payment) or the node is unexpanded.
    pub favoured: Vec<Vec<FavouredAt>>,
}

/// One node of the root's decision along a favoured path (`WorldReport::favoured`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FavouredAt {
    /// The tokens chosen before the node (the request's machine's).
    pub chosen: Vec<Token>,
    pub visits: u32,
    pub value_sum: f32,
    /// The edges of the node through the path.
    pub edges: usize,
}

/// Whether an edge whose tokens from its node are `edge` runs through `rest`, the path's tokens from the same node: one
/// begins the other.
fn through_path(edge: &[Token], rest: &[Token]) -> bool {
    let n = edge.len().min(rest.len());
    n > 0 && edge[..n] == rest[..n]
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

/// One root sub-step: the tokens chosen before it, its legal options (each one's token and its index among the step's
/// options), their visits and root values, the one taken. Visits and values are kept by option index, not by token: two
/// options with an equal token (interchangeable, `Searched::equal_tokens`) are counted apart. Equal when every field is,
/// the values by their bits (a NaN, no value, equals itself).
#[derive(Clone, Debug)]
pub struct RootStep {
    pub prefix: Vec<Token>,
    pub tokens: Vec<Token>,
    /// Beside each of `tokens`, its option's index in the step.
    pub options: Vec<usize>,
    pub visits: Vec<u32>,
    /// Beside each of `tokens`, its option's mean root value for the seat over every world's visits through it, cut
    /// edges included (`RootOption`'s values summed over the root's options that run through it): NaN where none was
    /// visited, or where the sub-step lies past what the root's options spell (a later sub-step searched in steps).
    pub q: Vec<f32>,
    pub chosen: Token,
}

impl PartialEq for RootStep {
    fn eq(&self, other: &RootStep) -> bool {
        self.prefix == other.prefix
            && self.tokens == other.tokens
            && self.options == other.options
            && self.visits == other.visits
            && self.chosen == other.chosen
            && self.q.len() == other.q.len()
            && self.q.iter().zip(&other.q).all(|(a, b)| a.to_bits() == b.to_bits())
    }
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
    /// Whether a capped line is observed at the cap for the network's value (`Reached::CappedAt`, `Config::cap_value`).
    pub(crate) cap_value: bool,
    /// Whether a request with no option but passing is passed unobserved (`Config::skip_forced_observe`), and how many
    /// were.
    pub(crate) skip_forced_observe: bool,
    pub(crate) forced_unobserved: usize,
}

/// Where playing an answer in a world led.
pub(crate) enum Reached {
    /// The line reached the turn cap or `MAX_FORCED` (as `Capped`), observed at the request there for the network's
    /// value (`Forks::cap_value`): the position, and the factoring at its first step.
    CappedAt(usize, Machine),
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
    /// A line capped here (`Reached::CappedAt`): evaluated once for its value, then a terminal with it.
    capped: bool,
    /// The shortcut's rule held here on the network's priors (whether or not it was taken: `Config::root_shortcut`).
    shortcut_would: bool,
    /// The root's prior adjustments (floor, noise) are made (`Search::adjust_root`).
    rooted: bool,
    /// A flattened priority's inner steps not evaluated yet (`Config::lazy_inner`).
    lazy: Option<Box<Lazy>>,
}

/// A flattened priority expanded before its inner steps were evaluated (`Config::lazy_inner`): its inner steps, the
/// first step's priors, and each whole answer's inner choices by its tokens.
struct Lazy {
    inner: Vec<Machine>,
    first: BTreeMap<usize, f32>,
    steps: BTreeMap<Vec<Token>, Vec<(usize, usize)>>,
}

/// The root's options over the worlds (`Config::root_bandit`): each option (its tokens and its class of payment) once,
/// with the root edges that are it in every world.
#[derive(Default)]
struct Slots {
    list: Vec<Slot>,
    /// Each root node's edges' slots.
    of: std::collections::HashMap<usize, Vec<usize>>,
}

struct Slot {
    path: Vec<Token>,
    keep: Option<std::sync::Arc<Vec<ObjectId>>>,
    /// Its options' indices, step by step (`Edge::index`, then `Edge::rest`), as its first world's root has them.
    index: Vec<usize>,
    /// Its place among a root's options with the same tokens and class of payment (0 but for interchangeable options,
    /// whose tokens are equal): each is its own slot.
    dup: usize,
    /// (node, edge), in the worlds' order.
    members: Vec<(usize, usize)>,
}

/// A root option's statistics summed over the live worlds.
#[derive(Clone, Copy, Debug, Default)]
struct Agg {
    n: u32,
    w: f32,
    vl: u32,
    replies: u32,
    prior: f32,
    /// The network's own prior, before the root's floor and noise (`Search::raw_priors`): what Gumbel reads.
    raw: f32,
    /// Some world's edge is cut, and some world's edge can be taken (neither cut nor refused).
    pruned: bool,
    open: bool,
    /// Refused in every world.
    dead: bool,
}

/// A batch's entry: a leaf, or a node to refine (`true`), and the path to it.
type Pending = (usize, Vec<(usize, usize)>, bool);

/// A root option's tokens, payment class and (node, edge) in each world, as reported.
type SlotView = (Vec<Token>, Option<Vec<ObjectId>>, Vec<(usize, usize)>);

/// An edge's options' indices, step by step: its own (`Edge::index`), then the whole answer's after it (`Edge::rest`).
fn index_path(e: &Edge) -> Vec<usize> {
    std::iter::once(e.index).chain(e.rest.iter().copied()).collect()
}

/// A root option's statistic `v` added at each step of its path from `base` (`path` its tokens, `index` its options'
/// indices step by step): by the step's prefix of tokens and the option's index there.
fn add_by_index<T: Copy + Default + std::ops::AddAssign>(
    into: &mut BTreeMap<Vec<Token>, BTreeMap<usize, T>>,
    base: &[Token],
    path: &[Token],
    index: &[usize],
    v: T,
) {
    let mut prefix = base.to_vec();
    for (&t, &i) in path.iter().zip(index) {
        *into.entry(prefix.clone()).or_default().entry(i).or_default() += v;
        prefix.push(t);
    }
}

/// The root options over the worlds' roots (`roots`: each root node and its edges, in the worlds' order): each option -
/// its tokens, its class of payment, and its place among a root's options with both equal (`Slot::dup`) - once, in the
/// order first met, with the root edges that are it. By a map, not a scan: a root of hundreds of whole answers over
/// tens of worlds is otherwise quadratic in its edges.
fn slots_of<'e>(roots: impl Iterator<Item = (usize, &'e [Edge])>) -> Slots {
    type Key = (Vec<Token>, Option<Vec<ObjectId>>, usize);
    let mut slots = Slots::default();
    let mut at: std::collections::HashMap<Key, usize> = std::collections::HashMap::new();
    for (r, edges) in roots {
        let mut of = Vec::with_capacity(edges.len());
        let mut seen: std::collections::HashMap<(&[Token], Option<&[ObjectId]>), usize> = std::collections::HashMap::new();
        for (k, e) in edges.iter().enumerate() {
            let rank = seen.entry((&e.path[..], e.keep.as_deref().map(|v| &v[..]))).or_default();
            let dup = *rank;
            *rank += 1;
            let s = *at.entry((e.path.clone(), e.keep.as_deref().cloned(), dup)).or_insert_with(|| {
                slots.list.push(Slot { path: e.path.clone(), keep: e.keep.clone(), index: index_path(e), dup, members: Vec::new() });
                slots.list.len() - 1
            });
            slots.list[s].members.push((r, k));
            of.push(s);
        }
        slots.of.insert(r, of);
    }
    slots
}

enum Selected {
    Leaf(usize, Vec<(usize, usize)>),
    /// A node whose inner steps are to be evaluated now (`Config::lazy_inner`): no simulation; the path is unwound.
    Refine(usize, Vec<(usize, usize)>),
    /// A root edge Gumbel chose was refused by the engine in the world: nothing to back up.
    Refused,
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
    /// The root's options over the worlds, while selection at the root reads them (`Config::root_bandit`).
    slots: Option<Slots>,
    /// Gumbel's state (`Config::gumbel`): each slot's draw, its visits when the simulations began, the budget planned,
    /// the options considered (`m`, fixed as the simulations begin) and sequential halving's visit table for them.
    gumbel_g: Vec<f32>,
    gumbel_base: Vec<u32>,
    gumbel_n: usize,
    gumbel_m: usize,
    gumbel_table: Vec<u32>,
    cache: EvalCache,
    lazy_refined: usize,
    capped_valued: usize,
    /// The network's priors at each root whose priors were adjusted (`Search::adjust_root`).
    raw_priors: std::collections::HashMap<usize, Vec<f32>>,
}

/// The evaluation cache (`Config::eval_cache`) and the duplicate count (`Config::count_duplicates`), keyed by the
/// network's input (`input_key`).
#[derive(Default)]
struct EvalCache {
    rows: std::collections::HashMap<u128, (f32, Vec<f32>)>,
    max: usize,
    seen: std::collections::HashSet<u128>,
    hits: usize,
    duplicates: usize,
    buf: Vec<u8>,
}

/// A key for the network's input of one row: everything `FeatureBatch` takes from the position (its real tokens'
/// features, the legal mask over its bucket, its options' place) as bytes, under two SipHash-2-4 digests keyed apart
/// (128 bits: two inputs that differ share a key with a chance of about one in 2^128 per pair). The outputs of an
/// evaluator are a function of this input (`Evaluator`; the server's are bit-identical per input).
fn input_key(e: &encode::Encoded, buf: &mut Vec<u8>) -> u128 {
    use std::hash::Hasher;
    use encode::{CATS, NUMS, PTRS};
    buf.clear();
    for x in [e.len, e.bucket, e.first_option, e.options] {
        buf.extend_from_slice(&(x as u32).to_le_bytes());
    }
    let len = e.len.min(e.kind.len());
    buf.extend_from_slice(&e.kind[..len]);
    for c in &e.card[..len.min(e.card.len())] {
        buf.extend_from_slice(&c.to_le_bytes());
    }
    buf.extend_from_slice(&e.cats[..(len * CATS).min(e.cats.len())]);
    buf.extend_from_slice(&e.ptrs[..(len * PTRS).min(e.ptrs.len())]);
    for x in &e.nums[..(len * NUMS).min(e.nums.len())] {
        buf.extend_from_slice(&x.to_bits().to_le_bytes());
    }
    buf.extend(e.legal.iter().map(|&l| u8::from(l)));
    let digest = |salt: u8| {
        #[allow(deprecated)]
        let mut h = std::hash::SipHasher::new_with_keys(0x5ea2_c4ca_c4e0_0001 ^ u64::from(salt), 0x9e37_79b9_7f4a_7c15);
        h.write(buf);
        h.finish()
    };
    (u128::from(digest(1)) << 64) | u128::from(digest(2))
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

    /// The position at the request a capped line stopped at, observed by the seat it asks, and its factoring's first
    /// step (one legal option or more): what the network values the cap at (decision 8). None where the request
    /// cannot be observed or has no step, and the line is a draw as before.
    fn capped_at(
        &mut self,
        world: usize,
        mut knowledge: Vec<Knowledge>,
        runs: Vec<Option<(RunKey, FactoredAnswer)>>,
        mut guards: Vec<CancelGuard>,
    ) -> Option<Reached> {
        let observing = Instant::now();
        let w = &mut self.worlds[world];
        let request = w.game.pending()?.clone();
        let i = request.player.index();
        let request = guards[i].filtered(&w.game, &request);
        let request = if self.atomic { crate::atomic::filter(&w.game, &request) } else { request };
        knowledge[i].update(&w.game);
        let (obs, index) = observe_indexed(&w.game, &knowledge[i], request.player, self.mode)?;
        let answering = Answering::new(&w.game, &request, &obs, &index);
        let m = Machine::new(&answering.ask, &[]);
        m.step()?;
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
        let at = w.positions.len() - 1;
        self.t_observe += observing.elapsed();
        Some(Reached::CappedAt(at, m))
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
        // An answer that ends the payment ends the run: the next request of its key is a new ask.
        runs[actor] = crate::record::run_key(&request)
            .filter(|_| !crate::record::ends_run(&answer))
            .map(|k| (k, factored));
        for forced in 0usize.. {
            let w = &mut self.worlds[world];
            if w.game.is_over() {
                return Reached::Over(outcome_for(&w.game, seat));
            }
            if forced >= MAX_FORCED || w.game.state.turn.number > cap {
                if self.cap_value
                    && let Some(at) = self.capped_at(world, knowledge, runs, guards)
                {
                    return at;
                }
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
            // A priority with no option but passing, not deferred: its factoring is pass alone, its one answer `Pass`
            // (`Answering::engine`), and no payment run is keyed by a priority - the same answer as observed, without
            // the observation. The seat's knowledge is still folded here, as observing it would.
            if self.skip_forced_observe
                && matches!(&request.kind, RequestKind::Priority { options } if options.is_empty())
                && !cantrip_engine::visibility::deferred(&w.game, request.player)
            {
                knowledge[i].update(&w.game);
                let cancelled = guards[i].before(&w.game, &request, &Answer::Pass);
                if w.game.answer(request.id, request.player, Answer::Pass).is_err() {
                    self.refused += 1;
                    return Reached::Stuck;
                }
                guards[i].after(&w.game, cancelled);
                runs[i] = None;
                self.forced_unobserved += 1;
                continue;
            }
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
                    if crate::record::ends_run(&a) {
                        runs[i] = None;
                    }
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
                    runs[i] = run.filter(|_| !crate::record::ends_run(&a)).map(|k| (k, f));
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
            capped: false,
            shortcut_would: false,
            rooted: false,
            lazy: None,
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
                    // A capped line is the network's value at the cap (decision 8): a node evaluated once, then a
                    // terminal with that value for the seat asked there.
                    Reached::CappedAt(pos, m) => {
                        let c = self.node(world, pos, m);
                        self.nodes[c].capped = true;
                        self.nodes[c].waste = waste || self.forks.last_waste;
                        self.capped_valued += 1;
                        Child::Node(c)
                    }
                    // A capped line that could not be observed (or with `Config::cap_value` off), and a stuck line, is
                    // a draw to this search.
                    Reached::Capped | Reached::Stuck => Child::Terminal(0.0),
                    Reached::Dead => Child::Dead,
                    Reached::OutOfTime => return None,
                }
            }
        })
    }

    /// A priority's whole answers (`Flat`) with each cast the plan does not settle the payment of made two edges, one for
    /// each of its two best classes of payment (`Config::pay_split`, `pay::classes`), each with half its prior; the
    /// follow-ups are the node's other options that cost mana, each once, by its first step's prior (`first`). By the
    /// first step's priors, not the whole answers': a lazy expansion (`Config::lazy_inner`) knows those alone, and an
    /// eager one and a lazy one must split alike, so that refining a lazy node gives the edges an eager one has.
    fn split_by_payment(&mut self, at: usize, edges: Vec<Edge>, first: &BTreeMap<usize, f32>) -> Vec<Edge> {
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
        let costs: Vec<Option<(u32, ManaCost)>> = edges
            .iter()
            .map(|e| {
                (e.token.kind == Tok::Option)
                    .then(|| position.answering.priority_option(&position.request, e.token.n))
                    .flatten()
                    .and_then(|o| pay::option_cost(&w.game, o))
                    .map(|c| (e.token.n, c))
            })
            .collect();
        // Each option that costs mana once (its whole answers share its cost), with its first step's prior.
        let mut options_costing: Vec<(u32, ManaCost, f32)> = Vec::new();
        for (e, c) in edges.iter().zip(&costs) {
            if let Some((n, cost)) = c
                && !options_costing.iter().any(|(m, _, _)| m == n)
            {
                options_costing.push((*n, *cost, first.get(&e.index).copied().unwrap_or(0.0)));
            }
        }
        let mut out = Vec::with_capacity(edges.len());
        let mut split = 0;
        for (k, e) in edges.into_iter().enumerate() {
            let Some((n, cost)) = &costs[k] else {
                out.push(e);
                continue;
            };
            let follow: pay::FollowUps =
                options_costing.iter().filter(|(m, _, _)| m != n).map(|&(_, c, p)| (c, p)).collect();
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

    /// The edge of `at` to take: the most `Q + U`, none if every edge is dead. At a root with the bandit
    /// (`Config::root_bandit`) each option's `n`, `w` and virtual loss are summed over the live worlds; at a root with
    /// forced playouts (`Config::forced_playouts`) an option under `sqrt(k * P * N)` visits is taken first.
    fn best(&self, at: usize) -> Option<usize> {
        let node = &self.nodes[at];
        let shared = self.shared_slots(at);
        let aggs: Option<Vec<Agg>> = shared.map(|_| (0..self.slot_count()).map(|s| self.agg(s)).collect());
        let stats = |k: usize| -> (u32, f32, u32) {
            match (shared, &aggs) {
                (Some(of), Some(aggs)) => {
                    let a = &aggs[of[k]];
                    (a.n, a.w, a.vl)
                }
                _ => {
                    let e = &node.edges[k];
                    (e.n, e.w, e.vl)
                }
            }
        };
        let total: u32 = match &aggs {
            Some(aggs) => aggs.iter().map(|a| a.n + a.vl).sum(),
            None => node.edges.iter().map(|e| e.n + e.vl).sum(),
        };
        let sqrt = (total.max(1) as f32).sqrt();
        let c = self.config.c_puct;
        let force = (self.config.forced_playouts > 0.0 && self.is_root(at)).then_some(self.config.forced_playouts);
        let mut best: Option<(usize, f32)> = None;
        let mut forced: Option<(usize, f32)> = None;
        for (k, e) in node.edges.iter().enumerate() {
            if e.pruned || e.child == Child::Dead {
                continue;
            }
            let (n, w, vl) = stats(k);
            let visits = n + vl;
            let q = if visits == 0 {
                node.value
            } else {
                (w - vl as f32) / visits as f32
            };
            let score = q + c * e.prior * sqrt / (1.0 + visits as f32);
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((k, score));
            }
            if let Some(kf) = force
                && e.prior > 0.0
                && (visits as f32) < (kf * e.prior * total as f32).sqrt()
                && forced.is_none_or(|(_, s)| score > s)
            {
                forced = Some((k, score));
            }
        }
        forced.or(best).map(|b| b.0)
    }

    /// One simulation's way down world `world`'s tree: from its root, through `forced` there (Gumbel's choice) or the
    /// best edge, to a leaf, a terminal, or a node to refine.
    fn select(&mut self, world: usize, forced: Option<usize>) -> Selected {
        let mut path = Vec::new();
        let mut at = self.forks.worlds[world].root;
        let mut forced = forced;
        loop {
            if !self.nodes[at].expanded {
                if self.nodes[at].queued {
                    return Selected::Collision(path);
                }
                self.nodes[at].queued = true;
                return Selected::Leaf(at, path);
            }
            // A capped line's node: the network's value at the cap, as a terminal (decision 8).
            if self.nodes[at].capped {
                return Selected::Terminal(self.nodes[at].value, self.nodes[at].seat, path);
            }
            // Selected a second time with its inner steps unevaluated (`Config::lazy_inner`): evaluated first.
            if self.nodes[at].lazy.is_some() {
                if self.nodes[at].queued {
                    return Selected::Collision(path);
                }
                self.nodes[at].queued = true;
                return Selected::Refine(at, path);
            }
            let chosen = forced.take();
            let Some(e) = chosen.or_else(|| self.best(at)) else {
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
                    // Refused: take the virtual loss back and choose again here (Gumbel chooses again at the top).
                    self.nodes[at].edges[e].vl -= 1;
                    path.pop();
                    if chosen.is_some() {
                        return Selected::Refused;
                    }
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
    /// At a root with the bandit an option is cut and restored in every world at once, on its summed statistics; with
    /// Gumbel the root is never trimmed (sequential halving chooses there).
    fn prune(&mut self, path: &[(usize, usize)]) {
        if !self.config.prune {
            return;
        }
        let skip = |s: &Self, n: usize| s.config.gumbel.is_some() && s.is_root(n);
        if self.config.prune_reversible {
            for &(n, _) in path {
                if skip(self, n) {
                    continue;
                }
                for k in 0..self.nodes[n].edges.len() {
                    if self.nodes[n].edges[k].pruned && self.trim_holds(n, k).is_none() {
                        self.set_pruned(n, k, false);
                        self.restored += 1;
                    }
                }
            }
        }
        for &(n, e) in path {
            if self.nodes[n].edges[e].pruned {
                break;
            }
            if skip(self, n) {
                continue;
            }
            let Some(kind) = self.prune_kind(n, e) else {
                continue;
            };
            self.set_pruned(n, e, true);
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

    /// Whether the trim rule holds of edge `e` of node `n` against its uncut visited siblings now, cut or not. At a
    /// root with the bandit, on the options' statistics summed over the worlds.
    fn trim_holds(&self, n: usize, e: usize) -> Option<PruneKind> {
        let node = &self.nodes[n];
        let edge = &node.edges[e];
        if let Some(of) = self.shared_slots(n) {
            let a = self.agg(of[e]);
            if a.vl > 0 || a.n == 0 || matches!(edge.child, Child::None | Child::Dead) {
                return None;
            }
            let mut best: Option<f32> = None;
            for s in 0..self.slot_count() {
                if s == of[e] {
                    continue;
                }
                let o = self.agg(s);
                if o.pruned || o.n == 0 || o.dead {
                    continue;
                }
                let q = o.w / o.n as f32;
                best = Some(best.map_or(q, |b: f32| b.max(q)));
            }
            let best = best?;
            let replies = u16::try_from(a.replies).unwrap_or(u16::MAX);
            return trim(a.w / a.n as f32, best, a.n, replies, self.config);
        }
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
            || node.capped
            || !node.machine.chosen().is_empty()
            || !matches!(pos.request.kind, RequestKind::Priority { .. })
        {
            return None;
        }
        flatten(&node.machine, self.config.flatten).filter(|f| !f.inner.is_empty() || f.fallbacks > 0)
    }

    /// Whether leaf `at`'s inner steps (`Flat`) wait until it is selected again (`Config::lazy_inner`): never at a
    /// world's root.
    fn defers_inner(&self, at: usize) -> bool {
        self.config.lazy_inner && !self.is_root(at)
    }

    /// The encoding of node `at`'s position at the step `m` is at.
    fn encoded_at(&self, at: usize, m: &Machine) -> encode::Encoded {
        let node = &self.nodes[at];
        let pos = &self.forks.worlds[node.world].positions[node.pos];
        encode::encode(&pos.obs, m.step().expect("an inner step is at a step"), m.chosen(), &m.names(), self.forks.table)
    }

    /// Node `at` expanded with the network's `value` and `logits` at its step, and, for a priority searched as whole
    /// answers (`flat`), the logits at its inner steps (`inner`, one row each). Given fewer inner rows than inner steps
    /// (`Config::lazy_inner`), each whole answer has its option's first-step prior shared evenly among the option's
    /// whole answers until `refine` gives it the product. A capped line's node takes the value alone.
    fn expand(&mut self, at: usize, value: f32, logits: &[f32], flat: Option<Flat>, inner: &[Vec<f32>]) {
        let t = self.config.prior_temperature.max(1e-3);
        let root = self.is_root(at);
        if self.nodes[at].capped {
            let node = &mut self.nodes[at];
            node.value = value;
            node.expanded = true;
            node.queued = false;
            return;
        }
        if let Some(flat) = flat {
            self.flattened += 1;
            self.flatten_fallbacks += flat.fallbacks;
            let node = &self.nodes[at];
            let step = node.machine.step().expect("a node is at a step");
            let first = softmax_legal(step, logits, t);
            let lazy = inner.len() < flat.inner.len();
            let levels: Vec<BTreeMap<usize, f32>> = if lazy {
                Vec::new()
            } else {
                flat.inner
                    .iter()
                    .zip(inner)
                    .map(|(m, l)| softmax_legal(m.step().expect("an inner step"), l, t))
                    .collect()
            };
            // Lazily, an option's whole answers share its first-step prior evenly.
            let mut answers: BTreeMap<usize, usize> = BTreeMap::new();
            if lazy {
                for c in &flat.combos {
                    *answers.entry(c.first).or_default() += 1;
                }
            }
            let mut steps: BTreeMap<Vec<Token>, Vec<(usize, usize)>> = BTreeMap::new();
            let mut edges: Vec<Edge> = flat
                .combos
                .into_iter()
                .map(|c| {
                    let mut prior = first.get(&c.first).copied().unwrap_or(0.0);
                    if lazy {
                        prior /= answers.get(&c.first).copied().unwrap_or(1).max(1) as f32;
                        steps.insert(c.path.clone(), c.steps);
                    } else {
                        for &(k, choice) in &c.steps {
                            prior *= levels[k].get(&choice).copied().unwrap_or(0.0);
                        }
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
            let rule = edges.iter().any(|e| e.token.kind == Tok::Pass)
                && first.iter().all(|(&i, &p)| step.tokens[i].kind == Tok::Pass || p < self.config.shortcut);
            let shortcut = rule && (self.config.root_shortcut || !root);
            if shortcut {
                edges.retain(|e| e.token.kind == Tok::Pass);
                edges.truncate(1);
                edges[0].prior = 1.0;
            }
            if !shortcut && self.config.pay_split > 0.0 && !self.config.pay_classic {
                edges = self.split_by_payment(at, edges, &first);
            }
            let node = &mut self.nodes[at];
            node.edges = edges;
            node.value = value;
            node.expanded = true;
            node.queued = false;
            node.shortcut = shortcut;
            node.shortcut_would = rule;
            node.lazy = (lazy && !shortcut).then(|| Box::new(Lazy { inner: flat.inner, first, steps }));
            if root {
                self.adjust_root(at);
            }
            self.favour(at);
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
        let rule = node.machine.chosen().is_empty()
            && matches!(pos.request.kind, RequestKind::Priority { .. })
            && edges.iter().any(|e| e.token.kind == Tok::Pass)
            && edges
                .iter()
                .all(|e| e.token.kind == Tok::Pass || e.prior < self.config.shortcut);
        let shortcut = rule && (self.config.root_shortcut || !root);
        if shortcut {
            edges.retain(|e| e.token.kind == Tok::Pass);
            edges[0].prior = 1.0;
        }
        let node = &mut self.nodes[at];
        node.edges = edges;
        node.value = value;
        node.expanded = true;
        node.queued = false;
        node.shortcut = shortcut;
        node.shortcut_would = rule;
        if root {
            self.adjust_root(at);
        }
        self.favour(at);
    }

    /// Whether node `at` is a sub-step of its world's root request (the root included).
    fn in_root_decision(&self, at: usize) -> bool {
        let w = &self.forks.worlds[self.nodes[at].world];
        !w.dead && self.nodes[w.root].pos == self.nodes[at].pos
    }

    /// `Config::favour_paths` on node `at`'s priors, once, as it is expanded: where it is in the root's decision and its
    /// chosen tokens begin a path (see `Config::favour_paths`).
    fn favour(&mut self, at: usize) {
        if self.config.favour_paths.is_empty() || !self.in_root_decision(at) {
            return;
        }
        let node = &mut self.nodes[at];
        if node.shortcut || node.edges.is_empty() {
            return;
        }
        let chosen = node.machine.chosen().to_vec();
        for (path, f) in &self.config.favour_paths {
            let f = f.clamp(0.0, 1.0);
            if f <= 0.0 || path.len() <= chosen.len() || path[..chosen.len()] != chosen[..] {
                continue;
            }
            let rest = &path[chosen.len()..];
            let through = |e: &Edge| through_path(&e.path, rest);
            let (n, share) = node.edges.iter().filter(|e| through(e)).fold((0usize, 0.0f32), |(n, p), e| (n + 1, p + e.prior));
            if n == 0 {
                continue;
            }
            for e in &mut node.edges {
                e.prior = if !through(e) {
                    (1.0 - f) * e.prior
                } else if share > 0.0 {
                    (1.0 - f) * e.prior + f * e.prior / share
                } else {
                    (1.0 - f) * e.prior + f / n as f32
                };
            }
        }
    }

    /// Each favoured path's nodes in world `w`'s root decision (`WorldReport::favoured`).
    fn favoured_in(&self, w: usize) -> Vec<Vec<FavouredAt>> {
        let world = &self.forks.worlds[w];
        self.config
            .favour_paths
            .iter()
            .map(|(path, _)| {
                let mut out = Vec::new();
                if world.dead {
                    return out;
                }
                let mut at = world.root;
                loop {
                    let node = &self.nodes[at];
                    let chosen = node.machine.chosen();
                    if !node.expanded || path.len() <= chosen.len() || path[..chosen.len()] != chosen[..] {
                        break;
                    }
                    let rest = &path[chosen.len()..];
                    let through: Vec<&Edge> = node.edges.iter().filter(|e| through_path(&e.path, rest)).collect();
                    out.push(FavouredAt {
                        chosen: chosen.to_vec(),
                        visits: through.iter().map(|e| e.n).sum(),
                        value_sum: through.iter().map(|e| e.w).sum(),
                        edges: through.len(),
                    });
                    match through.as_slice() {
                        [one] if one.path.len() < rest.len() => match one.child {
                            Child::Node(c) if self.nodes[c].pos == node.pos => at = c,
                            _ => break,
                        },
                        _ => break,
                    }
                }
                out
            })
            .collect()
    }

    /// A lazily expanded node's inner steps evaluated (`Config::lazy_inner`, `inner` one row each): each whole answer's
    /// prior made the product of its steps' as an eager expansion makes it (halved for a class of payment, as
    /// `split_by_payment` halves it), and a root's adjustments made again on them.
    fn refine(&mut self, at: usize, inner: &[Vec<f32>]) {
        let t = self.config.prior_temperature.max(1e-3);
        let Some(lazy) = self.nodes[at].lazy.take() else {
            self.nodes[at].queued = false;
            return;
        };
        let levels: Vec<BTreeMap<usize, f32>> = lazy
            .inner
            .iter()
            .zip(inner)
            .map(|(m, l)| softmax_legal(m.step().expect("an inner step"), l, t))
            .collect();
        let node = &mut self.nodes[at];
        for e in &mut node.edges {
            let Some(steps) = lazy.steps.get(&e.path) else {
                continue;
            };
            let mut prior = lazy.first.get(&e.index).copied().unwrap_or(0.0);
            for &(k, choice) in steps {
                prior *= levels.get(k).and_then(|l| l.get(&choice)).copied().unwrap_or(0.0);
            }
            if e.keep.is_some() {
                prior /= 2.0;
            }
            e.prior = prior;
        }
        node.queued = false;
        let rooted = std::mem::replace(&mut node.rooted, false);
        self.lazy_refined += 1;
        if rooted {
            self.raw_priors.remove(&at);
            self.adjust_root(at);
        }
    }

    /// The root's priors adjusted for exploration, once: the floor (`Config::root_floor`), the floor on one option
    /// (`Config::root_favour`), then the noise
    /// (`Config::root_noise`, drawn by each option's tokens and payment class from the search's seed, so every world
    /// draws the same). The network's own are kept for the report (`RootOption::net_prior`).
    fn adjust_root(&mut self, at: usize) {
        let (floor, noise, seed) = (self.config.root_floor, self.config.root_noise, self.config.seed);
        {
            let node = &mut self.nodes[at];
            if std::mem::replace(&mut node.rooted, true) || node.shortcut || node.edges.is_empty() {
                return;
            }
        }
        let favour = self.config.root_favour.filter(|&(_, e)| e > 0.0);
        if floor <= 0.0 && noise <= 0.0 && favour.is_none() {
            return;
        }
        let raw: Vec<f32> = self.nodes[at].edges.iter().map(|e| e.prior).collect();
        self.raw_priors.insert(at, raw);
        let node = &mut self.nodes[at];
        let k = node.edges.len() as f32;
        if floor > 0.0 {
            for e in &mut node.edges {
                e.prior = (1.0 - floor) * e.prior + floor / k;
            }
        }
        if let Some((token, f)) = favour {
            let through = |e: &Edge| e.path.first().copied().unwrap_or(e.token) == token;
            let (n, share) = node.edges.iter().filter(|e| through(e)).fold((0usize, 0.0f32), |(n, p), e| (n + 1, p + e.prior));
            if n > 0 {
                for e in &mut node.edges {
                    e.prior = if !through(e) {
                        (1.0 - f) * e.prior
                    } else if share > 0.0 {
                        (1.0 - f) * e.prior + f * e.prior / share
                    } else {
                        (1.0 - f) * e.prior + f / n as f32
                    };
                }
            }
        }
        if noise > 0.0 {
            let alpha = self.config.root_noise_alpha.max(1e-3);
            let draws: Vec<f64> = node
                .edges
                .iter()
                .map(|e| gamma(alpha as f64, &mut option_seed(seed ^ 0xd12c_4e7a, &e.path, e.keep.as_deref())))
                .collect();
            let sum: f64 = draws.iter().sum::<f64>().max(1e-300);
            for (e, d) in node.edges.iter_mut().zip(draws) {
                e.prior = (1.0 - noise) * e.prior + noise * (d / sum) as f32;
            }
        }
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

/// Whether two encodings are the same network input (`FeatureBatch` reads nothing else of them).
fn same_input(a: &encode::Encoded, b: &encode::Encoded) -> bool {
    use encode::{CATS, NUMS, PTRS};
    let n = a.len;
    a.len == b.len
        && a.bucket == b.bucket
        && a.first_option == b.first_option
        && a.options == b.options
        && a.kind.get(..n) == b.kind.get(..n)
        && a.card.get(..n) == b.card.get(..n)
        && a.cats.get(..n * CATS) == b.cats.get(..n * CATS)
        && a.ptrs.get(..n * PTRS) == b.ptrs.get(..n * PTRS)
        && a.nums.get(..n * NUMS).map(|x| x.iter().map(|v| v.to_bits()).collect::<Vec<_>>())
            == b.nums.get(..n * NUMS).map(|x| x.iter().map(|v| v.to_bits()).collect::<Vec<_>>())
        && a.legal == b.legal
}

/// A generator's state for one root option, from `seed`, its tokens and its class of payment: the same option draws
/// the same in every world.
fn option_seed(seed: u64, path: &[Token], keep: Option<&Vec<ObjectId>>) -> u64 {
    use std::hash::{Hash, Hasher};
    #[allow(deprecated)]
    let mut h = std::hash::SipHasher::new_with_keys(0x0b7e_5eed, 0x6a7b_c11a);
    path.hash(&mut h);
    keep.map(|k| k.iter().map(|o| o.0).collect::<Vec<_>>()).hash(&mut h);
    splitmix(seed ^ h.finish())
}

/// A uniform draw in (0, 1).
fn uniform(state: &mut u64) -> f64 {
    *state = splitmix(*state);
    ((*state >> 11) as f64 + 0.5) / (1u64 << 53) as f64
}

/// A standard normal draw (Box-Muller).
fn normal(state: &mut u64) -> f64 {
    let (u, v) = (uniform(state), uniform(state));
    (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
}

/// A Gamma(`alpha`, 1) draw (Marsaglia and Tsang), for Dirichlet noise.
fn gamma(alpha: f64, state: &mut u64) -> f64 {
    if alpha < 1.0 {
        let u = uniform(state);
        return gamma(alpha + 1.0, state) * u.powf(1.0 / alpha);
    }
    let d = alpha - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let x = normal(state);
        let v = (1.0 + c * x).powi(3);
        if v <= 0.0 {
            continue;
        }
        let u = uniform(state);
        if u < 1.0 - 0.0331 * x.powi(4) || u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

/// Sequential halving's visit table (`mctx`'s `get_sequence_of_considered_visits`): at the root's `i`th simulation,
/// the visits an option must have to be visited, `m` options considered over `n` simulations.
fn halving_sequence(m: usize, n: usize) -> Vec<u32> {
    if m <= 1 {
        return (0..n as u32).collect();
    }
    let log2 = (m as f64).log2().ceil().max(1.0) as usize;
    let mut out = Vec::with_capacity(n);
    let mut visits = vec![0u32; m];
    let mut considered = m;
    while out.len() < n {
        let extra = (n / (log2 * considered)).max(1);
        for _ in 0..extra {
            out.extend_from_slice(&visits[..considered]);
            for v in &mut visits[..considered] {
                *v += 1;
            }
        }
        considered = (considered / 2).max(2);
    }
    out.truncate(n);
    out
}

impl Search<'_> {
    /// With the bandit, each root option cut in every world or in none (a kept tree's roots were judged one world at a
    /// time): cut where the trim holds on its summed statistics; with Gumbel, never at the root.
    fn unify_root_cuts(&mut self) {
        let roots: Vec<usize> = self.forks.worlds.iter().filter(|w| !w.dead).map(|w| w.root).collect();
        for r in roots {
            for k in 0..self.nodes[r].edges.len() {
                let Some(&slot) = self.shared_slots(r).map(|of| &of[k]) else { continue };
                let cut = self.config.gumbel.is_none()
                    && self.config.prune
                    && self.agg(slot).pruned
                    && self.trim_holds(r, k).is_some();
                self.set_pruned(r, k, cut);
            }
        }
    }

    /// Whether node `at` is its live world's root.
    fn is_root(&self, at: usize) -> bool {
        let w = &self.forks.worlds[self.nodes[at].world];
        !w.dead && w.root == at
    }

    /// The slots of root `at`'s edges, where selection at the root reads every world's statistics.
    fn shared_slots(&self, at: usize) -> Option<&Vec<usize>> {
        if !self.config.bandit() {
            return None;
        }
        let of = self.slots.as_ref()?.of.get(&at)?;
        self.is_root(at).then_some(of)
    }

    fn slot_count(&self) -> usize {
        self.slots.as_ref().map_or(0, |s| s.list.len())
    }

    /// Slot `s`'s statistics over the live worlds.
    fn agg(&self, s: usize) -> Agg {
        let mut a = Agg { dead: true, ..Agg::default() };
        let mut first = true;
        let Some(slots) = self.slots.as_ref() else {
            return a;
        };
        for &(n, e) in &slots.list[s].members {
            if self.forks.worlds[self.nodes[n].world].dead {
                continue;
            }
            let edge = &self.nodes[n].edges[e];
            a.n += edge.n;
            a.w += edge.w;
            a.vl += edge.vl;
            a.replies += u32::from(edge.replies);
            if first {
                a.prior = edge.prior;
                a.raw = self.raw_priors.get(&n).and_then(|v| v.get(e)).copied().unwrap_or(edge.prior);
                first = false;
            }
            let dead = edge.child == Child::Dead;
            a.pruned |= edge.pruned;
            a.dead &= dead;
            a.open |= !dead && !edge.pruned;
        }
        a
    }

    /// Cuts or restores edge `e` of node `n`: at a root with the bandit, the option in every live world.
    fn set_pruned(&mut self, n: usize, e: usize, cut: bool) {
        let Some(&s) = self.shared_slots(n).map(|of| &of[e]) else {
            self.nodes[n].edges[e].pruned = cut;
            return;
        };
        let members = self.slots.as_ref().map(|x| x.list[s].members.clone()).unwrap_or_default();
        for (m, k) in members {
            if !self.forks.worlds[self.nodes[m].world].dead {
                self.nodes[m].edges[k].pruned = cut;
            }
        }
    }

    /// The live worlds' expanded roots' options, each (tokens, class of payment) once, in the worlds' order
    /// (`slots_of`).
    fn build_slots(&self) -> Slots {
        slots_of(
            self.forks
                .worlds
                .iter()
                .filter(|w| !w.dead && self.nodes[w.root].expanded)
                .map(|w| (w.root, &self.nodes[w.root].edges[..])),
        )
    }

    /// The live worlds' roots not expanded yet, evaluated in one batch, each with its inner steps (`Flat`): the roots
    /// that are the same network input (every fresh world's, the root being one observation) by one evaluation
    /// (`Config::shared_root`). The evaluations made (each a simulation) and the rows sent.
    fn expand_roots(&mut self, evaluator: &dyn Evaluator) -> Result<(usize, usize), EvalError> {
        let mut refined = self.refine_roots(evaluator)?;
        let roots: Vec<usize> = self
            .forks
            .worlds
            .iter()
            .filter(|w| !w.dead)
            .map(|w| w.root)
            .filter(|&r| !self.nodes[r].expanded)
            .collect();
        if roots.is_empty() {
            return Ok((0, refined));
        }
        // Each group: its rows (the root's step, then its inner steps), and the roots that are those rows.
        let mut groups: Vec<(Vec<encode::Encoded>, Vec<usize>)> = Vec::new();
        for r in roots {
            let mut own = vec![self.encoded(r)];
            if let Some(f) = self.flat_plan(r) {
                for m in &f.inner {
                    own.push(self.encoded_at(r, m));
                }
            }
            let same = |g: &(Vec<encode::Encoded>, Vec<usize>)| {
                g.0.len() == own.len() && g.0.iter().zip(&own).all(|(a, b)| same_input(a, b))
            };
            match groups.iter_mut().find(|g| same(g)) {
                Some(g) => g.1.push(r),
                None => groups.push((own, vec![r])),
            }
        }
        let sizes: Vec<usize> = groups.iter().map(|g| g.0.len()).collect();
        let members: Vec<Vec<usize>> = groups.iter().map(|g| g.1.clone()).collect();
        let rows: Vec<encode::Encoded> = groups.into_iter().flat_map(|g| g.0).collect();
        let (values, policies, sent) = self.evaluate_rows(evaluator, &rows)?;
        refined += sent;
        let mut row = 0;
        for (size, roots) in sizes.iter().zip(&members) {
            let v = values[row];
            if !v.is_finite() {
                return Err(EvalError(format!("the network valued a root {v}: not a number to search on")));
            }
            for &r in roots {
                let flat = self.flat_plan(r);
                self.expand(r, v.clamp(-1.0, 1.0), &policies[row], flat, &policies[row + 1..row + size]);
            }
            row += size;
        }
        Ok((members.len(), refined))
    }

    /// The live worlds' roots whose inner steps were left unevaluated (a kept tree's, `Config::lazy_inner`), refined:
    /// a root is always evaluated whole. The rows sent.
    fn refine_roots(&mut self, evaluator: &dyn Evaluator) -> Result<usize, EvalError> {
        let pending: Vec<usize> = self
            .forks
            .worlds
            .iter()
            .filter(|w| !w.dead)
            .map(|w| w.root)
            .filter(|&r| self.nodes[r].lazy.is_some())
            .collect();
        if pending.is_empty() {
            return Ok(0);
        }
        let mut rows: Vec<encode::Encoded> = Vec::new();
        let mut spans = Vec::with_capacity(pending.len());
        for &r in &pending {
            let start = rows.len();
            let inner: Vec<encode::Encoded> = self.nodes[r]
                .lazy
                .as_ref()
                .map(|l| l.inner.iter().map(|m| self.encoded_at(r, m)).collect())
                .unwrap_or_default();
            rows.extend(inner);
            spans.push((start, rows.len()));
        }
        let (_, policies, sent) = self.evaluate_rows(evaluator, &rows)?;
        for (&r, &(a, b)) in pending.iter().zip(&spans) {
            self.refine(r, &policies[a..b]);
        }
        Ok(sent)
    }

    /// The network's outputs for `rows` (values, logits), and the rows sent: through the cache where there is one
    /// (`Config::eval_cache`: a row whose input was asked before, or is asked earlier in the same batch, is not sent),
    /// counting repeated inputs where asked (`Config::count_duplicates`). With neither, the rows as they are, as before.
    #[allow(clippy::type_complexity)]
    fn evaluate_rows(
        &mut self,
        evaluator: &dyn Evaluator,
        rows: &[encode::Encoded],
    ) -> Result<(Vec<f32>, Vec<Vec<f32>>, usize), EvalError> {
        if rows.is_empty() {
            return Ok((Vec::new(), Vec::new(), 0));
        }
        let caching = self.cache.max > 0;
        if !caching && !self.config.count_duplicates {
            let out = evaluator.evaluate_positions(&rows.iter().collect::<Vec<_>>())?;
            return Ok((out.value, out.policy, rows.len()));
        }
        let cache = &mut self.cache;
        let keys: Vec<u128> = rows.iter().map(|e| input_key(e, &mut cache.buf)).collect();
        for k in &keys {
            if !cache.seen.insert(*k) {
                cache.duplicates += 1;
            }
        }
        if !caching {
            let out = evaluator.evaluate_positions(&rows.iter().collect::<Vec<_>>())?;
            return Ok((out.value, out.policy, rows.len()));
        }
        // Each row from the cache, or from the one row of its input sent in this batch.
        let mut send: Vec<usize> = Vec::new();
        let mut from: Vec<Option<usize>> = Vec::with_capacity(rows.len());
        let mut sending: std::collections::HashMap<u128, usize> = std::collections::HashMap::new();
        for (r, k) in keys.iter().enumerate() {
            if cache.rows.contains_key(k) {
                cache.hits += 1;
                from.push(None);
            } else if let Some(&i) = sending.get(k) {
                cache.hits += 1;
                from.push(Some(i));
            } else {
                sending.insert(*k, send.len());
                from.push(Some(send.len()));
                send.push(r);
            }
        }
        let out = if send.is_empty() {
            Outputs::default()
        } else {
            evaluator.evaluate_positions(&send.iter().map(|&r| &rows[r]).collect::<Vec<_>>())?
        };
        let mut values = Vec::with_capacity(rows.len());
        let mut policies = Vec::with_capacity(rows.len());
        for (r, f) in from.iter().enumerate() {
            match f {
                Some(i) => {
                    values.push(out.value[*i]);
                    policies.push(out.policy[*i].clone());
                }
                None => {
                    let (v, p) = &cache.rows[&keys[r]];
                    values.push(*v);
                    policies.push(p.clone());
                }
            }
        }
        for (i, &r) in send.iter().enumerate() {
            if cache.rows.len() >= cache.max {
                break;
            }
            cache.rows.insert(keys[r], (out.value[i], out.policy[i].clone()));
        }
        Ok((values, policies, send.len()))
    }

    /// Gumbel's draws and the root's visits as the simulations begin (`Config::gumbel`), over `budget` simulations, and
    /// the options sequential halving considers: `m`, the options open now up to `Gumbel::considered`, fixed for the
    /// search with its visit table (an option refused later leaves the halving without changing its phases).
    fn gumbel_setup(&mut self, g: Gumbel, budget: usize) {
        let n = self.slot_count();
        self.gumbel_n = budget.max(1);
        self.gumbel_base = (0..n).map(|s| self.agg(s).n).collect();
        let open = (0..n).filter(|&s| self.agg(s).open).count();
        self.gumbel_m = g.considered.min(open).max(1);
        self.gumbel_table = halving_sequence(self.gumbel_m, self.gumbel_n);
        let seed = self.config.seed;
        self.gumbel_g = match &self.slots {
            Some(slots) if g.noise => slots
                .list
                .iter()
                .map(|s| {
                    // An interchangeable option (`Slot::dup`) draws its own.
                    let salt = if s.dup == 0 { 0 } else { splitmix(s.dup as u64) };
                    let u = uniform(&mut option_seed(seed ^ 0x6a6d_be17 ^ salt, &s.path, s.keep.as_deref()));
                    (-(-u.ln()).ln()) as f32
                })
                .collect(),
            _ => vec![0.0; n],
        };
    }

    /// A slot's visits in this search (virtual loss included).
    fn fresh(&self, s: usize, a: &Agg) -> u32 {
        (a.n + a.vl).saturating_sub(self.gumbel_base.get(s).copied().unwrap_or(0))
    }

    /// Completed values over the root's options (`mctx`'s `qtransform_completed_by_mix_value`): an option's mean, or
    /// for an unvisited one the mixed value - the root's network value and the visited options' means weighted by the
    /// network's priors (before the root's floor and noise), `(v + N * sum(p q) / sum(p)) / (N + 1)`.
    fn completed_q(&self, aggs: &[Agg]) -> Vec<f32> {
        let v = self.forks.worlds.iter().find(|w| !w.dead).map_or(0.0, |w| self.nodes[w.root].value);
        let (mut sum_n, mut sum_p, mut weighted) = (0.0f32, 0.0f32, 0.0f32);
        for a in aggs.iter().filter(|a| a.n > 0) {
            let p = a.raw.max(f32::MIN_POSITIVE);
            sum_n += a.n as f32;
            sum_p += p;
            weighted += p * (a.w / a.n as f32);
        }
        let mixed = if sum_p > 0.0 { (v + sum_n * weighted / sum_p) / (sum_n + 1.0) } else { v };
        aggs.iter().map(|a| if a.n > 0 { a.w / a.n as f32 } else { mixed }).collect()
    }

    /// `sigma(completed q)` over the root's options (see `Gumbel`), the completed values rescaled over the options not
    /// refused everywhere.
    fn sigma(&self, aggs: &[Agg], g: Gumbel) -> Vec<f32> {
        let q = self.completed_q(aggs);
        let live = || q.iter().zip(aggs).filter(|(_, a)| !a.dead).map(|(q, _)| *q);
        let lo = live().fold(f32::INFINITY, f32::min);
        let hi = live().fold(f32::NEG_INFINITY, f32::max);
        let span = (hi - lo).max(1e-8);
        let most = aggs.iter().map(|a| a.n).max().unwrap_or(0) as f32;
        let scale = (g.c_visit + most) * g.c_scale;
        q.iter().map(|&x| if lo.is_finite() { scale * (x - lo) / span } else { 0.0 }).collect()
    }

    /// The root option and world of the next simulation under Gumbel (see `Gumbel`): the option whose visits in this
    /// search are the ones sequential halving asks now, by `g + logit + sigma(completed q)` (the logit the network's,
    /// before the root's floor and noise); its `j`th visit in the `j`th world it can be taken in, so every option is
    /// visited in the same worlds in turn (a paired comparison). The halving's table is the one `gumbel_setup` made for
    /// its `m`. An option the engine refused in every world is no longer open: a visit the table plans for it goes to
    /// the least visited option still open (the best of them first), so the phases go on as planned for the rest.
    fn gumbel_pick(&mut self) -> Option<(usize, usize)> {
        let g = self.config.gumbel?;
        let n = self.slot_count();
        let aggs: Vec<Agg> = (0..n).map(|s| self.agg(s)).collect();
        let open: Vec<usize> = (0..n).filter(|&s| aggs[s].open).collect();
        if open.is_empty() || self.gumbel_table.is_empty() {
            return None;
        }
        let index: u32 = (0..n).map(|s| self.fresh(s, &aggs[s])).sum();
        let table = &self.gumbel_table;
        let wanted = table[(index as usize).min(table.len() - 1)];
        let sigma = self.sigma(&aggs, g);
        let score = |s: usize| self.gumbel_g[s] + aggs[s].raw.max(1e-30).ln() + sigma[s];
        let best = |among: &mut dyn Iterator<Item = usize>| {
            among.max_by(|&a, &b| score(a).total_cmp(&score(b)).then(b.cmp(&a)))
        };
        let pick = best(&mut open.iter().copied().filter(|&s| self.fresh(s, &aggs[s]) == wanted)).or_else(|| {
            let least = open.iter().map(|&s| self.fresh(s, &aggs[s])).min()?;
            best(&mut open.iter().copied().filter(|&s| self.fresh(s, &aggs[s]) == least))
        })?;
        let slot = &self.slots.as_ref()?.list[pick];
        let avail: Vec<(usize, usize)> = slot
            .members
            .iter()
            .copied()
            .filter(|&(node, e)| {
                !self.forks.worlds[self.nodes[node].world].dead
                    && !self.nodes[node].edges[e].pruned
                    && self.nodes[node].edges[e].child != Child::Dead
            })
            .collect();
        let j = self.fresh(pick, &aggs[pick]) as usize;
        let &(node, edge) = avail.get(j % avail.len().max(1))?;
        Some((self.nodes[node].world, edge))
    }

    /// Gumbel's answer at the root: of the options visited most in this search, the best by `g + logit + sigma`.
    fn gumbel_final(&self) -> Option<usize> {
        let g = self.config.gumbel?;
        let n = self.slot_count();
        let aggs: Vec<Agg> = (0..n).map(|s| self.agg(s)).collect();
        let most = (0..n).filter(|&s| !aggs[s].dead).map(|s| self.fresh(s, &aggs[s])).max()?;
        if most == 0 {
            return None;
        }
        let sigma = self.sigma(&aggs, g);
        let score = |s: usize| self.gumbel_g.get(s).copied().unwrap_or(0.0) + aggs[s].raw.max(1e-30).ln() + sigma[s];
        (0..n)
            .filter(|&s| !aggs[s].dead && self.fresh(s, &aggs[s]) == most)
            .max_by(|&a, &b| score(a).total_cmp(&score(b)).then(b.cmp(&a)))
    }

    /// Gumbel's policy target over the root's options: `softmax(logit + sigma(completed q))`, the logit the network's
    /// (before the root's floor and noise, which are exploration, not a policy to learn), 0 where refused everywhere.
    fn gumbel_target(&self, aggs: &[Agg], g: Gumbel) -> Vec<f32> {
        let sigma = self.sigma(aggs, g);
        let x: Vec<f32> = aggs
            .iter()
            .zip(&sigma)
            .map(|(a, s)| if a.dead { f32::NEG_INFINITY } else { a.raw.max(1e-30).ln() + s })
            .collect();
        let top = x.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        if !top.is_finite() {
            return vec![0.0; aggs.len()];
        }
        let e: Vec<f32> = x.iter().map(|&v| (v - top).exp()).collect();
        let sum: f32 = e.iter().sum::<f32>().max(1e-30);
        e.into_iter().map(|v| v / sum).collect()
    }

    /// KataGo's policy target pruning over the root's options (`Config::forced_playouts`): from each option but the
    /// most visited, up to `sqrt(k * P * N)` visits are taken back as long as its PUCT score, its mean held, stays under
    /// the most visited's; an option left with one visit is taken out.
    fn pruned_target(&self, aggs: &[Agg]) -> Vec<u32> {
        let k = self.config.forced_playouts;
        let c = self.config.c_puct;
        let total: u32 = aggs.iter().map(|a| a.n).sum();
        let sqrt = (total.max(1) as f32).sqrt();
        let Some(star) = (0..aggs.len()).filter(|&s| aggs[s].n > 0).max_by(|&a, &b| aggs[a].n.cmp(&aggs[b].n).then(b.cmp(&a)))
        else {
            return aggs.iter().map(|a| a.n).collect();
        };
        let puct = |a: &Agg, n: u32| a.w / a.n.max(1) as f32 + c * a.prior * sqrt / (1.0 + n as f32);
        let best = puct(&aggs[star], aggs[star].n);
        aggs.iter()
            .enumerate()
            .map(|(s, a)| {
                if s == star || a.n == 0 {
                    return a.n;
                }
                let forced = (k * a.prior * total as f32).sqrt();
                let mut n = a.n;
                while n > 0 && ((a.n - n + 1) as f32) <= forced && puct(a, n - 1) < best {
                    n -= 1;
                }
                if n <= 1 { 0 } else { n }
            })
            .collect()
    }

    /// The live roots' options by tokens (the classes of payment of a split cast summed), cut ones left out, in the
    /// roots' order: visits and summed values.
    fn root_paths(&self) -> Vec<(Vec<Token>, u32, f32)> {
        let mut out: Vec<(Vec<Token>, u32, f32)> = Vec::new();
        for w in self.forks.worlds.iter().filter(|w| !w.dead) {
            for e in self.nodes[w.root].edges.iter().filter(|e| !e.pruned) {
                match out.iter_mut().find(|o| o.0 == e.path) {
                    Some(o) => {
                        o.1 += e.n;
                        o.2 += e.w;
                    }
                    None => out.push((e.path.clone(), e.n, e.w)),
                }
            }
        }
        out
    }

    /// The root's visits now (`Snapshot`).
    fn snapshot(&self, simulations: usize) -> Snapshot {
        let options: Vec<(Vec<Token>, u32)> = self.root_paths().into_iter().map(|(p, n, _)| (p, n)).collect();
        let best = (0..options.len())
            .filter(|&k| options[k].1 > 0)
            .max_by(|&a, &b| options[a].1.cmp(&options[b].1).then(b.cmp(&a)))
            .map(|k| options[k].0.clone());
        Snapshot { simulations, options, best }
    }

    /// Whether the root has nothing to compare (`Config::early_stop`): no option cut in any world, one option holding
    /// the root's visits but for fewer than a batch's (the detours virtual loss makes while the best option's own
    /// visits are few: with a batch of 1, one option visited), and every other option visited at least
    /// `early_stop_margin` below it (no close value).
    fn settled(&self) -> bool {
        let live = || self.forks.worlds.iter().filter(|w| !w.dead);
        if live().any(|w| self.nodes[w.root].edges.iter().any(|e| e.pruned)) {
            return false;
        }
        let paths = self.root_paths();
        let Some(best) = (0..paths.len()).filter(|&k| paths[k].1 > 0).max_by(|&a, &b| paths[a].1.cmp(&paths[b].1).then(b.cmp(&a)))
        else {
            return false;
        };
        let q = paths[best].2 / paths[best].1 as f32;
        let others: u32 = paths.iter().enumerate().filter(|&(k, _)| k != best).map(|(_, p)| p.1).sum();
        others < self.config.batch.max(1) as u32
            && paths
                .iter()
                .enumerate()
                .filter(|&(k, p)| k != best && p.1 > 0)
                .all(|(_, p)| p.2 / p.1 as f32 <= q - self.config.early_stop_margin)
    }

    /// Whether the root is calm enough to stop on (`TimeUse`): its visits' entropy over the options and its value's
    /// distance from the network's both under their bounds.
    fn calm(&self, tu: TimeUse) -> bool {
        let paths = self.root_paths();
        let n: u32 = paths.iter().map(|p| p.1).sum();
        if n == 0 {
            return false;
        }
        let entropy: f32 = paths
            .iter()
            .filter(|p| p.1 > 0)
            .map(|p| {
                let x = p.1 as f32 / n as f32;
                -x * x.ln()
            })
            .sum();
        let mean = paths.iter().map(|p| p.2).sum::<f32>() / n as f32;
        let v = self.forks.worlds.iter().find(|w| !w.dead).map_or(0.0, |w| self.nodes[w.root].value);
        entropy < tu.entropy && (mean - v).abs() < tu.swing
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

/// What the shortcut audit found (`audit_shortcut`).
#[derive(Clone, Debug)]
pub struct Audit {
    /// The search, its root searched with every option (`Config::root_shortcut` off) and `Config::root_floor` under
    /// its priors; `decision.search`'s `root_options` hold each option's visits and values.
    pub decision: Decision,
    /// The root's passing option, and the other option with the best value for the seat among those visited.
    pub pass: Option<RootOption>,
    pub best_other: Option<RootOption>,
    /// `best_other`'s value less passing's, where both were visited: how much the shortcut would have given up.
    pub margin: Option<f32>,
}

/// The floor `audit_shortcut` puts under the root's priors where `Config::root_floor` gives none.
pub const AUDIT_FLOOR: f32 = 0.05;

/// The shortcut audit (the review's item 9): at a request where the priority shortcut would pass at the root, a full
/// search with every option and a floor under the root's priors (`config.root_floor`, `AUDIT_FLOOR` if 0), for a tool
/// counting how often an option the policy excluded wins by a margin. None where the shortcut's rule does not hold at
/// the root (one evaluation of the root says so), or the request is the network's alone. The tree is never kept.
#[allow(clippy::too_many_arguments)]
pub fn audit_shortcut(
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
) -> Result<Option<Audit>, EvalError> {
    let audit = Config {
        root_shortcut: false,
        root_floor: if config.root_floor > 0.0 { config.root_floor } else { AUDIT_FLOOR },
        reuse: Reuse::None,
        ..config.clone()
    };
    // The root alone first: whether the rule holds.
    let probe = Config { simulations: Some(1), deadline: None, shared_root: true, ..audit.clone() };
    let first = search(game, request, knowledge, mode, table, evaluator, &probe, prior, guard, clock)?;
    if !first.search.as_ref().is_some_and(|s| s.shortcut_would) {
        return Ok(None);
    }
    let decision = search(game, request, knowledge, mode, table, evaluator, &audit, prior, guard, clock)?;
    let options = decision.search.as_ref().map(|s| s.root_options.clone()).unwrap_or_default();
    let is_pass = |o: &RootOption| o.path.first().is_some_and(|t| t.kind == Tok::Pass);
    let pass = options.iter().find(|o| is_pass(o)).cloned();
    let best_other = options
        .iter()
        .filter(|o| !is_pass(o) && o.q.is_some())
        .max_by(|a, b| a.q.unwrap_or(f32::MIN).total_cmp(&b.q.unwrap_or(f32::MIN)))
        .cloned();
    let margin = match (&pass, &best_other) {
        (Some(p), Some(b)) => p.q.zip(b.q).map(|(p, b)| b - p),
        _ => None,
    };
    Ok(Some(Audit { decision, pass, best_other, margin }))
}

/// Re-roots a kept tree at the request `game` now asks `seat` (see `Kept`): each live world's root followed down the
/// answers the real game has been given since, to the node of this request. None, and the tree is let go, unless every
/// live world gets there with the seat's observation of the real game; and none across another seat's choice.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn reroot(
    k: Kept,
    game: &Game,
    request: &Request,
    obs: &Observation,
    seat: PlayerId,
    mode: InfoMode,
    count: usize,
    follow_mode: Follow,
    turn_cap: u32,
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
    let Kept { mut worlds, mut nodes, deals, chosen, request: root_request, .. } = k;
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
    info.uncapped = uncap(&worlds, &mut nodes, turn_cap);
    info.principal = principal
        .into_iter()
        .filter(|e| e.1 > 0)
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|e| e.0);
    let nodes = compact(&mut worlds, nodes);
    Some((worlds, nodes, deals, info))
}

/// A kept tree's capped lines under its new roots (`reroot`): a line capped at the old root's horizon
/// (`Reached::CappedAt`: a node valued once, then a terminal) that a new root's horizon (`turn_cap` turns past it)
/// reaches is played again when its edge is next taken (`Child::None`: `materialize`), as a fresh search would play
/// it; the capped node is no longer reachable, and `compact` drops it. One past the new horizon stays capped. The
/// edge's statistics stand, as any edge's whose line is played out again. The edges so opened.
fn uncap(worlds: &[World], nodes: &mut [Node], turn_cap: u32) -> usize {
    let mut opened = 0;
    for w in worlds.iter().filter(|w| !w.dead) {
        let horizon = w.root_turn.saturating_add(turn_cap);
        let mut stack = vec![w.root];
        while let Some(n) = stack.pop() {
            for e in 0..nodes[n].edges.len() {
                let Child::Node(c) = nodes[n].edges[e].child else { continue };
                if !nodes[c].capped {
                    stack.push(c);
                } else if w.positions[nodes[c].pos].turn <= horizon {
                    nodes[n].edges[e].child = Child::None;
                    opened += 1;
                }
            }
        }
    }
    opened
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
    // A capped line's node is a leaf valued at the old horizon, its request never expanded: no root.
    if nodes[at].capped {
        return Err(Lost::Unsearched);
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
    crate::refuse_runaway(&answering.ask)?;
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
        forks: Forks {
            mode,
            table,
            turn_cap: config.turn_cap,
            worlds: Vec::new(),
            refused: 0,
            atomic: config.atomic,
            last_waste: false,
            unpayable: 0,
            t_play: Duration::ZERO,
            t_setup: Duration::ZERO,
            t_goto: Duration::ZERO,
            t_observe: Duration::ZERO,
            stop: config.deadline.map(|d| (clock, d)),
            cap_value: config.cap_value,
            skip_forced_observe: config.skip_forced_observe,
            forced_unobserved: 0,
        },
        nodes: Vec::new(),
        pruned: 0,
        pruned_immediate: 0,
        pruned_shallow: 0,
        restored: 0,
        flattened: 0,
        flatten_fallbacks: 0,
        follow: Default::default(),
        pay_splits: 0,
        slots: None,
        gumbel_g: Vec::new(),
        gumbel_base: Vec::new(),
        gumbel_n: 0,
        gumbel_m: 0,
        gumbel_table: Vec::new(),
        cache: EvalCache { max: config.eval_cache, ..EvalCache::default() },
        lazy_refined: 0,
        capped_valued: 0,
        raw_priors: Default::default(),
    };
    let count = if mode == InfoMode::Full {
        1
    } else {
        config.worlds.max(1)
    };
    let players = game.state.players.len();
    let mut deals: Vec<WorldReport> = Vec::new();
    let mut reused: Option<Reused> = None;
    let mut pre_rows = 0usize;
    if matches!(config.reuse, Reuse::TopUp | Reuse::Add)
        && config.root_shortcut
        && let Some(k) = kept.take()
        && let Some(r) = reroot(k, game, request, &obs, seat, mode, count, config.reuse_follow, config.turn_cap)
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
        // A root is always evaluated whole (`Config::lazy_inner`).
        if config.lazy_inner {
            pre_rows += s.refine_roots(evaluator)?;
        }
        if config.reuse_check {
            info.check = Some(s.check_reroot(game, request, &obs, knowledge, &root_guard, seat, evaluator)?);
        }
        for w in 0..s.forks.worlds.len() {
            if !s.forks.worlds[w].dead {
                let r = s.forks.worlds[w].root;
                s.adjust_root(r);
            }
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
    let mut rows = pre_rows;
    let mut waste_leaves = 0usize;
    let (mut encoding, mut evaluating) = (Duration::ZERO, Duration::ZERO);
    let mut next = 0usize;
    let past = |clock: &dyn Clock| config.deadline.is_some_and(|d| clock.now() >= d);
    let full = || bound.is_some_and(|m| held() >= m);
    let mut memory_bound = false;
    // The roots evaluated before the simulations, one evaluation for the worlds whose roots are the same input
    // (`Config::shared_root`).
    if config.shares_root() && sims < target && !past(clock) {
        let t0 = Instant::now();
        let (evals, sent) = s.expand_roots(evaluator)?;
        evaluating += t0.elapsed();
        sims += evals;
        evaluations += evals;
        rows += sent;
        batches += usize::from(evals > 0);
    }
    if config.bandit() {
        s.slots = Some(s.build_slots());
        s.unify_root_cuts();
    }
    if let Some(g) = config.gumbel {
        s.gumbel_setup(g, if config.simulations.is_some() { target.saturating_sub(sims) } else { g.budget });
    }
    let mut snapshots_left: std::collections::VecDeque<usize> = {
        let mut at = config.snapshots.clone();
        at.sort_unstable();
        at.into()
    };
    let mut snapshots: Vec<Snapshot> = Vec::new();
    let (mut early_stopped, mut time_stopped) = (false, false);
    while sims < target {
        if past(clock) {
            break;
        }
        if full() {
            memory_bound = true;
            break;
        }
        // Between batches (a batch of terminals too): the snapshots asked for, the early stop and the time use.
        while snapshots_left.front().is_some_and(|&at| sims - carried >= at) {
            snapshots_left.pop_front();
            snapshots.push(s.snapshot(sims - carried));
        }
        if config.early_stop > 0 && sims - carried >= config.early_stop && s.settled() {
            early_stopped = true;
            break;
        }
        if let (Some(tu), Some(deadline)) = (config.time_use, config.deadline) {
            let now = clock.now();
            let span = deadline.saturating_duration_since(started).as_secs_f32();
            let share = if span > 0.0 { now.saturating_duration_since(started).as_secs_f32() / span } else { 1.0 };
            if share >= tu.min_share && (s.settled() || s.calm(tu)) {
                time_stopped = true;
                break;
            }
        }
        let alive: Vec<usize> = (0..s.forks.worlds.len()).filter(|&w| !s.forks.worlds[w].dead).collect();
        if alive.is_empty() {
            break;
        }
        // Every root took the shortcut: nothing to search.
        if alive.iter().all(|&w| s.nodes[s.forks.worlds[w].root].shortcut) {
            break;
        }
        // Each entry a leaf to expand, or a node whose inner steps are to be evaluated (`Config::lazy_inner`).
        let mut leaves: Vec<Pending> = Vec::new();
        let mut new_leaves = 0usize;
        let mut progress = false;
        let mut tries = 0;
        // Out of time or memory before the batch is made: the leaves chosen so far are let go unevaluated.
        let mut stopped = false;
        while sims + new_leaves < target && leaves.len() < batch && tries < 4 * batch {
            if past(clock) {
                stopped = true;
                break;
            }
            if full() {
                (stopped, memory_bound) = (true, true);
                break;
            }
            tries += 1;
            // Gumbel chooses the root option, and the world its next visit is in; otherwise the worlds take turns.
            let (world, forced) = if config.gumbel.is_some() {
                match s.gumbel_pick() {
                    Some((w, e)) => (w, Some(e)),
                    None => break,
                }
            } else {
                let alive: Vec<usize> = (0..s.forks.worlds.len()).filter(|&w| !s.forks.worlds[w].dead).collect();
                let Some(&world) = alive.get(next % alive.len().max(1)) else {
                    break;
                };
                next += 1;
                (world, None)
            };
            match s.select(world, forced) {
                Selected::Leaf(n, path) => {
                    leaves.push((n, path, false));
                    new_leaves += 1;
                }
                Selected::Refine(n, path) => leaves.push((n, path, true)),
                Selected::Terminal(v, seat, path) => {
                    s.backup(&path, seat, v);
                    s.prune(&path);
                    sims += 1;
                    progress = true;
                }
                Selected::Collision(path) => {
                    s.unwind(&path);
                    // Gumbel would choose the same again until the batch is evaluated.
                    if forced.is_some() && !leaves.is_empty() {
                        break;
                    }
                }
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
                Selected::Refused => progress = true,
            }
        }
        if stopped {
            for (n, path, _) in &leaves {
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
        // Each leaf's own step, and the inner steps of a priority searched as whole answers (`Flat`), in one batch: a
        // leaf's inner steps only where they are evaluated at once (a root, or `Config::lazy_inner` off); a node
        // selected again with its inner steps unevaluated, those alone.
        let mut flats: Vec<Option<Flat>> = Vec::with_capacity(leaves.len());
        let mut encoded: Vec<encode::Encoded> = Vec::with_capacity(leaves.len());
        let mut first_row = Vec::with_capacity(leaves.len());
        for (n, _, refine) in &leaves {
            first_row.push(encoded.len());
            if *refine {
                let inner: Vec<encode::Encoded> = s.nodes[*n]
                    .lazy
                    .as_ref()
                    .map(|l| l.inner.iter().map(|m| s.encoded_at(*n, m)).collect())
                    .unwrap_or_default();
                encoded.extend(inner);
                flats.push(None);
                continue;
            }
            let flat = s.flat_plan(*n);
            encoded.push(s.encoded(*n));
            if let Some(f) = &flat
                && !s.defers_inner(*n)
            {
                for m in &f.inner {
                    encoded.push(s.encoded_at(*n, m));
                }
            }
            flats.push(flat);
        }
        let t1 = Instant::now();
        let (values, policies, sent) = s.evaluate_rows(evaluator, &encoded)?;
        encoding += t1 - t0;
        evaluating += t1.elapsed();
        batches += 1;
        evaluations += new_leaves;
        rows += sent;
        waste_leaves += leaves.iter().filter(|(n, _, r)| !r && s.nodes[*n].waste).count();
        // The perfect-information value of each leaf's world, where one is given.
        let full_values: Option<Vec<Option<f32>>> = match value {
            Some(v) => {
                let full: Vec<Option<encode::Encoded>> =
                    leaves.iter().map(|(n, _, r)| if *r { None } else { s.encoded_full(*n) }).collect();
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
        for (k, (n, path, refine)) in leaves.iter().enumerate() {
            let r = first_row[k];
            let end = first_row.get(k + 1).copied().unwrap_or(encoded.len());
            if *refine {
                s.refine(*n, &policies[r..end]);
                s.unwind(path);
                continue;
            }
            let v = full_values
                .as_ref()
                .and_then(|f| f[k])
                .unwrap_or(values[r]);
            // A value that is no number would back a NaN up the tree into the root's value (a game's root_values written
            // null): an error to see, never a value to search on. Inputs held to the f16's range (`f16_bits`) keep it so.
            if !v.is_finite() {
                return Err(EvalError(format!("the network valued a leaf {v}: not a number to search on")));
            }
            let v = v.clamp(-1.0, 1.0);
            let flat = flats[k].take();
            s.expand(*n, v, &policies[r], flat, &policies[r + 1..end]);
            let seat = s.nodes[*n].seat;
            s.backup(path, seat, v);
        }
        for (_, path, refine) in &leaves {
            if !refine {
                s.prune(path);
            }
        }
        sims += new_leaves;
    }
    if !config.snapshots.is_empty() {
        snapshots.push(s.snapshot(sims - carried));
    }

    // The root's options over the worlds (`RootOption`), and what is recorded at its first step (`Target`): the
    // visits, Gumbel's target, or the visits with KataGo's target pruning.
    if s.slots.is_none() {
        s.slots = Some(s.build_slots());
    }
    let slot_list: Vec<SlotView> = s
        .slots
        .as_ref()
        .map(|x| x.list.iter().map(|l| (l.path.clone(), l.keep.as_ref().map(|k| k.to_vec()), l.members.clone())).collect())
        .unwrap_or_default();
    // Each slot's options' indices, step by step (`Slot::index`).
    let slot_index: Vec<Vec<usize>> = s.slots.as_ref().map(|x| x.list.iter().map(|l| l.index.clone()).collect()).unwrap_or_default();
    let aggs: Vec<Agg> = (0..slot_list.len()).map(|k| s.agg(k)).collect();
    let live_member = |members: &[(usize, usize)]| {
        members.iter().copied().find(|&(n, _)| !s.forks.worlds[s.nodes[n].world].dead)
    };
    let target_kind = if config.gumbel.is_some() {
        Target::Gumbel
    } else if config.forced_playouts > 0.0 {
        Target::Pruned
    } else {
        Target::Visits
    };
    // Each option's recorded count at the root's first step where it is not its visits.
    let recorded_counts: Option<Vec<u32>> = match (target_kind, config.gumbel) {
        (Target::Gumbel, Some(g)) => Some(
            s.gumbel_target(&aggs, g).iter().map(|&p| (p * TARGET_SCALE).round() as u32).collect(),
        ),
        (Target::Pruned, _) => {
            // The visits as recorded leave cut edges out.
            let uncut: Vec<Agg> = slot_list
                .iter()
                .zip(&aggs)
                .map(|((_, _, members), a)| {
                    let mut u = Agg { prior: a.prior, ..Agg::default() };
                    for &(n, e) in members {
                        let edge = &s.nodes[n].edges[e];
                        if !s.forks.worlds[s.nodes[n].world].dead && !edge.pruned {
                            u.n += edge.n;
                            u.w += edge.w;
                        }
                    }
                    u
                })
                .collect();
            Some(s.pruned_target(&uncut))
        }
        _ => None,
    };
    let recorded_total: f32 = recorded_counts.as_ref().map_or(0.0, |c| c.iter().map(|&n| n as f32).sum());
    let completed = s.completed_q(&aggs);
    let root_options: Vec<RootOption> = slot_list
        .iter()
        .enumerate()
        .filter_map(|(k, (path, keep, members))| {
            let (n, e) = live_member(members)?;
            let a = &aggs[k];
            let net_prior = s.raw_priors.get(&n).and_then(|v| v.get(e)).copied().unwrap_or(s.nodes[n].edges[e].prior);
            Some(RootOption {
                path: path.clone(),
                keep: keep.clone(),
                prior: a.prior,
                net_prior,
                visits: a.n,
                value_sum: a.w,
                q: (a.n > 0).then(|| a.w / a.n as f32),
                completed_q: completed[k],
                target: recorded_counts
                    .as_ref()
                    .map(|c| if recorded_total > 0.0 { c[k] as f32 / recorded_total } else { 0.0 }),
                pruned: a.pruned,
                dead: a.dead,
            })
        })
        .collect();
    let root_prefix: Vec<Token> = s
        .forks
        .worlds
        .iter()
        .find(|w| !w.dead && s.nodes[w.root].expanded)
        .map(|w| s.nodes[w.root].machine.chosen().to_vec())
        .unwrap_or_default();
    // Gumbel's answer at the root: the survivor of sequential halving.
    let gumbel_choice: Option<Vec<Token>> = s.gumbel_final().map(|k| slot_list[k].0.clone());

    // The root's visits, summed over the worlds by prefix and token, which the answer is chosen by (two options of an
    // equal token make the same answer); and as recorded (`recorded`: by prefix and option index, so two options of an
    // equal token are counted apart, and at the root's first step the target where it is not the visits).
    let mut visits: BTreeMap<Vec<Token>, BTreeMap<Token, u32>> = BTreeMap::new();
    let mut recorded: BTreeMap<Vec<Token>, BTreeMap<usize, u32>> = BTreeMap::new();
    // Whole answers' visits at the steps where the root's priority was searched as whole answers (`Flat`).
    let mut wholes: BTreeMap<Vec<Token>, BTreeMap<Vec<Token>, u32>> = BTreeMap::new();
    let (mut root_w, mut root_n) = (0.0f64, 0u64);
    for (at, node) in s.nodes.iter().enumerate() {
        if node.pos != s.forks.worlds[node.world].root_pos || !node.expanded {
            continue;
        }
        let replaced = recorded_counts.is_some() && s.forks.worlds[node.world].root == at;
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
            if !replaced {
                add_by_index(&mut recorded, &base, &e.path, &index_path(e), e.n);
            }
            if e.path.len() > 1 || !e.rest.is_empty() {
                *wholes.entry(base.clone()).or_default().entry(e.path.clone()).or_default() += e.n;
            }
        }
    }
    if let Some(counts) = &recorded_counts {
        for (((path, _, _), index), &n) in slot_list.iter().zip(&slot_index).zip(counts) {
            add_by_index(&mut recorded, &root_prefix, path, index, n);
        }
    }
    // Each root option's values, at each step of its path, by prefix and option index: what `RootStep::q` reads (every
    // world's visits, cut edges included, as `RootOption`'s).
    let mut option_q: BTreeMap<Vec<Token>, BTreeMap<usize, (u64, f64)>> = BTreeMap::new();
    for (((path, _, members), index), a) in slot_list.iter().zip(&slot_index).zip(&aggs) {
        if live_member(members).is_some() {
            let mut prefix = root_prefix.clone();
            for (&t, &i) in path.iter().zip(index) {
                let entry = option_q.entry(prefix.clone()).or_default().entry(i).or_default();
                entry.0 += u64::from(a.n);
                entry.1 += f64::from(a.w);
                prefix.push(t);
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
        root_prefix: root_prefix.clone(),
        root_options,
        target: target_kind,
        steps: Vec::new(),
        snapshots,
        early_stopped,
        time_stopped,
        cache_hits: s.cache.hits,
        duplicate_rows: s.cache.duplicates,
        distinct_rows: s.cache.seen.len(),
        lazy_refined: s.lazy_refined,
        lazy_pending_rows: s.nodes.iter().filter_map(|n| n.lazy.as_ref()).map(|l| l.inner.len()).sum(),
        forced_unobserved: s.forks.forced_unobserved,
        capped_valued: s.capped_valued,
        shortcut_would: s.forks.worlds.iter().filter(|w| !w.dead).any(|w| s.nodes[w.root].shortcut_would),
        equal_tokens: 0,
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
        for (wi, report) in worlds.iter_mut().enumerate() {
            report.favoured = s.favoured_in(wi);
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
    // A step's options as recorded, by index (`recorded`: Gumbel's target, or the pruned visits, at the root's first
    // step), and their root values.
    let record = |prefix: &Vec<Token>, options: &[usize]| -> Vec<u32> {
        options.iter().map(|i| recorded.get(prefix).and_then(|v| v.get(i)).copied().unwrap_or(0)).collect()
    };
    let values = |prefix: &Vec<Token>, options: &[usize]| -> Vec<f32> {
        options
            .iter()
            .map(|i| match option_q.get(prefix).and_then(|v| v.get(i)) {
                Some(&(n, w)) if n > 0 => (w / n as f64) as f32,
                _ => f32::NAN,
            })
            .collect()
    };
    let mut gumbel_taken = false;
    while let Some(step) = m.step() {
        let legal: Vec<usize> = step.legal().collect();
        if legal.is_empty() {
            break;
        }
        steps += 1;
        let prefix = m.chosen().to_vec();
        // Gumbel answers at the root's first step with a choice: the survivor of sequential halving.
        if planned.is_empty()
            && !gumbel_taken
            && legal.len() > 1
            && prefix == root_prefix
            && let Some(path) = &gumbel_choice
        {
            gumbel_taken = true;
            planned.extend(path.iter().copied());
        }
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
                let tokens: Vec<Token> = legal.iter().map(|&j| step.tokens[j]).collect();
                searched.steps.push(StepOrigin { prefix: prefix.clone(), chosen: want, searched: true });
                searched.visits.push(RootStep {
                    visits: record(&prefix, &legal),
                    q: values(&prefix, &legal),
                    options: legal.clone(),
                    prefix,
                    tokens,
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
            let tokens: Vec<Token> = legal.iter().map(|&i| step.tokens[i]).collect();
            searched.steps.push(StepOrigin { prefix: prefix.clone(), chosen: step.tokens[legal[k]], searched: true });
            searched.visits.push(RootStep {
                visits: record(&prefix, &legal),
                q: values(&prefix, &legal),
                options: legal.clone(),
                prefix,
                tokens,
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
            let i = net::pick(&out.policy[0], &legal, config.choice, &mut random);
            searched.steps.push(StepOrigin { prefix, chosen: step.tokens[i], searched: false });
            i
        };
        m.choose(i);
    }
    searched.equal_tokens = searched
        .visits
        .iter()
        .filter(|r| r.tokens.iter().enumerate().any(|(i, t)| r.tokens[..i].contains(t)))
        .count();
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

    fn edge(index: usize, path: Vec<Token>, n: u32) -> Edge {
        Edge {
            index,
            rest: (1..path.len()).map(|k| 10 * k).collect(),
            token: path[0],
            path,
            prior: 0.25,
            n,
            w: n as f32 * 0.5,
            vl: 0,
            replies: 0,
            pruned: false,
            child: Child::None,
            keep: None,
        }
    }

    fn token(n: u32) -> Token {
        Token::of(cantrip_obs::factor::Tok::Pick, cantrip_obs::Ref::Object(n), cantrip_obs::Ref::None, 0)
    }

    /// Two options of an equal token (interchangeable copies) are two root options: their own slot each, the k-th of
    /// one root the k-th of every root, and their visits counted apart by option index; distinct tokens are one slot
    /// over the worlds as before.
    #[test]
    fn options_of_an_equal_token_are_counted_apart() {
        let (a, b) = (token(1), token(2));
        let world = |n: [u32; 3]| vec![edge(0, vec![a], n[0]), edge(1, vec![a], n[1]), edge(2, vec![b, a], n[2])];
        let roots = [world([3, 1, 4]), world([2, 5, 0])];
        let slots = slots_of(roots.iter().enumerate().map(|(r, e)| (r, &e[..])));
        assert_eq!(slots.list.len(), 3);
        assert_eq!(slots.list.iter().map(|s| (s.dup, s.index.clone())).collect::<Vec<_>>(), [(0, vec![0]), (1, vec![1]), (0, vec![2, 10])]);
        assert_eq!(slots.list[0].members, [(0, 0), (1, 0)]);
        assert_eq!(slots.list[1].members, [(0, 1), (1, 1)]);
        assert_eq!(slots.of[&1], [0, 1, 2]);
        // The visits by index: each option its own, a whole answer's at each of its steps.
        let mut by: BTreeMap<Vec<Token>, BTreeMap<usize, u32>> = BTreeMap::new();
        for e in roots.iter().flatten() {
            add_by_index(&mut by, &[], &e.path, &index_path(e), e.n);
        }
        assert_eq!(by[&vec![]], BTreeMap::from([(0, 5), (1, 6), (2, 4)]));
        assert_eq!(by[&vec![b]], BTreeMap::from([(10, 4)]));
        let total: u32 = by[&vec![]].values().sum();
        let shares: f32 = by[&vec![]].values().map(|&n| n as f32 / total as f32).sum();
        assert!((shares - 1.0).abs() < 1e-6);
    }

    /// The first option (a priority's pass) the likeliest, the others alike, and a fixed value: lines that pass along,
    /// into the next turns.
    struct Passing;

    impl Evaluator for Passing {
        fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
            Ok(Outputs {
                value: vec![0.0; b.rows],
                policy: b.options.iter().map(|&(_, n)| (0..n).map(|i| if i == 0 { 4.0 } else { 0.0 }).collect()).collect(),
                aux: vec![[0.0; 2]; b.rows],
            })
        }
    }

    /// Both seats with lands and Shocks: a search's lines run into the next turns.
    const SHOCKS: &str = "[Bot1 Deck]
Mountain x4
Shock x3
Shock
Mountain
Shock
Mountain
Shock x12

[Bot2 Deck]
Mountain x4
Shock x3
Shock
Mountain
Shock
Mountain
Shock x12

[Actions]
Bot1.Hand.Mountain.Play
[T2 Bot2.Main1]
Bot2.Hand.Mountain.Play
[T3 Bot1.Main1]
Bot1.Hand.Mountain.Play
[T4 Bot2.Main1]
Bot2.Hand.Mountain.Play
[T5 Bot1.Main1]
Bot1.Hand.Mountain.Play
[T6 Bot2.Main1]
Bot2.Hand.Mountain.Play
[T7 Bot1.Main1]
Bot1.Hand.Mountain.Play
[Handover Bot1.Pass]

[Expect]
Turn = 7
";

    /// A kept tree re-rooted a turn on: its lines capped at the old horizon that the new one reaches are opened to be
    /// played again (`uncap`), those past it stay capped, and a capped node is no root (`follow`'s refusal is by the
    /// same flag).
    #[test]
    fn capped_lines_under_a_new_root_are_opened_to_its_horizon() {
        let script = cantrip_script::GameScript::parse(SHOCKS, "capped").expect("the script parses");
        let result = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}
{}", f.message, f.report));
        let game = result.game;
        let request = game.pending().cloned().expect("a request");
        let k = Knowledge::of(&game, request.player);
        let config = Config { turn_cap: 1, worlds: 1, batch: 1, ..Config::simulations(1500) };
        let mut kept = None;
        search_keeping(&game, &request, &k, InfoMode::Full, crate::record::table(), &Passing, &config, &Prior::new(), &CancelGuard::default(), &WallClock, &mut kept)
            .expect("a search");
        let Kept { mut worlds, nodes, .. } = kept.expect("the tree is kept");
        let w = &worlds[0];
        let turn = |n: usize| w.positions[nodes[n].pos].turn;
        let root_turn = w.root_turn;
        let mut turns: BTreeMap<(u32, bool), usize> = BTreeMap::new();
        for n in 0..nodes.len() {
            *turns.entry((turn(n), nodes[n].capped)).or_default() += 1;
        }
        assert!(nodes.iter().any(|n| n.capped), "lines were capped: {turns:?}");
        // A node of the next turn with a capped line below it: the new root.
        let below = |r: usize| -> Vec<usize> {
            let mut out = Vec::new();
            let mut stack = vec![r];
            while let Some(n) = stack.pop() {
                for e in &nodes[n].edges {
                    if let Child::Node(c) = e.child {
                        if nodes[c].capped { out.push(c) } else { stack.push(c) }
                    }
                }
            }
            out
        };
        let root = (0..nodes.len())
            .find(|&n| !nodes[n].capped && nodes[n].expanded && turn(n) == root_turn + 1 && !below(n).is_empty())
            .expect("a node of the next turn with capped lines below it");
        let capped_below = below(root);
        assert!(capped_below.iter().all(|&c| turn(c) == root_turn + 2), "capped one turn past the old horizon");
        let mut nodes = nodes;
        // At the old horizon nothing is opened; at the new root's, every capped line below it is.
        assert_eq!(uncap(&worlds, &mut nodes, 1), 0);
        worlds[0].root = root;
        worlds[0].root_turn = root_turn + 1;
        // A cap that the lines are still past leaves them capped.
        assert_eq!(uncap(&worlds, &mut nodes, 0), 0);
        let opened = uncap(&worlds, &mut nodes, 1);
        assert_eq!(opened, capped_below.len());
        let reachable = |nodes: &[Node]| {
            let mut out = Vec::new();
            let mut stack = vec![root];
            while let Some(n) = stack.pop() {
                for e in &nodes[n].edges {
                    if let Child::Node(c) = e.child {
                        out.push(c);
                        stack.push(c);
                    }
                }
            }
            out
        };
        assert!(reachable(&nodes).iter().all(|&c| !nodes[c].capped), "no capped line within the new horizon");
    }

    /// The slots by a map are the slots by a scan: the same options in the same order, with the same members.
    #[test]
    fn slots_by_a_map_are_the_slots_by_a_scan() {
        let tokens: Vec<Token> = (0..40).map(token).collect();
        let roots: Vec<Vec<Edge>> = (0..5)
            .map(|w| (0..40).map(|k| edge(k, vec![tokens[(k * 7 + w) % 40], tokens[k % 3]], k as u32)).collect())
            .collect();
        let slots = slots_of(roots.iter().enumerate().map(|(r, e)| (r, &e[..])));
        let mut scanned: Vec<(Vec<Token>, Vec<(usize, usize)>)> = Vec::new();
        for (r, edges) in roots.iter().enumerate() {
            for (k, e) in edges.iter().enumerate() {
                match scanned.iter_mut().find(|s| s.0 == e.path) {
                    Some(s) => s.1.push((r, k)),
                    None => scanned.push((e.path.clone(), vec![(r, k)])),
                }
            }
        }
        assert_eq!(slots.list.iter().map(|s| (s.path.clone(), s.members.clone())).collect::<Vec<_>>(), scanned);
    }
}
