//! Exploration tooling (the review of 2026-11-13, item 9): a search with a prior floor at its root, to ask what the
//! policy excludes.
//!
//! - `audit`: positions the priority shortcut passed without a search (every option but passing under
//!   `search::Config::shortcut` of the prior), sampled at random from traced games, searched in full with a floor on every
//!   option's prior and the shortcut off; it counts how often an option the policy excluded comes out ahead of passing
//!   by a margin. Rare, and the shortcut is fine; common, and it is hiding plays.
//! - `verify`: the verification step of the people-and-language-models pipeline. Given a position (a `.cantrip`
//!   scenario's, as self-play's `--starts` takes it) and an action X nominated there, a search with a floor on X's prior,
//!   over 16 to 32 worlds, compares X with the search's best other option world by world (the same deal for both: a
//!   paired comparison), and gives the mean difference of their values with an interval.
//!
//! Both floor the root's priors with the search's own options: the audit through `search::audit_shortcut`
//! (`Config::root_floor` under every option, `root_shortcut` off), the verification with `Config::root_favour` on X.
//! Each option's prior as searched and the network's are in the search's `root_options` (`RootOption::prior`,
//! `net_prior`), which say the floor was applied.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use cantrip_ai::eval::MASKED;
use cantrip_ai::net::Payment;
use cantrip_ai::record::table;
use cantrip_ai::search::{self, Config, RootOption, RootStep, WallClock};
use cantrip_ai::trace::Trace;
use cantrip_ai::{Answering, Evaluator, Knowledge, Prior, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::encode::{self, Encoded};
use cantrip_obs::factor::{Machine, Token};
use serde_json::{Value, json};

use crate::selfplay::Start;

/// The softmax of `logits` over `legal`, in `legal`'s order.
pub fn prior(logits: &[f32], legal: &[usize]) -> Vec<f32> {
    let at = |o: usize| logits.get(o).copied().filter(|&x| x > MASKED / 2.0).unwrap_or(f32::NEG_INFINITY);
    let top = legal.iter().map(|&o| at(o)).fold(f32::NEG_INFINITY, f32::max);
    if !top.is_finite() {
        return vec![1.0 / legal.len().max(1) as f32; legal.len()];
    }
    let w: Vec<f32> = legal.iter().map(|&o| (at(o) - top).exp()).collect();
    let sum: f32 = w.iter().sum();
    w.iter().map(|x| x / sum).collect()
}

/// The search's root at the request `game` waits on, for the seat asked: `request` filtered as the seat's search sees
/// it (`Payment::filtered`), its first sub-step with a choice, encoded, and that step's tokens and legal options.
pub struct Root {
    pub request: Request,
    pub encoded: Encoded,
    pub tokens: Vec<Token>,
    pub legal: Vec<usize>,
    /// The forced tokens before it.
    pub prefix: Vec<Token>,
    pub answering: Answering,
    pub machine: Machine,
}

pub fn root(game: &Game, payment: &Payment, knowledge: &Knowledge, mode: InfoMode) -> Result<Root, String> {
    let request = game.pending().cloned().ok_or("the game asks nothing")?;
    let request = payment.filtered(game, &request);
    let (obs, index) = observe_indexed(game, knowledge, request.player, mode).ok_or("no observation of the request")?;
    let answering = Answering::new(game, &request, &obs, &index);
    let mut m = Machine::new(&answering.ask, &[]);
    loop {
        let step = m.step().ok_or("the request has no choice")?;
        let legal: Vec<usize> = step.legal().collect();
        match legal.len() {
            0 => return Err("a step with no legal option".into()),
            1 => m.choose(legal[0]),
            _ => break,
        }
    }
    let step = m.step().ok_or("the request has no choice")?.clone();
    let encoded = encode::encode(&obs, &step, m.chosen(), &m.names(), table());
    let legal: Vec<usize> = step.legal().filter(|&i| encoded.option_token(i).is_some_and(|t| encoded.legal[t])).collect();
    Ok(Root { prefix: m.chosen().to_vec(), request, encoded, tokens: step.tokens.clone(), legal, answering, machine: m })
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// The 97.5% quantile of Student's t with `df` degrees of freedom (a two-sided 95% interval); the normal's past 30.
pub fn t975(df: usize) -> f64 {
    const T: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086,
        2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045, 2.042,
    ];
    match df {
        0 => f64::NAN,
        d if d <= 30 => T[d - 1],
        _ => 1.96,
    }
}

/// The mean of `xs`, and a 95% interval for it (Student's t).
pub fn mean_interval(xs: &[f64]) -> (f64, f64, f64) {
    let n = xs.len() as f64;
    if xs.is_empty() {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    let mean = xs.iter().sum::<f64>() / n;
    if xs.len() < 2 {
        return (mean, f64::NAN, f64::NAN);
    }
    let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    let half = t975(xs.len() - 1) * sd / n.sqrt();
    (mean, mean - half, mean + half)
}

/// The Wilson 95% interval for `k` of `n`.
pub fn wilson(k: u64, n: u64) -> (f64, f64) {
    if n == 0 {
        return (f64::NAN, f64::NAN);
    }
    let (z, n, p) = (1.96f64, n as f64, k as f64 / n as f64);
    let centre = (p + z * z / (2.0 * n)) / (1.0 + z * z / n);
    let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / (1.0 + z * z / n);
    (centre - half, centre + half)
}

// ---------------------------------------------------------------------------------------------------------------
// The shortcut audit

/// How the audit searches.
#[derive(Clone, Debug)]
pub struct AuditOptions {
    /// Shortcut positions to audit.
    pub positions: usize,
    /// Trace positions looked at, at most, while looking for them.
    pub max_looked: usize,
    pub floor: f32,
    pub simulations: usize,
    pub worlds: usize,
    pub batch: usize,
    pub mode: InfoMode,
    /// An excluded option ahead of passing by this much (in value, [-1, 1]) wins.
    pub margin: f32,
    /// ... with at least this many visits.
    pub min_visits: u32,
    /// The shortcut's bound the positions are found with (`search::Config::shortcut`): the search's own by default; a
    /// larger one asks what a wider shortcut would hide.
    pub shortcut: f32,
    pub seed: u64,
}

impl Default for AuditOptions {
    fn default() -> AuditOptions {
        AuditOptions {
            positions: 2000,
            max_looked: 200_000,
            floor: 0.2,
            simulations: 4096,
            worlds: 8,
            batch: 32,
            mode: InfoMode::Decklist,
            margin: 0.05,
            min_visits: 8,
            shortcut: Config::default().shortcut,
            seed: 0xa0d17,
        }
    }
}

/// One position of `trace` (its answer `at`) audited: `None` where the request is no priority the shortcut passed;
/// otherwise a JSON line (see `audit`).
pub fn audit_position(trace: &Trace, start: Option<&Start>, at: usize, evaluator: &dyn Evaluator, prior: &Prior, options: &AuditOptions) -> Result<Option<Value>, String> {
    if trace.steps.get(at).is_none_or(|s| s.kind != "Priority") {
        return Ok(None);
    }
    let (game, payments) = crate::reanalyse::stand_in(trace, start, at)?;
    let request = game.pending().cloned().ok_or("the game asks nothing")?;
    let seat = request.player;
    let k = Knowledge::of(&game, seat);
    let payment = &payments[seat.index()];
    let guard = cantrip_ai::cancel::CancelGuard::default();
    let filtered = payment.filtered(&game, &request);
    // The search's audit: none where the shortcut would not pass here (the root's rule on its priors at
    // `options.shortcut`), else a full search with the floor under every root option and the root's shortcut off.
    let config = Config {
        shortcut: options.shortcut,
        root_floor: options.floor,
        worlds: options.worlds,
        batch: options.batch.max(1),
        seed: splitmix(options.seed ^ trace.seed.rotate_left(11) ^ ((at as u64) << 24)),
        ..Config::simulations(options.simulations)
    };
    let Some(audit) = search::audit_shortcut(&game, &filtered, &k, options.mode, table(), evaluator, &config, prior, &guard, &WallClock).map_err(|e| e.0)?
    else {
        return Ok(None);
    };
    let s = audit.decision.search.as_ref().ok_or("the search made no answer of its own")?;
    let entry = |o: &RootOption| {
        json!({
            "Token": format!("{:?}", o.path),
            "Prior": o.net_prior,
            "SearchPrior": o.prior,
            "Visits": o.visits,
            "Q": o.q,
        })
    };
    let pass = audit.pass.as_ref();
    let is_pass = |o: &RootOption| o.path.first().is_some_and(|t| t.kind == cantrip_obs::factor::Tok::Pass);
    // The best other option by value among those visited enough (`min_visits`).
    let best = s
        .root_options
        .iter()
        .filter(|o| !is_pass(o) && o.visits >= options.min_visits && o.q.is_some())
        .max_by(|a, b| a.q.unwrap_or(f32::MIN).total_cmp(&b.q.unwrap_or(f32::MIN)));
    let most = s.root_options.iter().enumerate().max_by(|(a, x), (b, y)| x.visits.cmp(&y.visits).then(b.cmp(a))).map(|(_, o)| o);
    let margin = match (best.and_then(|b| b.q), pass.and_then(|p| p.q)) {
        (Some(b), Some(p)) => Some(b - p),
        _ => None,
    };
    Ok(Some(json!({
        "seed": trace.seed,
        "at": at,
        "turn": game.state.turn.number,
        "seat": seat.0 + 1,
        "options": s.root_options.len(),
        "floored": s.root_options.iter().any(|o| o.prior != o.net_prior),
        "pass": pass.map(entry),
        "best_excluded": best.map(entry),
        "margin": margin,
        "wins": margin.is_some_and(|m| m >= options.margin),
        "chosen_excluded": most.is_some_and(|m| !is_pass(m)),
        "root_value": s.root_value,
    })))
}

/// Each legal option's prior, by its index in the step.
fn prior_of(logits: &[f32], legal: &[usize]) -> BTreeMap<usize, f32> {
    legal.iter().copied().zip(prior(logits, legal)).collect()
}

/// The audit over `traces` (by seed; `starts` for the games begun at scenario positions): trace positions drawn at
/// random (by `options.seed`) until `options.positions` shortcut positions are audited, on `lanes` lanes. Each audited
/// position's line goes to `out`; the summary is returned.
pub fn audit<F>(traces: &BTreeMap<u64, Trace>, starts: Option<&crate::selfplay::Starts>, options: &AuditOptions, prior: &Prior, lanes: usize, out: &std::path::Path, load: F) -> Result<Value, String>
where
    F: Fn() -> Result<Box<dyn Evaluator>, String> + Send + Sync,
{
    let mut all: Vec<(u64, usize)> = traces.iter().flat_map(|(&seed, t)| t.steps.iter().enumerate().filter(|(_, s)| s.kind == "Priority").map(move |(i, _)| (seed, i))).collect();
    all.sort_by_key(|&(seed, i)| splitmix(options.seed ^ seed.rotate_left(7) ^ (i as u64) << 32));
    all.truncate(options.max_looked);
    let next = AtomicUsize::new(0);
    let found = AtomicUsize::new(0);
    let lines = std::sync::Mutex::new(Vec::<Value>::new());
    let looked = AtomicUsize::new(0);
    let errors = std::sync::Mutex::new(BTreeMap::<String, u64>::new());
    std::thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::new();
        for lane in 0..lanes.max(1) {
            let (all, next, found, lines, looked, errors, load) = (&all, &next, &found, &lines, &looked, &errors, &load);
            let h = std::thread::Builder::new().name(format!("audit-{lane}")).stack_size(crate::STACK_SIZE).spawn_scoped(scope, move || -> Result<(), String> {
                let evaluator = load()?;
                while found.load(Ordering::Relaxed) < options.positions {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(seed, at)) = all.get(i) else { break };
                    looked.fetch_add(1, Ordering::Relaxed);
                    let trace = &traces[&seed];
                    let start = trace.start.as_ref().and_then(|n| starts.and_then(|s| s.list.iter().find(|x| &x.name == n)));
                    match audit_position(trace, start, at, evaluator.as_ref(), prior, options) {
                        Ok(Some(v)) => {
                            if found.fetch_add(1, Ordering::Relaxed) < options.positions {
                                lines.lock().unwrap_or_else(|e| e.into_inner()).push(v);
                            }
                        }
                        Ok(None) => {}
                        Err(e) => *errors.lock().unwrap_or_else(|e| e.into_inner()).entry(e.chars().take(120).collect()).or_default() += 1,
                    }
                }
                Ok(())
            });
            handles.push(h.map_err(|e| e.to_string())?);
        }
        for h in handles {
            h.join().map_err(|_| "a lane panicked".to_string())??;
        }
        Ok(())
    })?;
    let mut lines = lines.into_inner().unwrap_or_else(|e| e.into_inner());
    lines.sort_by_key(|v| (v["seed"].as_u64(), v["at"].as_u64()));
    let text: String = lines.iter().map(|v| format!("{v}\n")).collect();
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    let n = lines.len() as u64;
    let wins = lines.iter().filter(|v| v["wins"] == json!(true)).count() as u64;
    let chosen = lines.iter().filter(|v| v["chosen_excluded"] == json!(true)).count() as u64;
    let unfloored = lines.iter().filter(|v| v["floored"] != json!(true)).count();
    let margins: Vec<f64> = lines.iter().filter_map(|v| v["margin"].as_f64()).collect();
    Ok(json!({
        "Looked": looked.into_inner(),
        "Audited": n,
        "Wins": wins,
        "WinShare": if n > 0 { wins as f64 / n as f64 } else { f64::NAN },
        "WinShareInterval": wilson(wins, n),
        "ChosenExcluded": chosen,
        "ChosenExcludedShare": if n > 0 { chosen as f64 / n as f64 } else { f64::NAN },
        "MeanMargin": mean_interval(&margins).0,
        "Unfloored": unfloored,
        "Errors": errors.into_inner().unwrap_or_else(|e| e.into_inner()),
        "Margin": options.margin,
        "MinVisits": options.min_visits,
        "Floor": options.floor,
        "Shortcut": options.shortcut,
        "Simulations": options.simulations,
        "Worlds": options.worlds,
        "Mode": cantrip_ai::shard::mode_name(options.mode),
    }))
}

