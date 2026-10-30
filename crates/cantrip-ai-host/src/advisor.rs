//! The advisor's report of one decision: what the AI would do at a request of a seat a person plays, and why, for the
//! advisor's table (`dotnet/tools/Cantrip.Advisor`), which shows it beside the game and records it. Asked with
//! `AiStart`'s `Report`; nothing in play or self-play asks it, and it changes no answer.
//!
//! - **The search's root** (`search::Report`): per world, the deal and the root's options (the network's prior, the
//!   visits, their mean value); the most visited options completed into whole answers, each made into a payload in the
//!   seat's own handles, so the page can play it as the seat's client would.
//! - **The belief**: the network's belief head (the `belief` output, per opponent pool card, the logit that a copy is in
//!   the opponent's hand), beside the exact prior: the hidden part of the hand is a draw without replacement from the
//!   unseen cards, so the chance of at least one copy is hypergeometric.
//! - **The truth**: the hidden state of the real game (each seat's hand, each library's top), in a field of its own
//!   (`Truth`), apart from everything the seat could know. The advisor's table never shows it before the game is over.
//!
//! The labels are words for the engine's answer, the objects named by their cards; the engine stays the only judge of
//! what is legal.

use cantrip_ai::net::Decision;
use cantrip_ai::record::table;
use cantrip_ai::search::{Report, Searched};
use cantrip_ai::trace::{Seats, Trace};
use cantrip_ai::{Answering, EvalError, Knowledge, TractEvaluator, observe_indexed};
use cantrip_engine::*;
use cantrip_host::session::Session;
use cantrip_obs::InfoMode;
use cantrip_obs::encode::{self, Origin};
use cantrip_obs::factor::Machine;
use cantrip_obs::{ObjectObs, ZoneObs};
use serde_json::{Value, json};

/// Belief guesses reported, the likeliest first.
const BELIEF_TOP: usize = 10;
/// Cards of each library's top kept in the truth.
const LIBRARY_TOP: usize = 10;

/// The report of a decision (see the module): `decided` is what the AI made of `request` for its seat.
pub fn analysis(
    session: &Session,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    decided: &Result<Decision, EvalError>,
    belief: Option<&TractEvaluator>,
) -> Value {
    let game = session.game();
    let mut v = json!({});
    let seat = request.player;
    v["Seat"] = json!(seat.0 + 1);
    v["Kind"] = json!(cantrip_ai::kind_name(&request.kind));
    v["Turn"] = json!(game.state.turn.number);
    if let Ok(d) = decided {
        v["Chosen"] = json!({
            "Label": describe(game, request, &d.answer),
            "Payload": session.payload(&d.answer, request).0,
        });
        v["Value"] = json!(d.value);
        let (trivial, why) = trivial(request, d.search.as_ref());
        v["Trivial"] = json!(trivial);
        v["TrivialWhy"] = json!(why);
        if let Some(s) = &d.search {
            v["Search"] = json!({
                "Simulations": s.simulations,
                "Evaluations": s.evaluations,
                "Worlds": s.worlds,
                "RootValue": s.root_value,
                "NetValue": s.net_value,
                "Shortcut": s.shortcut,
                "SearchMs": s.elapsed.as_millis() as u64,
            });
            if let Some(f) = s.fusion {
                v["Fusion"] =
                    json!({ "Worlds": f.worlds, "Distinct": f.distinct, "Plurality": f.plurality });
            }
            if let Some(r) = &s.report {
                report(&mut v, session, request, r, &d.answer);
            }
        }
    }
    match belief.map(|b| belief_of(game, request, knowledge, mode, b)) {
        Some(Ok(b)) => v["Belief"] = b,
        Some(Err(e)) => v["BeliefError"] = json!(e.0),
        None => {}
    }
    v["Truth"] = truth(game);
    v
}

