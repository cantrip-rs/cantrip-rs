//! Self-play (`cantrip_ai_host::selfplay`, docs/ai-player-plan.md "Phase 4"), with the tiny model checked in under
//! `cantrip-ai/tests/data/tiny`, at a small budget and a short game (a few turns, then closed at the turn cap):
//!
//! - every recorded sample's policy target is a distribution summing to 1 over legal options, searched sub-steps
//!   carry the search's root visits (some spread over several options) and their decisions a root value in [-1, 1];
//! - a run writes shards whose header names the producer, and a report that counts what it played.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use cantrip_ai::shard::{self, ShardReader};
use cantrip_ai::{Evaluator, Prior, TractEvaluator, record::table};
use cantrip_ai_host::selfplay::{self, Options, Pairing, Run};
use cantrip_obs::InfoMode;
use cantrip_obs::encode::from_compact;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../cantrip-ai/tests/data/tiny")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pairing() -> Pairing {
    let deck = |n: &str| {
        selfplay::load_deck(&root().join("decks").join(format!("{n}.txt"))).expect("the decklist")
    };
    Pairing {
        names: ["rw-aggro".into(), "ub-reanimator".into()],
        lists: [deck("rw-aggro"), deck("ub-reanimator")],
    }
}

fn options() -> Options {
    Options {
        model: "tiny".into(),
        simulations: 8,
        sampled: 4,
        max_turns: 4,
        ..Options::default()
    }
}

fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(cantrip_ai_host::STACK_SIZE)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn recorded_visits_are_distributions_over_legal_options() {
    on_big_stack(|| {
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
        let played = selfplay::play(&evaluator, &pairing(), 11, 0, &options(), &prior).unwrap();
        assert!(played.searched > 0, "no request was searched");
        assert_eq!(played.network_fallbacks, 0);
        let (game, failures) = selfplay::record(played, InfoMode::Full);
        assert!(failures.is_empty(), "{failures:?}");
        assert!(!game.samples.is_empty());
        let (mut spread, mut rooted, mut searched, mut with_q) = (0, 0, 0, 0);
        for s in &game.samples {
            let sum: f32 = s.visits.iter().map(|v| v.1).sum();
            assert!(
                (sum - 1.0).abs() < 1e-4,
                "visits sum to {sum}: {:?}",
                s.visits
            );
            let e = from_compact(&s.position).unwrap();
            for &(o, p) in &s.visits {
                assert!(p > 0.0);
                let at = e
                    .option_token(o as usize)
                    .expect("a visited option is in the position");
                assert!(e.legal[at], "a visited option is legal");
            }
            spread += (s.visits.len() > 1) as usize;
            // A searched sub-step's visits are the search's, each with its root value where the root's options spell
            // it (NaN where unvisited); any other is one-hot, no q.
            if s.searched {
                assert!(s.q.is_empty() || s.q.len() == s.visits.len(), "a q beside each visit");
                assert!(s.q.iter().all(|q| q.is_nan() || (-1.0..=1.0).contains(q)), "q {:?}", s.q);
                with_q += (!s.q.is_empty()) as usize;
                searched += 1;
            } else {
                assert_eq!(s.visits, vec![(s.chosen, 1.0)], "an unsearched sub-step is the answer given");
                assert!(s.q.is_empty());
            }
            assert!(s.visits.len() == 1 || s.searched, "a spread of visits is a search's");
        }
        assert!(searched > 0, "no sample is marked searched");
        assert!(with_q > 0, "no searched sample has its options' root values");
        for &(decision, value) in &game.meta.root_values {
            assert!((-1.0..=1.0).contains(&value), "root value {value}");
            rooted += game.samples.iter().any(|s| s.decision == decision) as usize;
        }
        assert!(spread > 0, "no searched sub-step spread its visits");
        assert!(rooted > 0, "no root value names a recorded decision");
    });
}