// ---------------------------------------------------------------------------------------------------------------
// Verification

/// How a nomination is verified.
#[derive(Clone, Debug)]
pub struct VerifyOptions {
    pub floor: f32,
    pub worlds: usize,
    pub simulations: usize,
    pub batch: usize,
    pub mode: InfoMode,
    pub seed: u64,
}

impl Default for VerifyOptions {
    fn default() -> VerifyOptions {
        VerifyOptions { floor: 0.2, worlds: 16, simulations: 4096, batch: 32, mode: InfoMode::Decklist, seed: 0x7e21f7 }
    }
}

/// Each legal option of the root's first sub-step with a choice, by its index in the step: its token, the network's
/// prior, and its words (`advisor::describe`): the option completed into a whole answer by the network's likeliest
/// choices after it.
pub fn options_at(game: &Game, root: &Root, evaluator: &dyn Evaluator, mode: InfoMode, knowledge: &Knowledge) -> Result<Vec<(usize, Token, f32, String)>, String> {
    let logits = evaluator.evaluate_positions(&[&root.encoded]).map_err(|e| e.0)?.policy.remove(0);
    let p = prior_of(&logits, &root.legal);
    let (obs, _) = observe_indexed(game, knowledge, root.request.player, mode).ok_or("no observation")?;
    let mut out = Vec::new();
    for &i in &root.legal {
        let mut m = root.machine.clone();
        m.choose(i);
        while let Some(s) = m.step() {
            let open: Vec<usize> = s.legal().collect();
            let k = match open.len() {
                0 => break,
                1 => open[0],
                _ => {
                    let e = encode::encode(&obs, s, m.chosen(), &m.names(), table());
                    let l = evaluator.evaluate_positions(&[&e]).map_err(|e| e.0)?.policy.remove(0);
                    *open.iter().max_by(|&&a, &&b| l.get(a).copied().unwrap_or(MASKED).total_cmp(&l.get(b).copied().unwrap_or(MASKED)).then(b.cmp(&a))).expect("open")
                }
            };
            m.choose(k);
        }
        let label = m
            .answer()
            .and_then(|f| root.answering.engine(game, &root.request, f).ok())
            .map(|a| crate::advisor::describe(game, &root.request, &a))
            .unwrap_or_else(|| format!("{:?}", root.tokens[i]));
        out.push((i, root.tokens[i], p.get(&i).copied().unwrap_or(0.0), label));
    }
    Ok(out)
}

