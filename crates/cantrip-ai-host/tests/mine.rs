//! The pair miner (`cantrip_ai_host::mine`) with the tiny model: floors on given whole answers reach every sub-step of
//! the root's decision they pass through and are reported world by world; two candidate answers that part only at a
//! target are compared there; a pair's fork is found where its answers first differ, with the identity of everything
//! before it proved; and a plan's positions are judged and written as reanalyse's policy targets, the replay checked
//! against the recorded positions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::net::Payment;
use cantrip_ai::search::{self, Config, WallClock};
use cantrip_ai::shard::{self, ShardReader};
use cantrip_ai::trace::Trace;
use cantrip_ai::{Answering, Evaluator, Knowledge, Prior, TractEvaluator, observe_indexed, record::table};
use cantrip_ai_host::mine::{self, JudgeOptions, JudgeRun, Planned};
use cantrip_ai_host::selfplay::{self, Pairing, Run, Start};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::factor::{FactoredAnswer, decompose};

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../cantrip-ai/tests/data/tiny")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(cantrip_ai_host::STACK_SIZE).spawn(f).unwrap().join().unwrap()
}

fn load() -> Result<Box<dyn Evaluator>, String> {
    Ok(Box::new(TractEvaluator::load(&tiny(), table()).map_err(|e| e.to_string())?))
}

/// Bot2 holds three Lightning Bolts and has a Mountain on the battlefield, in its first main phase.
const BOLTS: &str = "[Bot1 Deck]
30 Grizzly Bears

[Bot2 Deck]
3 Lightning Bolt
27 Mountain

[Actions]
[T2 Bot2.Main1]
Bot2.Hand.Mountain.Play
";