/// The per-sample searched flag is the search's (`Searched::steps`): over a game, a sample is searched for each sub-step
/// the search reached and chose (`StepOrigin::searched`), and the sub-steps of a searched decision the search never
/// reached (the network's answer there) come out not searched, one-hot.
#[test]
fn unreached_sub_steps_of_a_searched_decision_are_not_searched() {
    on_big_stack(|| {
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
        // A full search at a budget so small that a decision's later sub-steps are often never reached.
        let options = Options { simulations: 4, batch: 2, max_turns: 12, ..options() };
        let (mut reached, mut unreached) = (0usize, 0usize);
        let mut games = 0;
        for seed in [11u64, 12, 13] {
            let played = selfplay::play(&evaluator, &pairing(), seed, 0, &options, &prior).unwrap();
            assert_eq!(played.network_fallbacks, 0);
            let steps: Vec<_> = played.roots.values().flat_map(|r| r.steps.iter().cloned()).collect();
            let (r, u) = (steps.iter().filter(|s| s.searched).count(), steps.iter().filter(|s| !s.searched).count());
            let (game, failures) = selfplay::record(played, InfoMode::Full);
            assert!(failures.is_empty(), "{failures:?}");
            let rooted: HashSet<u32> = game.meta.root_values.iter().map(|&(d, _)| d).collect();
            let searched = game.samples.iter().filter(|s| s.searched).count();
            // A searched decision's sub-steps with a choice the search did not reach: one-hot, no q.
            let not_reached: Vec<_> = game
                .samples
                .iter()
                .filter(|s| rooted.contains(&s.decision) && !s.searched)
                .filter(|s| from_compact(&s.position).unwrap().legal.iter().filter(|&&l| l).count() > 1)
                .collect();
            assert!(not_reached.iter().all(|s| s.visits == vec![(s.chosen, 1.0)] && s.q.is_empty()));
            // Each sub-step with a choice of a searched decision is a sample: searched where the search reached it,
            // not searched where it did not - nor where none of the search's visited options is the recorded step's
            // (a reached sub-step whose visits do not name the answer's options is not the search's target).
            assert!(searched <= r, "a searched sample is a sub-step the search reached: {searched} of {r}");
            assert_eq!(searched + not_reached.len(), r + u, "a sample for each sub-step with a choice");
            assert!(game.samples.iter().filter(|s| !rooted.contains(&s.decision)).all(|s| !s.searched));
            (reached, unreached) = (reached + r, unreached + u);
            games += 1;
        }
        assert_eq!(games, 3);
        assert!(reached > 0 && unreached > 0, "reached {reached}, unreached {unreached}");
    });
}

/// Search settings by name reach every search (`Options::search`): a game searched with Gumbel at a shared root in
/// Decklist mode records its targets as distributions over legal options, as visits are.
#[test]
fn gumbel_targets_are_recorded_as_visits_are() {
    on_big_stack(|| {
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
        let options = Options {
            mode: InfoMode::Decklist,
            worlds: 2,
            search: vec!["Gumbel=true".into(), "GumbelNoise=true".into(), "EvalCache=4096".into()],
            ..options()
        };
        let played = selfplay::play(&evaluator, &pairing(), 13, 0, &options, &prior).unwrap();
        assert!(played.searched > 0, "no request was searched");
        let (game, failures) = selfplay::record(played, InfoMode::Decklist);
        assert!(failures.is_empty(), "{failures:?}");
        let mut spread = 0;
        let mut searched = 0;
        for s in &game.samples {
            let sum: f32 = s.visits.iter().map(|v| v.1).sum();
            assert!((sum - 1.0).abs() < 1e-4, "visits sum to {sum}: {:?}", s.visits);
            assert!(s.visits.iter().all(|&(_, p)| p > 0.0 && p <= 1.0), "shares, not the scaled target: {:?}", s.visits);
            spread += (s.visits.len() > 1) as usize;
            if s.searched {
                searched += 1;
                assert!(s.q.is_empty() || s.q.len() == s.visits.len());
                assert!(s.q.iter().all(|q| q.is_nan() || (-1.0..=1.0).contains(q)), "q {:?}", s.q);
            }
        }
        assert!(searched > 0, "no Gumbel target is marked searched");
        assert!(spread > 0, "no Gumbel target spread over options");
        // Every searched decision's target is Gumbel's, and the game says so (train.py's --fast-policy reads it).
        assert!(!game.meta.gumbel_decisions.is_empty(), "no decision is listed as Gumbel's");
        assert!(
            game.meta.root_values.iter().all(|(d, _)| game.meta.gumbel_decisions.contains(d)),
            "a searched decision missing from gumbel_decisions"
        );
        // A setting no search knows is refused.
        let bad = Options { search: vec!["NoSuchSetting=1".into()], ..options };
        assert!(selfplay::play(&evaluator, &pairing(), 13, 0, &bad, &prior).is_err());
    });
}