/// The option an action names among `options` (`options_at`): `#k` the k-th legal option (from 0), or else the
/// option whose words hold `action` (ignoring case); where several do and their words are the same (copies of a card
/// in hand), the first.
pub fn find_action(options: &[(usize, Token, f32, String)], action: &str) -> Result<usize, String> {
    if let Some(k) = action.strip_prefix('#').and_then(|k| k.parse::<usize>().ok()) {
        return options.get(k).map(|o| o.0).ok_or(format!("no option #{k}: {} options", options.len()));
    }
    let want = action.to_lowercase();
    let hits: Vec<&(usize, Token, f32, String)> = options.iter().filter(|o| o.3.to_lowercase().contains(&want)).collect();
    match hits.as_slice() {
        [one] => Ok(one.0),
        [first, rest @ ..] if rest.iter().all(|o| o.3 == first.3) => Ok(first.0),
        [] => Err(format!("no option is \"{action}\"; the options: {}", options.iter().enumerate().map(|(k, o)| format!("#{k} {}", o.3)).collect::<Vec<_>>().join("; "))),
        many => Err(format!("\"{action}\" names {} options: {}", many.len(), many.iter().map(|o| o.3.clone()).collect::<Vec<_>>().join("; "))),
    }
}

/// The verification of action X (`action`, `find_action`'s) at `game`'s position (see the module): a JSON object with
/// the options, X's and the best other option's values world by world, and the paired difference with its interval.
pub fn verify(game: &Game, action: &str, evaluator: &dyn Evaluator, prior: &Prior, options: &VerifyOptions) -> Result<Value, String> {
    let request = game.pending().cloned().ok_or("the game asks nothing")?;
    let seat = request.player;
    let k = Knowledge::of(game, seat);
    let payment = Payment::default();
    let r = root(game, &payment, &k, options.mode)?;
    let listed = options_at(game, &r, evaluator, options.mode, &k)?;
    let x = find_action(&listed, action)?;
    let x_token = r.tokens[x];
    let config = Config {
        shortcut: 0.0,
        root_favour: Some((x_token, options.floor)),
        worlds: options.worlds,
        batch: options.batch.max(1),
        seed: options.seed,
        report: true,
        ..Config::simulations(options.simulations)
    };
    let guard = cantrip_ai::cancel::CancelGuard::default();
    let d = search::search(game, &r.request, &k, options.mode, table(), evaluator, &config, prior, &guard, &WallClock).map_err(|e| e.0)?;
    let s = d.search.ok_or("the search made no answer of its own")?;
    let report = s.report.as_ref().ok_or("the search made no report")?;
    // Per world, each first option's visits and value summed over its edges (whole answers share their first token).
    let per_world: Vec<BTreeMap<Token, (u32, f64)>> = report
        .worlds
        .iter()
        .map(|w| {
            let mut m: BTreeMap<Token, (u32, f64)> = BTreeMap::new();
            if !w.dead {
                for e in &w.edges {
                    let t = e.path.first().copied().unwrap_or(e.token);
                    let entry = m.entry(t).or_default();
                    entry.0 += e.visits;
                    entry.1 += f64::from(e.value_sum);
                }
            }
            m
        })
        .collect();
    let mut totals: BTreeMap<Token, u32> = BTreeMap::new();
    for w in &per_world {
        for (t, (n, _)) in w {
            *totals.entry(*t).or_default() += n;
        }
    }
    let alt = totals.iter().filter(|(t, _)| **t != x_token).max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(t, _)| *t);
    let q = |w: &BTreeMap<Token, (u32, f64)>, t: Token| w.get(&t).filter(|(n, _)| *n > 0).map(|(n, sum)| sum / f64::from(*n));
    let mut diffs = Vec::new();
    let mut worlds = Vec::new();
    for (i, w) in per_world.iter().enumerate() {
        let (qx, qa) = (q(w, x_token), alt.and_then(|a| q(w, a)));
        if let (Some(a), Some(b)) = (qx, qa) {
            diffs.push(a - b);
        }
        worlds.push(json!({
            "World": i,
            "X": { "Visits": w.get(&x_token).map_or(0, |e| e.0), "Q": qx },
            "Alternative": { "Visits": alt.and_then(|a| w.get(&a)).map_or(0, |e| e.0), "Q": qa },
        }));
    }
    let (mean, lo, hi) = mean_interval(&diffs);
    let label = |t: Token| listed.iter().find(|o| o.1 == t).map(|o| o.3.clone()).unwrap_or_else(|| format!("{t:?}"));
    let first: Option<&RootStep> = s.visits.iter().find(|v| v.prefix == r.prefix);
    let chosen = first.and_then(|f| (0..f.tokens.len()).max_by(|&a, &b| f.visits[a].cmp(&f.visits[b]).then(b.cmp(&a))).map(|k| f.tokens[k]));
    Ok(json!({
        "Seat": seat.0 + 1,
        "Turn": game.state.turn.number,
        "Options": listed.iter().enumerate().map(|(k, o)| json!({ "Index": k, "Option": o.3, "Prior": o.2 })).collect::<Vec<_>>(),
        "Action": label(x_token),
        "ActionPrior": listed.iter().find(|o| o.0 == x).map(|o| o.2),
        "Alternative": alt.map(label),
        "Chosen": chosen.map(label),
        "Floor": options.floor,
        // X's prior as searched (the floor on it) over the root's options through it (`RootOption::prior`).
        "ActionSearchPrior": s.root_options.iter().filter(|o| o.path.first() == Some(&x_token)).map(|o| o.prior).sum::<f32>(),
        "Worlds": options.worlds,
        "Simulations": options.simulations,
        "Mode": cantrip_ai::shard::mode_name(options.mode),
        "Paired": diffs.len(),
        "QDifference": { "Mean": mean, "Low": lo, "High": hi },
        "Verdict": if lo > 0.0 { "better" } else if hi < 0.0 { "worse" } else { "undecided" },
        "PerWorld": worlds,
        "RootValue": s.root_value,
    }))
}