/// Whether the AI treats the request as needing no thought, and why: one legal answer (no search ran), a payment (no
/// policy step), or the priority shortcut (every option but passing below the shortcut's prior).
fn trivial(request: &Request, search: Option<&Searched>) -> (bool, &'static str) {
    match (&request.kind, search) {
        (RequestKind::PayCosts { .. }, None) => (true, "payment"),
        (_, None) => (true, "no choice to search"),
        (_, Some(s)) if s.shortcut => (true, "priority shortcut"),
        (_, Some(s)) if s.simulations == 0 => (true, "no choice to search"),
        _ => (false, ""),
    }
}

/// The search's root into `v`: the candidates with their prior, visit share and value, overall and per world; the
/// worlds' deals and values; and how much the worlds disagreed.
fn report(v: &mut Value, session: &Session, request: &Request, r: &Report, chosen: &Answer) {
    let game = session.game();
    let live: Vec<_> = r
        .worlds
        .iter()
        .filter(|w| !w.dead && !w.edges.is_empty())
        .collect();
    let total: u32 = live
        .iter()
        .flat_map(|w| &w.edges)
        .filter(|e| !e.pruned)
        .map(|e| e.visits)
        .sum();
    let mut candidates = Vec::new();
    for c in &r.candidates {
        let (mut visits, mut sum, mut prior, mut priors) = (0u32, 0.0f64, 0.0f64, 0usize);
        let mut per_world = Vec::new();
        for (i, w) in r.worlds.iter().enumerate() {
            let Some(e) = w.edges.iter().find(|e| e.path == c.path) else {
                continue;
            };
            prior += f64::from(e.prior);
            priors += 1;
            if !e.pruned {
                visits += e.visits;
                sum += f64::from(e.value_sum);
            }
            per_world.push(json!({
                "World": i,
                "Visits": e.visits,
                "Value": (e.visits > 0).then(|| e.value_sum / e.visits as f32),
                "Pruned": e.pruned,
            }));
        }
        let mut entry = json!({
            "Token": format!("{:?}", c.path),
            "Prior": (priors > 0).then(|| prior / priors as f64),
            "Visits": visits,
            "VisitShare": if total > 0 { f64::from(visits) / f64::from(total) } else { 0.0 },
            "Value": (visits > 0).then(|| sum / f64::from(visits)),
            "PrunedWorlds": per_world.iter().filter(|w| w["Pruned"] == json!(true)).count(),
            "PerWorld": per_world,
        });
        match &c.answer {
            Ok(a) => {
                entry["Label"] = json!(describe(game, request, a));
                entry["Payload"] = json!(session.payload(a, request).0);
                entry["Chosen"] = json!(a == chosen);
            }
            Err(why) => {
                entry["Label"] = json!(format!("{:?}", c.factored));
                entry["Refused"] = json!(why);
                entry["Chosen"] = json!(false);
            }
        }
        candidates.push(entry);
    }
    v["Candidates"] = json!(candidates);

    // Each world: its deal, its root value, and the option it visited most.
    let label = |t: &Vec<cantrip_obs::factor::Token>| {
        r.candidates
            .iter()
            .find(|c| c.path == *t)
            .and_then(|c| c.answer.as_ref().ok())
            .map(|a| describe(game, request, a))
            .unwrap_or_else(|| format!("{t:?}"))
    };
    let mut best_values = Vec::new();
    let worlds: Vec<Value> = r
        .worlds
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let best = w
                .edges
                .iter()
                .filter(|e| e.visits > 0 && !e.pruned)
                .max_by(|a, b| a.visits.cmp(&b.visits));
            if let Some(e) = best {
                best_values.push(f64::from(e.value_sum / e.visits as f32));
            }
            json!({
                "World": i,
                "Seed": w.seed.to_string(),
                "Dead": w.dead,
                "NetValue": w.value,
                "Shortcut": w.shortcut,
                "Hands": w.hands.iter().map(|(s, h)| json!({ "Seat": s, "Cards": h })).collect::<Vec<_>>(),
                "LibraryTops": w.library_tops.iter().map(|(s, h)| json!({ "Seat": s, "Cards": h })).collect::<Vec<_>>(),
                "Best": best.map(|e| label(&e.path)),
                "BestValue": best.map(|e| e.value_sum / e.visits as f32),
                "Options": w.edges.iter().map(|e| json!({
                    "Token": format!("{:?}", e.path),
                    "Prior": e.prior,
                    "Visits": e.visits,
                    "Value": (e.visits > 0).then(|| e.value_sum / e.visits as f32),
                    "Pruned": e.pruned,
                    "Dead": e.dead,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    v["Worlds"] = json!(worlds);

    v["Stats"] = stats(r);

    // How much the worlds disagreed: the share of worlds not with the plurality's choice, and the spread of each
    // world's value for its own best option.
    let n = best_values.len() as f64;
    if n >= 2.0 {
        let mean = best_values.iter().sum::<f64>() / n;
        let sd = (best_values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        let (lo, hi) = best_values
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        v["WorldValues"] = json!({ "Mean": mean, "Sd": sd, "Min": lo, "Max": hi });
    }
}

/// The belief head's likeliest cards in each opponent's hidden hand, beside the exact prior (see the module).
fn belief_of(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    belief: &TractEvaluator,
) -> Result<Value, EvalError> {
    let mut k = knowledge.clone();
    k.update(game);
    let seat = request.player;
    let (obs, index) = observe_indexed(game, &k, seat, mode)
        .ok_or_else(|| EvalError("no observation of the request (it is deferred)".into()))?;
    let answering = Answering::new(game, request, &obs, &index);
    let m = Machine::new(&answering.ask, &[]);
    let step = m
        .step()
        .ok_or_else(|| EvalError("the request has no step".into()))?;
    let e = encode::encode(&obs, step, m.chosen(), &m.names(), table());
    let logits = belief.belief(&e)?;
    let mut out = Vec::new();
    for (p, pool) in obs.pools.iter().enumerate() {
        if p == seat.index() || pool.is_empty() {
            continue;
        }
        let unseen: u32 = pool.iter().map(|c| u32::from(c.count)).sum();
        let hand = obs.players.get(p).map_or(0, |pl| u32::from(pl.hand));
        let known = obs
            .objects
            .iter()
            .filter(|o| match o {
                ObjectObs::Hidden {
                    place,
                    owner,
                    identity,
                    ..
                } => usize::from(*owner) == p && place.zone == ZoneObs::Hand && identity.is_some(),
                ObjectObs::Visible(v) => usize::from(v.owner) == p && v.place.zone == ZoneObs::Hand,
                _ => false,
            })
            .count() as u32;
        let hidden = hand.saturating_sub(known);
        let mut cards = Vec::new();
        for c in pool {
            let token = e.origin.iter().position(|o| matches!(o, Origin::PoolCard { player, name } if usize::from(*player) == p && *name == c.name));
            let probability = token
                .and_then(|t| logits.get(t))
                .map(|&x| 1.0 / (1.0 + (-f64::from(x)).exp()));
            cards.push(json!({
                "Card": c.name,
                "Unseen": c.count,
                "Belief": probability,
                "Prior": at_least_one(unseen, u32::from(c.count), hidden),
            }));
        }
        cards.sort_by(|a, b| {
            let x = |v: &Value| v["Belief"].as_f64().unwrap_or(-1.0);
            x(b).total_cmp(&x(a))
        });
        cards.truncate(BELIEF_TOP);
        out.push(
            json!({ "Seat": p + 1, "HiddenInHand": hidden, "Unseen": unseen, "Cards": cards }),
        );
    }
    Ok(json!(out))
}

/// The chance that a draw of `hand` cards without replacement from `pool` cards, `copies` of them one card, holds at least
/// one copy: 1 - C(pool - copies, hand) / C(pool, hand).
fn at_least_one(pool: u32, copies: u32, hand: u32) -> f64 {
    if hand == 0 || copies == 0 {
        return 0.0;
    }
    if hand > pool.saturating_sub(copies) {
        return 1.0;
    }
    let mut none = 1.0f64;
    for i in 0..hand {
        none *= f64::from(pool - copies - i) / f64::from(pool - i);
    }
    1.0 - none
}

/// The real game's hidden state, by card name: each seat's hand and the top of each library, top first.
pub fn truth(game: &Game) -> Value {
    let s = &game.state;
    let name = |id: &ObjectId| game.cards.get(s.obj(*id).card).name.to_string();
    let seats: Vec<Value> = s
        .seats()
        .map(|p| {
            json!({
                "Seat": p.0 + 1,
                "Hand": s.zone(Zone::hand(p)).iter().map(name).collect::<Vec<_>>(),
                "LibraryTop": s.zone(Zone::library(p)).iter().take(LIBRARY_TOP).map(name).collect::<Vec<_>>(),
                "Library": s.zone(Zone::library(p)).len(),
            })
        })
        .collect();
    json!({ "Seats": seats })
}

/// Words for an answer to a request, the objects named by their cards.
pub fn describe(game: &Game, request: &Request, answer: &Answer) -> String {
    let s = &game.state;
    let object = |id: ObjectId| -> String {
        if id.index() < s.objects.len() {
            game.cards.get(s.obj(id).card).name.to_string()
        } else {
            format!("object {}", id.index())
        }
    };
    let player = |p: PlayerId| -> String {
        if p == request.player {
            "yourself".into()
        } else {
            format!("seat {}", p.0 + 1)
        }
    };
    let target = |t: &Target| -> String {
        match *t {
            Target::None => "nothing".into(),
            Target::Player(p) => player(p),
            Target::Object(id) => object(id),
            Target::Ability(n) => format!("ability {n}"),
            #[allow(unreachable_patterns)]
            ref other => format!("{other:?}"),
        }
    };
    let list = |names: Vec<String>| {
        if names.is_empty() {
            "nothing".to_string()
        } else {
            names.join(", ")
        }
    };
    match answer {
        Answer::Pass => "Pass".into(),
        Answer::Act(a) => {
            let source = a.source.map(object).unwrap_or_default();
            let verb = match a.kind {
                ActionKind::PlayLand => "Play".to_string(),
                ActionKind::CastSpell => "Cast".to_string(),
                ActionKind::ActivateAbility => "Activate".to_string(),
                ActionKind::CancelCasting => "Cancel the spell or ability being cast".to_string(),
                ActionKind::PayCost => "Pay the cost".to_string(),
                ActionKind::DeclineCost => "Decline to pay".to_string(),
                ActionKind::ConfirmCasting => "Confirm".to_string(),
                ActionKind::AssignMana => "Spend mana from the pool".to_string(),
                other => format!("{other:?}"),
            };
            let mut text = if source.is_empty() {
                verb
            } else {
                format!("{verb} {source}")
            };
            if !a.targets.is_empty() {
                text.push_str(&format!(
                    " targeting {}",
                    list(a.targets.iter().map(target).collect())
                ));
            }
            if let Some(c) = a.color {
                text.push_str(&format!(" ({c:?})"));
            }
            text
        }
        Answer::Cards(ids) => format!("Choose {}", list(ids.iter().map(|&i| object(i)).collect())),
        Answer::Targets(ts) => format!("Target {}", list(ts.iter().map(target).collect())),
        Answer::Modes(m) => format!(
            "Modes {}",
            list(m.iter().map(|n| (n + 1).to_string()).collect())
        ),
        Answer::Colors(c) => format!("Colors {c:?}"),
        Answer::Yes => match request.kind {
            RequestKind::Mulligan { .. } => "Mulligan".into(),
            _ => "Yes".into(),
        },
        Answer::No => match request.kind {
            RequestKind::Mulligan { .. } => "Keep".into(),
            _ => "No".into(),
        },
        Answer::Attack(pairs) => {
            if pairs.is_empty() {
                "No attack".into()
            } else {
                format!(
                    "Attack with {}",
                    list(
                        pairs
                            .iter()
                            .map(|(a, t)| format!("{} at {}", object(*a), target(t)))
                            .collect()
                    )
                )
            }
        }
        Answer::Block(pairs) => {
            if pairs.is_empty() {
                "No blocks".into()
            } else {
                format!(
                    "Block: {}",
                    list(
                        pairs
                            .iter()
                            .map(|(b, a)| format!("{} blocks {}", object(*b), object(*a)))
                            .collect()
                    )
                )
            }
        }
        Answer::Player(p) => format!("Choose {}", player(*p)),
        Answer::Name(Some(c)) => format!("Name {}", game.cards.get(*c).name),
        Answer::Name(None) => "Name nothing".into(),
        Answer::Order(o) => format!("Order {o:?}"),
        Answer::Placements {
            placements,
            shuffle,
        } => {
            if *shuffle {
                "Shuffle".into()
            } else {
                format!(
                    "Place {}",
                    placements
                        .iter()
                        .enumerate()
                        .map(|(i, cards)| format!(
                            "{}: {}",
                            i + 1,
                            list(cards.iter().map(|&c| object(c)).collect())
                        ))
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            }
        }
        Answer::Divided(ts) => format!(
            "Divide {}",
            list(
                ts.iter()
                    .map(|(t, n)| format!("{n} to {}", target(t)))
                    .collect()
            )
        ),
        Answer::Cancel => "Cancel".into(),
        Answer::Announce(n) => format!("Announce {n}"),
        Answer::Shares(s) => format!(
            "Damage {}",
            list(
                s.iter()
                    .map(|(src, to, n)| format!(
                        "{} deals {n} to {}",
                        object(*src),
                        to.map_or("the defender".to_string(), object)
                    ))
                    .collect()
            )
        ),
        other => format!("{other:?}"),
    }
}

/// The trace of `game` (`cantrip_ai::trace`), a dealt game: its answers, read from the game tree's path to where the game
/// stands, given again to the game dealt as a trace deals it, each kept as the trace keeps it. An error if the game dealt
/// again is not the game: a request that is not the one answered (the table dealt it some other way).
pub fn trace_of(
    game: &Game,
    seed: u64,
    decks: Vec<String>,
    lists: Vec<Vec<String>>,
) -> Result<Trace, String> {
    let mut replay = deal(&lists, seed, true, None)?;
    let mut seats = Seats::new(&replay);
    let mut trace = Trace::new(seed, decks, lists);
    for (i, node) in game.tree.path(game.at).into_iter().enumerate() {
        let Some((id, player, answer)) = game.tree.answer(node) else {
            continue;
        };
        let request = replay
            .pending()
            .cloned()
            .ok_or_else(|| format!("answer {i}: the game dealt again asks nothing"))?;
        if request.id != *id || request.player != *player {
            return Err(format!(
                "answer {i}: the game dealt again asks request {} of seat {}, the game answered request {id} of seat {}",
                request.id,
                request.player.0 + 1,
                player.0 + 1
            ));
        }
        trace.keep(&mut seats, &replay, &request, answer, None, None);
        replay
            .answer(request.id, request.player, answer.clone())
            .map_err(|Rejected(why)| format!("answer {i}: refused when given again: {why}"))?;
    }
    if game.is_over() {
        trace.outcome = game
            .state
            .seats()
            .map(|p| match game.state.outcome {
                Outcome::Won(w) if w == p => 1,
                Outcome::Won(_) => -1,
                _ => 0,
            })
            .collect();
    }
    Ok(trace)
}

/// Lean numbers about a searched decision, which tell a close or contested decision from a clear one: the
/// root's visit entropy (nats, over the first sub-step's options summed over the worlds), the value gap between the two
/// most visited options, the spread of the worlds' values for their own best option, whether the worlds' most visited
/// options differ, and how far the visits moved from the prior (total variation, and whether the top option changed).
/// All read from the finished tree: no evaluation.
pub fn stats(r: &Report) -> Value {
    use std::collections::BTreeMap;
    let mut visits: BTreeMap<String, (u64, f64, f64, usize)> = BTreeMap::new();
    let mut world_best: Vec<String> = Vec::new();
    let mut best_values: Vec<f64> = Vec::new();
    for w in r.worlds.iter().filter(|w| !w.dead && !w.edges.is_empty()) {
        for e in &w.edges {
            let at = visits.entry(format!("{:?}", e.path)).or_default();
            if !e.pruned {
                at.0 += u64::from(e.visits);
                at.1 += f64::from(e.value_sum);
            }
            at.2 += f64::from(e.prior);
            at.3 += 1;
        }
        if let Some(e) = w
            .edges
            .iter()
            .filter(|e| e.visits > 0 && !e.pruned)
            .max_by(|a, b| a.visits.cmp(&b.visits))
        {
            world_best.push(format!("{:?}", e.path));
            best_values.push(f64::from(e.value_sum / e.visits as f32));
        }
    }
    let total: u64 = visits.values().map(|v| v.0).sum();
    if total == 0 {
        return json!(null);
    }
    let entropy: f64 = visits
        .values()
        .filter(|v| v.0 > 0)
        .map(|v| {
            let p = v.0 as f64 / total as f64;
            -p * p.ln()
        })
        .sum();
    let prior_total: f64 = visits
        .values()
        .map(|v| v.2 / v.3.max(1) as f64)
        .sum::<f64>()
        .max(1e-9);
    let tv: f64 = visits
        .values()
        .map(|v| (v.0 as f64 / total as f64 - (v.2 / v.3.max(1) as f64) / prior_total).abs())
        .sum::<f64>()
        / 2.0;
    let mut ranked: Vec<(&String, &(u64, f64, f64, usize))> = visits.iter().collect();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    let mean = |v: &(u64, f64, f64, usize)| (v.0 > 0).then(|| v.1 / v.0 as f64);
    let gap = match (
        ranked.first().and_then(|x| mean(x.1)),
        ranked.get(1).and_then(|x| mean(x.1)),
    ) {
        (Some(a), Some(b)) => Some(a - b),
        _ => None,
    };
    let prior_top = visits
        .iter()
        .max_by(|a, b| (a.1.2 / a.1.3.max(1) as f64).total_cmp(&(b.1.2 / b.1.3.max(1) as f64)))
        .map(|x| x.0.clone());
    let visit_top = ranked.first().map(|x| x.0.clone());
    let distinct = world_best
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let plurality = world_best
        .iter()
        .fold(BTreeMap::<&String, usize>::new(), |mut m, t| {
            *m.entry(t).or_default() += 1;
            m
        })
        .into_values()
        .max()
        .unwrap_or(0);
    let n = best_values.len() as f64;
    let spread = (n >= 2.0).then(|| {
        let mu = best_values.iter().sum::<f64>() / n;
        (best_values.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    });
    json!({
        "Options": visits.len(),
        "VisitEntropy": entropy,
        "ValueGapTop2": gap,
        "WorldValueSd": spread,
        "WorldBestDistinct": distinct,
        "WorldBestDisagree": distinct > 1,
        "WorldBestPluralityShare": if world_best.is_empty() { 0.0 } else { plurality as f64 / world_best.len() as f64 },
        "PriorVisitsTv": tv,
        "PriorTopIsVisitTop": prior_top == visit_top,
        // A re-search at a larger budget, for a sampled subset only (it is expensive): filled by whoever runs it.
        "BiggerBudgetChanges": null,
    })
}