#[test]
fn a_run_writes_shards_and_resumes_where_it_stopped() {
    on_big_stack(|| {
        let out = std::env::temp_dir().join(format!("cantrip-selfplay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let run = Run {
            out: out.clone(),
            games: 4,
            limit: None,
            lanes: 2,
            seed: 3,
            pairings: vec![pairing()],
            producer: "selfplay:tiny@8".into(),
            shard_mb: 64,
            shard_games: 1,
            resume: false,
            govern: Default::default(),
            grant: None,
            host: None,
        };
        let report = selfplay::run(&run, &options(), &prior, || {
            Ok(
                Box::new(TractEvaluator::load(&tiny(), table()).map_err(|e| e.to_string())?)
                    as Box<dyn Evaluator>,
            )
        })
        .unwrap();
        assert_eq!(report["Games"], 4, "{report}");
        assert_eq!(report["Capped"], 4, "four turns close every game: {report}");
        assert_eq!(report["Resumed"], 0, "{report}");
        let first = shard::shards(&out).unwrap();
        assert_eq!(first.len(), 4, "one game a shard");
        // Interrupted with two shards written, one of them still `.part`: the other two are played again, and the
        // two that read whole (the `.part` recovered and renamed) are not.
        std::fs::remove_file(&first[0]).unwrap();
        let part = first[1].with_file_name(format!(
            "{}.part",
            first[1].file_name().unwrap().to_string_lossy()
        ));
        std::fs::rename(&first[1], &part).unwrap();
        let resumed = Run {
            resume: true,
            ..run
        };
        let report = selfplay::run(&resumed, &options(), &prior, || {
            Ok(
                Box::new(TractEvaluator::load(&tiny(), table()).map_err(|e| e.to_string())?)
                    as Box<dyn Evaluator>,
            )
        })
        .unwrap();
        assert_eq!(report["Games"], 4, "{report}");
        assert_eq!(report["Resumed"], 3, "{report}");
        assert_eq!(report["GamesThisRun"], 1, "{report}");
        assert!(
            !part.exists(),
            "the readable .part was recovered and renamed"
        );
        let mut games = 0;
        let mut seeds = HashSet::new();
        for path in shard::shards(&out).unwrap() {
            let mut reader = ShardReader::open(&path).unwrap();
            assert_eq!(reader.header.producer, "selfplay:tiny@8");
            assert_eq!(reader.header.mode, "full");
            while let Some(g) = reader.game().unwrap() {
                games += 1;
                seeds.insert(g.meta.seed);
                assert!(
                    g.samples
                        .iter()
                        .all(|s| (s.visits.iter().map(|v| v.1).sum::<f32>() - 1.0).abs() < 1e-4)
                );
            }
        }
        assert_eq!(games, 4);
        assert_eq!(seeds.len(), 4, "no game was recorded twice");
        assert!(out.join("selfplay-report.json").exists());
        let _ = std::fs::remove_dir_all(&out);
    });
}

/// A traced game (`cantrip_ai::trace`) dealt and answered again from its trace is the game played: every answer is
/// kept, the replay asks each of them of the same seat, and it ends where the game did. A searched answer keeps its
/// root value.
#[test]
fn a_trace_replays_the_game_it_was_made_of() {
    on_big_stack(|| {
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
        let options = Options {
            traces: true,
            ..options()
        };
        let played = selfplay::play(&evaluator, &pairing(), 12, 1, &options, &prior).unwrap();
        let trace = played.trace.clone().expect("a trace");
        assert!(trace.complete, "every answer is kept");
        assert_eq!(trace.steps.len(), played.answers as usize);
        assert!(trace.steps.iter().any(|s| s.searched.is_some()), "a searched answer keeps its root value");
        let (game, _) = trace.replay(trace.steps.len()).unwrap();
        assert_eq!(game.log, played.game.log, "the replay is the game played");
        assert_eq!(game.state.turn.number, played.game.state.turn.number);
        // Halfway, the same request waits.
        let half = trace.steps.len() / 2;
        let (game, _) = trace.replay(half).unwrap();
        assert_eq!(game.pending().map(|r| r.player.0), Some(trace.steps[half].seat));
    });
}

/// A small scenario a game can begin from: each seat plays a land, and the game stands at Bot2's first main phase.
const START: &str = "[Bot1 Deck]
10 Mountain
10 Grizzly Bears
10 Lightning Bolt

[Bot2 Deck]
10 Mountain
10 Grizzly Bears
10 Lightning Bolt

[Actions]
Bot1.Hand.Mountain.Play
[T2 Bot2.Main1]
Bot2.Hand.Mountain.Play
";

fn starts(name: &str) -> (PathBuf, selfplay::Starts) {
    let dir = std::env::temp_dir().join(format!("cantrip-selfplay-starts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lands-down.cantrip"), START).unwrap();
    let starts = selfplay::Starts::load(&dir).unwrap();
    (dir, starts)
}

/// Against an earlier champion (`Options::opponents`) only the learner's decisions are search targets: the champion's
/// seat is recorded one-hot, unsearched, with no root value, and the meta names the learner's seat.
#[test]
fn a_game_against_an_earlier_champion_records_the_learner_alone() {
    on_big_stack(|| {
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let learner = TractEvaluator::load(&tiny(), table()).unwrap();
        let older = TractEvaluator::load(&tiny(), table()).unwrap();
        let players = selfplay::Players {
            evaluators: [&learner, &older],
            learner: [true, false],
            names: ["tiny".into(), "older".into()],
            beliefs: [None, None],
            abort: None,
            meter: None,
        };
        let played = selfplay::play_with(&players, &pairing(), 11, 1, None, &options(), &prior).unwrap();
        let (game, failures) = selfplay::record(played, InfoMode::Full);
        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(game.meta.learners, vec![1]);
        assert_eq!(game.meta.names, vec!["tiny1".to_string(), "older2".to_string()]);
        let theirs: Vec<_> = game.samples.iter().filter(|s| s.seat == 1).collect();
        assert!(!theirs.is_empty());
        assert!(theirs.iter().all(|s| !s.searched && s.visits == vec![(s.chosen, 1.0)] && s.q.is_empty()));
        assert!(game.samples.iter().any(|s| s.seat == 0 && s.searched), "the learner searched");
        for &(d, _) in &game.meta.root_values {
            assert!(game.samples.iter().filter(|s| s.decision == d).all(|s| s.seat == 0), "a root value of the champion's");
        }
    });
}

/// A game from a scenario's position (`Options::starts`): the script's answers make no samples, the decisions count
/// from the position, and the trace stands the game in again on the scenario's game.
#[test]
fn a_game_from_a_scenario_start_records_from_the_position() {
    on_big_stack(|| {
        let (dir, starts) = starts("one");
        let start = &starts.list[0];
        assert_eq!(start.name, "lands-down");
        let at = start.game.lock().unwrap().state.turn.number;
        assert_eq!(at, 2);
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
        let options = Options { traces: true, ..options() };
        let players = selfplay::Players::mirror(&evaluator, "tiny");
        let played = selfplay::play_with(&players, &pairing(), 5, 0, Some(start), &options, &prior).unwrap();
        let trace = played.trace.clone().expect("a trace");
        assert!(trace.complete);
        assert_eq!(trace.start.as_deref(), Some("lands-down"));
        assert!(trace.replay(1).is_err(), "a scenario's game is not dealt");
        let (again, _) = trace.replay_on(start.game_for(5), trace.steps.len()).unwrap();
        assert_eq!(again.log, played.game.log, "the replay from the position is the game played");
        let (game, failures) = selfplay::record(played, InfoMode::Full);
        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(game.meta.start, "lands-down");
        assert_eq!(game.meta.answers as usize, trace.steps.len(), "the answers counted from the position");
        assert!(game.samples.iter().all(|s| (s.decision as usize) < trace.steps.len()));
        assert!(game.samples.iter().all(|s| s.turn >= 2), "nothing before the position");
        assert_eq!(game.samples.first().map(|s| s.decision), Some(0));
        let _ = std::fs::remove_dir_all(&dir);
    });
}

/// A run with a share of its games against an earlier champion and a share from scenario starts: each game's
/// assignment is its index's (`selfplay::assignment`), and the shards and the report say which games were which.
#[test]
fn a_run_mixes_opponents_and_starts_by_the_games_index() {
    on_big_stack(|| {
        let (dir, starts) = starts("run");
        let out = std::env::temp_dir().join(format!("cantrip-selfplay-mixed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let options = Options {
            opponents: vec![selfplay::Opponent { name: "older".into(), share: 0.5 }],
            starts: Some(std::sync::Arc::new(starts)),
            start_share: 0.5,
            ..options()
        };
        let run = Run {
            out: out.clone(),
            games: 8,
            limit: None,
            lanes: 2,
            seed: 21,
            pairings: vec![pairing()],
            producer: "selfplay:tiny@8".into(),
            shard_mb: 64,
            shard_games: 100,
            resume: false,
            govern: Default::default(),
            grant: None,
            host: None,
        };
        let load = || Ok(Box::new(TractEvaluator::load(&tiny(), table()).map_err(|e| e.to_string())?) as Box<dyn Evaluator>);
        let report = selfplay::run_with(&run, &options, &prior, load, |_| load()).unwrap();
        assert_eq!(report["Games"], 8, "{report}");
        let (mut against, mut started) = (0u64, 0u64);
        for path in shard::shards(&out).unwrap() {
            let mut reader = ShardReader::open(&path).unwrap();
            while let Some(g) = reader.game().unwrap() {
                let i = (g.meta.seed - run.seed) as usize;
                let (opponent, start) = selfplay::assignment(i, run.seed, &options);
                assert_eq!(!g.meta.learners.is_empty(), opponent.is_some(), "game {i}");
                if let Some((_, seat)) = opponent {
                    assert_eq!(g.meta.learners, vec![seat as u8 + 1]);
                    assert!(g.meta.names[1 - seat].starts_with("older"));
                }
                assert_eq!(!g.meta.start.is_empty(), start.is_some(), "game {i}");
                against += u64::from(opponent.is_some());
                started += u64::from(start.is_some());
            }
        }
        assert!(against > 0 && against < 8, "{against} of 8 against the champion");
        assert!(started > 0 && started < 8, "{started} of 8 from a start");
        assert_eq!(report["Opponents"]["older"]["GamesThisRun"], against, "{report}");
        assert_eq!(report["Starts"]["GamesThisRun"]["lands-down"], started, "{report}");
        assert!(report["Snapshots"].is_null(), "no snapshots until the search keeps them");
        let _ = std::fs::remove_dir_all(&out);
        let _ = std::fs::remove_dir_all(&dir);
    });
}

/// Decklist self-play with worlds dealt by the belief head (`Options::belief`): the head is read at every search and
/// none fails; the games play as any other.
#[test]
fn decklist_self_play_deals_by_the_belief_head() {
    on_big_stack(|| {
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
        let belief = TractEvaluator::load(&tiny(), table()).unwrap();
        let options = Options { mode: InfoMode::Decklist, worlds: 2, ..options() };
        let mut players = selfplay::Players::mirror(&evaluator, "tiny");
        players.beliefs = [Some(&belief), Some(&belief)];
        let played = selfplay::play_with(&players, &pairing(), 13, 0, None, &options, &prior).unwrap();
        assert!(played.searched > 0);
        assert_eq!(played.belief_errors, 0);
        let again = selfplay::play_with(&players, &pairing(), 13, 0, None, &options, &prior).unwrap();
        assert_eq!(again.game.log, played.game.log, "the same seed plays the same game");
    });
}

/// A scenario whose seat knows its library's top: Bot1 casts Preordain on its first turn, keeps both cards on top and
/// draws the first; the Lightning Bolt it left on top is a card it knows the place of.
const KNOWN_TOP: &str = "[Bot1 Deck]
Island
Preordain
Mountain x5
Grizzly Bears
Lightning Bolt
Mountain x6
Grizzly Bears x6
Lightning Bolt x6

[Bot2 Deck]
Mountain x7
Grizzly Bears x8
Lightning Bolt x8

[Actions]
Bot1.Hand.Island.Play
Bot1.Battlefield.Island.Activate
Bot1.Hand.Preordain.Cast
Bot1.Pass
Bot1.Arrange(Grizzly Bears; Lightning Bolt |)
[T2 Bot2.Main1]
Bot2.Hand.Mountain.Play
";

fn start_of(name: &str, text: &str) -> selfplay::Start {
    let dir = std::env::temp_dir().join(format!("cantrip-selfplay-start-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.cantrip"));
    std::fs::write(&path, text).unwrap();
    let start = selfplay::Start::load(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    start
}

/// Games from one scenario start draw differently: each is dealt from it by its seed, its libraries' unseen cards in
/// another order (the same seed, the same game), and nothing a seat knows moves - the card a seat saw put on top stays
/// there, every seat's knowledge is what it was, and a script keeps its tops with `# selfplay: keep-top N`.
#[test]
fn games_from_a_start_draw_differently_and_keep_what_a_seat_knows() {
    use cantrip_ai::Knowledge;
    use cantrip_engine::*;
    on_big_stack(|| {
        let start = start_of("known-top", KNOWN_TOP);
        assert_eq!(start.keep_top, 0);
        let base = start.game.lock().unwrap().clone();
        let library = |g: &Game, p: u8| g.state.zone(Zone::library(PlayerId(p))).to_vec();
        let names = |g: &Game, p: u8| library(g, p).iter().map(|&id| g.name(id).to_string()).collect::<Vec<_>>();
        let known = |g: &Game, p: u8| Knowledge::of(g, PlayerId(p)).known().collect::<Vec<_>>();
        assert_eq!(names(&base, 0)[0], "Lightning Bolt");
        assert!(known(&base, 0).contains(&(library(&base, 0)[0], true)), "Bot1 knows its top card's place");
        let a = start.game_for(1);
        let again = start.game_for(1);
        let b = start.game_for(2);
        assert_eq!((names(&a, 0), names(&a, 1)), (names(&again, 0), names(&again, 1)), "the same seed deals the same game");
        assert_ne!((names(&a, 0), names(&a, 1)), (names(&b, 0), names(&b, 1)), "another seed, other draws");
        for g in [&a, &b] {
            for p in 0..2u8 {
                let mut x = library(g, p);
                let mut y = library(&base, p);
                x.sort();
                y.sort();
                assert_eq!(x, y, "the same cards, in another order");
                assert_eq!(known(g, p), known(&base, p), "seat {p} knows what it knew");
                assert_eq!(g.state.zone(Zone::hand(PlayerId(p))), base.state.zone(Zone::hand(PlayerId(p))));
            }
            assert_eq!(names(g, 0)[0], "Lightning Bolt", "the card Bot1 put on top stays there");
            assert_eq!(g.pending().map(|r| r.player), base.pending().map(|r| r.player));
        }
        assert_ne!(names(&a, 1), names(&base, 1), "the unseen library is dealt anew");
        // A script that keeps its tops.
        let kept = start_of("kept-top", &format!("# selfplay: keep-top 5\n{KNOWN_TOP}"));
        assert_eq!(kept.keep_top, 5);
        let k = kept.game_for(3);
        for p in 0..2u8 {
            assert_eq!(names(&k, p)[..5], names(&base, p)[..5]);
        }
        assert_ne!(names(&k, 1), names(&base, 1));
        let whole = start_of("kept-library", &format!("# selfplay: keep-library\n{KNOWN_TOP}"));
        assert_eq!(names(&whole.game_for(3), 1), names(&base, 1));
    });
}
