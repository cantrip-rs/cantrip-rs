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
        let (mut spread, mut rooted) = (0, 0);
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
        }
        for &(decision, value) in &game.meta.root_values {
            assert!((-1.0..=1.0).contains(&value), "root value {value}");
            rooted += game.samples.iter().any(|s| s.decision == decision) as usize;
        }
        assert!(spread > 0, "no searched sub-step spread its visits");
        assert!(rooted > 0, "no root value names a recorded decision");
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