fn bolts() -> Game {
    let dir = std::env::temp_dir().join(format!("cantrip-mine-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bolts.cantrip");
    std::fs::write(&path, BOLTS).unwrap();
    let start = Start::load(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    start.game.lock().unwrap().clone()
}

/// The Bolt at two of its targets: two whole answers with the same first token, parting at the target.
fn bolt_answers(game: &Game) -> (Answer, Answer) {
    let request = game.pending().unwrap().clone();
    let k = Knowledge::of(game, request.player);
    let (obs, index) = observe_indexed(game, &k, request.player, InfoMode::Decklist).unwrap();
    let answering = Answering::new(game, &request, &obs, &index);
    let cantrip_obs::AskKind::Priority { options } = &answering.ask.kind else { panic!("a priority ask") };
    let bolt = options.iter().position(|o| o.targets_required > 0).expect("the Bolt is castable");
    let made: Vec<Answer> = options[bolt]
        .target_candidates
        .iter()
        .filter_map(|&t| answering.engine(game, &request, &FactoredAnswer::Act { option: bolt as u32, targets: vec![t], color: None }).ok())
        .collect();
    assert!(made.len() >= 2, "the Bolt at two targets");
    (made[0].clone(), made[1].clone())
}

#[test]
fn floors_on_whole_answers_reach_the_root_and_are_reported_world_by_world() {
    on_big_stack(|| {
        let game = bolts();
        let tiny = TractEvaluator::load(&tiny(), table()).unwrap();
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let request = game.pending().unwrap().clone();
        let k = Knowledge::of(&game, request.player);
        let (a, b) = bolt_answers(&game);
        let (obs, index) = observe_indexed(&game, &k, request.player, InfoMode::Decklist).unwrap();
        let answering = Answering::new(&game, &request, &obs, &index);
        let path = |x: &Answer| decompose(&answering.ask, &[], &answering.factored(&request, x, None).unwrap()).unwrap();
        let (pa, pb) = (path(&a), path(&b));
        assert_eq!(pa[0], pb[0], "one first token");
        assert_ne!(pa, pb);
        let run = |favour: Vec<(Vec<cantrip_obs::factor::Token>, f32)>| {
            let config = Config { worlds: 2, batch: 4, shortcut: 0.0, report: true, favour_paths: favour, ..Config::simulations(32) };
            search::search(&game, &request, &k, InfoMode::Decklist, table(), &tiny, &config, &prior, &CancelGuard::default(), &WallClock).unwrap().search.unwrap()
        };
        let plain = run(Vec::new());
        assert!(plain.report.as_ref().unwrap().worlds.iter().all(|w| w.favoured.is_empty()), "nothing favoured, nothing reported");
        let floored = run(vec![(pa.clone(), 0.3), (pb.clone(), 0.3)]);
        let whole = |o: &search::RootOption, p: &[cantrip_obs::factor::Token]| floored.root_prefix.iter().chain(&o.path).copied().collect::<Vec<_>>() == p;
        let on = |p: &[cantrip_obs::factor::Token]| floored.root_options.iter().filter(|o| whole(o, p)).map(|o| o.prior).sum::<f32>();
        assert!(on(&pa) >= 0.3 * 0.7 - 1e-5, "the first has its floor after the second's: {}", on(&pa));
        assert!(on(&pb) >= 0.3 - 1e-5, "the second has its floor: {}", on(&pb));
        for w in &floored.report.as_ref().unwrap().worlds {
            assert_eq!(w.favoured.len(), 2);
            for f in &w.favoured {
                assert_eq!(f.len(), 1, "a whole answer's edge is at the root alone");
                assert!(f[0].edges >= 1);
                assert_eq!(f[0].chosen, floored.root_prefix);
            }
        }
        let visited: u32 = floored.report.as_ref().unwrap().worlds.iter().map(|w| w.favoured[0][0].visits.min(w.favoured[1][0].visits)).sum();
        assert!(visited > 0, "both are searched");
    });
}

#[test]
fn two_answers_that_part_at_a_target_are_judged_there_paired_by_world() {
    on_big_stack(|| {
        let game = bolts();
        let tiny = TractEvaluator::load(&tiny(), table()).unwrap();
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let (a, b) = bolt_answers(&game);
        let o = JudgeOptions { simulations: 96, worlds: 4, batch: 4, ..JudgeOptions::default() };
        let v = mine::judge(&game, &Payment::default(), [&a, &b], &tiny, &prior, &o, 5).unwrap();
        assert_eq!(v["deeper"], true, "they part after the first token: {v}");
        assert_eq!(v["worlds"].as_array().unwrap().len(), 4);
        assert!(v["visits"][0].as_u64().unwrap() > 0 && v["visits"][1].as_u64().unwrap() > 0, "{v}");
        let paired = v["paired"].as_u64().unwrap();
        if paired >= 2 {
            let (m, lo, hi) = (v["difference"]["mean"].as_f64().unwrap(), v["difference"]["low"].as_f64().unwrap(), v["difference"]["high"].as_f64().unwrap());
            assert!(lo <= m && m <= hi, "{v}");
        }
        assert!(["first", "second", "undecided"].contains(&v["verdict"].as_str().unwrap()));
        // Deterministic per seed; one answer twice is no comparison.
        assert_eq!(mine::judge(&game, &Payment::default(), [&a, &b], &tiny, &prior, &o, 5).unwrap(), v);
        assert!(mine::judge(&game, &Payment::default(), [&a, &a], &tiny, &prior, &o, 5).is_err());
    });
}

fn played(base: &Path, games: usize, seed: u64) -> PathBuf {
    let deck = |n: &str| selfplay::load_deck(&root().join("decks").join(format!("{n}.txt"))).unwrap();
    let prior = Prior::from_decks(&root().join("decks")).unwrap();
    let options = selfplay::Options { model: "tiny".into(), simulations: 8, sampled: 4, max_turns: 4, traces: true, mode: InfoMode::Decklist, ..selfplay::Options::default() };
    let run = Run {
        out: base.join("selfplay"),
        games,
        limit: None,
        lanes: 2,
        seed,
        pairings: vec![Pairing { names: ["rw-aggro".into(), "ub-reanimator".into()], lists: [deck("rw-aggro"), deck("ub-reanimator")] }],
        producer: "selfplay:tiny@8".into(),
        shard_mb: 64,
        shard_games: 100,
        resume: false,
        govern: Default::default(),
        grant: None,
        host: None,
    };
    selfplay::run(&run, &options, &prior, load).unwrap();
    base.join("selfplay")
}

/// `trace` with its answer `at` replaced by another legal answer, and the answers after it dropped.
fn forked(trace: &Trace, at: usize) -> Option<Trace> {
    let (game, mut seats) = trace.replay(at).ok()?;
    let request = game.pending()?.clone();
    let k = Knowledge::of(&game, request.player);
    let (obs, index) = observe_indexed(&game, &k, request.player, InfoMode::Full)?;
    let answering = Answering::new(&game, &request, &obs, &index);
    let given = answering.engine(&game, &request, &trace.steps[at].answer).ok()?;
    let first = cantrip_obs::factor::Machine::new(&answering.ask, &[]);
    let legal: Vec<usize> = first.step()?.legal().collect();
    // Each first option, completed by the first legal option at every later step: the first that is another answer.
    for i in legal {
        let mut m = first.clone();
        m.choose(i);
        while let Some(step) = m.step() {
            let next = step.legal().next()?;
            m.choose(next);
        }
        let Some(f) = m.answer().cloned() else { continue };
        let Ok(answer) = answering.engine(&game, &request, &f) else { continue };
        if answer == given {
            continue;
        }
        let made = seats.factor(&game, &request, &answer, Some(&f))?;
        let mut t = trace.clone();
        t.steps.truncate(at);
        t.steps.push(cantrip_ai::trace::Step { seat: request.player.0, turn: game.state.turn.number, kind: cantrip_ai::kind_name(&request.kind), answer: made, searched: None, by_hand: None });
        return Some(t);
    }
    None
}

#[test]
fn a_pairs_fork_is_its_first_differing_answer_and_the_games_are_one_before_it() {
    on_big_stack(|| {
        let base = std::env::temp_dir().join(format!("cantrip-mine-forks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = played(&base, 2, 71);
        let rec = mine::load(&dir).unwrap();
        assert_eq!(rec.traces.len(), 2);
        let (&seed, trace) = rec.traces.iter().next().unwrap();
        let game = &rec.games[&seed];
        // One game twice: no fork.
        let same = mine::fork((trace, game), (trace, game), rec.mode).unwrap();
        assert_eq!(same["status"], "identical", "{same}");
        // A decision with a choice partway in, answered otherwise.
        let mut found = None;
        for at in (trace.steps.len() / 3)..trace.steps.len() {
            if let Some(t) = forked(trace, at) {
                found = Some((at, t));
                break;
            }
        }
        let (at, other) = found.expect("a decision with another answer");
        let v = mine::fork((trace, game), (&other, game), rec.mode).unwrap();
        assert_eq!(v["status"], "fork", "{v}");
        assert_eq!(v["decision"], at, "{v}");
        for k in ["same_log", "same_board", "same_request", "same_positions", "same_fork_position"] {
            assert_eq!(v["proof"][k], true, "{k}: {v}");
        }
        assert_ne!(v["answers"][0], v["answers"][1]);
        // The other game of the pair: a deal of its own is no pair.
        let (&seed2, trace2) = rec.traces.iter().find(|(s, _)| **s != seed).unwrap();
        assert!(mine::fork((trace, game), (trace2, &rec.games[&seed2]), rec.mode).is_err());
        let _ = std::fs::remove_dir_all(&base);
    });
}

#[test]
fn a_plans_positions_are_judged_and_written_as_reanalyse_targets() {
    on_big_stack(|| {
        let base = std::env::temp_dir().join(format!("cantrip-mine-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = played(&base, 3, 81);
        let rec = mine::load(&dir).unwrap();
        let mut seeds: Vec<u64> = rec.traces.keys().copied().collect();
        seeds.sort_unstable();
        let tiny = load().unwrap();
        let mut plan = Vec::new();
        // A fork position (two candidate answers) in the first game, and plain positions in the others, one held out.
        let first = &rec.traces[&seeds[0]];
        let mut fork_at = None;
        for at in 1..first.steps.len() {
            if let Some(t) = forked(first, at)
                && rec.games[&seeds[0]].samples.iter().any(|s| s.decision == at as u32 && from_choice(s))
            {
                fork_at = Some((at, t.steps[at].clone()));
                break;
            }
        }
        let (at, other) = fork_at.expect("a fork");
        plan.push(Planned { id: "f0".into(), dir: "a".into(), seed: seeds[0], decision: at as u32, split: "train".into(), candidates: Some([first.steps[at].clone(), other]) });
        for (i, &seed) in seeds.iter().enumerate().skip(1) {
            let g = &rec.games[&seed];
            let d: Vec<u32> = g.samples.iter().filter(|s| from_choice(s)).map(|s| s.decision).take(2).collect();
            for (j, d) in d.into_iter().enumerate() {
                plan.push(Planned { id: format!("l{i}-{j}"), dir: "a".into(), seed, decision: d, split: if i == 2 { "heldout".into() } else { "train".into() }, candidates: None });
            }
        }
        drop(tiny);
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let out = base.join("mined");
        let run = JudgeRun { dirs: [("a".to_string(), dir.clone())].into_iter().collect(), out: out.clone(), lanes: 2, producer: "mine:tiny@64".into(), targets: true };
        let o = JudgeOptions { simulations: 64, worlds: 2, batch: 4, search: vec![("RootShortcut".into(), "false".into())], ..JudgeOptions::default() };
        let report = mine::run(&run, &plan, &o, &prior, load).unwrap();
        assert_eq!(report["MismatchedPositions"], 0, "{report}");
        assert!(report["Errors"].as_object().unwrap().is_empty(), "{report}");
        let lines: Vec<serde_json::Value> = std::fs::read_to_string(out.join("judged.jsonl")).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lines.len(), plan.len());
        let f = lines.iter().find(|l| l["id"] == "f0").unwrap();
        assert!(f["judgement"]["verdict"].is_string(), "{f}");
        assert!(f["target"]["kl"].is_number(), "{f}");
        for l in &lines {
            assert!(l.get("error").is_none(), "{l}");
            assert!(l["target"].is_object(), "{l}");
        }
        // The shards: reanalyse's, positions the recorded ones, by split.
        let mut original: HashMap<(u64, u32, u16), Vec<u8>> = HashMap::new();
        for g in rec.games.values() {
            for s in &g.samples {
                original.insert((g.meta.seed, s.decision, s.step), s.position.clone());
            }
        }
        for (split, want) in [("train", plan.iter().filter(|p| p.split == "train").count()), ("heldout", plan.iter().filter(|p| p.split == "heldout").count())] {
            let mut decisions = 0;
            for path in shard::shards(&out.join(split).join("a")).unwrap() {
                let mut reader = ShardReader::open(&path).unwrap();
                assert_eq!(reader.header.producer, "mine:tiny@64");
                while let Some(g) = reader.game().unwrap() {
                    assert!(g.meta.reanalysed);
                    decisions += g.meta.root_values.len();
                    for s in &g.samples {
                        assert_eq!(original.get(&(g.meta.seed, s.decision, s.step)), Some(&s.position), "the replay is the game");
                        assert!(plan.iter().any(|p| p.seed == g.meta.seed && p.decision == s.decision && p.split == split));
                    }
                }
            }
            assert_eq!(decisions, want, "{split}");
        }
        // Kept positions: the fork alone, of the training split.
        let keep: std::collections::HashSet<(String, u64, u32)> = [("a".to_string(), seeds[0], at as u32)].into_iter().collect();
        let kept = base.join("kept");
        assert_eq!(mine::filter(&out.join("train"), &keep, &kept, "mine:tiny@64").unwrap(), (1, 1));
        for path in shard::shards(&kept).unwrap() {
            for g in shard::read(&path).unwrap().1 {
                assert!(g.meta.reanalysed);
                assert!(g.samples.iter().all(|s| s.decision == at as u32 && g.meta.seed == seeds[0]));
                assert_eq!(g.meta.root_values.len(), 1);
            }
        }
        assert!(mine::filter(&out.join("train"), &keep, &kept, "mine:tiny@64").is_err(), "an output with shards is refused");
        // The judgement alone (another network's cross-check): no shards.
        let out2 = base.join("judged-only");
        let run2 = JudgeRun { out: out2.clone(), targets: false, ..run };
        let r2 = mine::run(&run2, &plan[..1], &o, &prior, load).unwrap();
        assert_eq!(r2["Samples"], 0);
        assert!(shard::shards(&out2.join("train")).map(|s| s.is_empty()).unwrap_or(true));
        let _ = std::fs::remove_dir_all(&base);
    });
}

fn from_choice(s: &shard::Sample) -> bool {
    cantrip_obs::encode::from_compact(&s.position).is_some_and(|e| (0..e.options).filter(|&o| e.option_token(o).is_some_and(|t| e.legal[t])).count() > 1)
}

#[test]
fn a_mana_ability_activated_by_hand_is_kept_and_made_again_exactly() {
    on_big_stack(|| {
        let mut game = bolts();
        // Cast the Bolt (at its first target), reaching the payment, where the Mountain's ability is an option.
        let request = game.pending().unwrap().clone();
        let (cast, _) = bolt_answers(&game);
        game.answer(request.id, request.player, cast).unwrap();
        let pay = game.pending().unwrap().clone();
        let RequestKind::PayCosts { options, .. } = &pay.kind else { panic!("a payment: {:?}", pay.kind) };
        let mountain = options.iter().position(|o| o.kind == ActionKind::ActivateAbility).expect("a mana ability to activate");
        let by_hand = Answer::Act(Action::of(&options[mountain]));
        let kept = cantrip_ai::trace::ByHand::of(&pay, &by_hand).expect("an option of the request");
        assert_eq!(kept.option as usize, mountain);
        assert_eq!(kept.answer(&pay).unwrap(), by_hand);
        // Through a trace's step, with the seats' knowledge kept.
        let step = cantrip_ai::trace::Step { seat: pay.player.0, turn: 2, kind: "PayCosts".into(), answer: FactoredAnswer::AutoPay, searched: None, by_hand: Some(kept.clone()) };
        let mut seats = cantrip_ai::trace::Seats::new(&game);
        assert_eq!(seats.step(&game, &pay, &step).unwrap(), by_hand);
        // No action, or an action no option makes: nothing kept.
        assert!(cantrip_ai::trace::ByHand::of(&pay, &Answer::Pass).is_none());
        let wrong = cantrip_ai::trace::ByHand { option: kept.option, action: "no such action".into() };
        assert!(wrong.answer(&pay).is_err());
        // It serialises, and a step without it reads as before.
        let text = serde_json::to_string(&step).unwrap();
        assert_eq!(serde_json::from_str::<cantrip_ai::trace::Step>(&text).unwrap(), step);
        let plain = cantrip_ai::trace::Step { by_hand: None, ..step };
        assert!(!serde_json::to_string(&plain).unwrap().contains("by_hand"));
    });
}
