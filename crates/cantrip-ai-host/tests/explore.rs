//! Exploration tooling (`cantrip_ai_host::explore`) with the tiny model: the search's floor on a nominated option
//! (`search::Config::root_favour`) reaches the root's priors and nothing else, a nominated action is verified world by
//! world with an interval, and the shortcut audit runs over traced self-play games.

use std::path::{Path, PathBuf};
use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::net::Payment;
use cantrip_ai::search::{self, Config, WallClock};
use cantrip_ai::{Evaluator, Knowledge, Prior, TractEvaluator, record::table};
use cantrip_ai_host::explore::{self, AuditOptions, VerifyOptions};
use cantrip_ai_host::selfplay::{self, Pairing, Run, Start};
use cantrip_obs::InfoMode;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../cantrip-ai/tests/data/tiny")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(cantrip_ai_host::STACK_SIZE).spawn(f).unwrap().join().unwrap()
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

fn bolts() -> Start {
    // A directory of its own each call: the tests run at once, and one removing a shared directory raced another
    // writing its scenario into it ("the game is over at its position").
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("cantrip-explore-{}-{call}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bolts.cantrip");
    std::fs::write(&path, BOLTS).unwrap();
    let start = Start::load(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    start
}

#[test]
fn the_floor_on_one_option_is_put_on_the_root_alone() {
    on_big_stack(|| {
        let start = bolts();
        let game = start.game.lock().unwrap().clone();
        let tiny = TractEvaluator::load(&tiny(), table()).unwrap();
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let request = game.pending().unwrap().clone();
        let k = Knowledge::of(&game, request.player);
        let r = explore::root(&game, &Payment::default(), &k, InfoMode::Decklist).unwrap();
        assert!(r.legal.len() >= 2, "a choice at the root");
        let x = r.tokens[*r.legal.last().unwrap()];
        let run = |favour: Option<(cantrip_obs::factor::Token, f32)>| {
            let config = Config { worlds: 2, batch: 4, shortcut: 0.0, root_favour: favour, ..Config::simulations(16) };
            let d = search::search(&game, &r.request, &k, InfoMode::Decklist, table(), &tiny, &config, &prior, &CancelGuard::default(), &WallClock).unwrap();
            d.search.unwrap().root_options
        };
        let plain = run(None);
        let floored = run(Some((x, 0.5)));
        assert_eq!(plain.len(), floored.len());
        let through = |o: &search::RootOption| o.path.first() == Some(&x);
        assert!(plain.iter().all(|o| o.prior == o.net_prior), "no floor, the network's priors");
        for (a, b) in plain.iter().zip(&floored) {
            assert_eq!(a.path, b.path);
            assert_eq!(a.net_prior, b.net_prior, "the network's own prior is kept");
        }
        let on_x: f32 = floored.iter().filter(|o| through(o)).map(|o| o.prior).sum();
        assert!(on_x >= 0.5 - 1e-5, "X has at least the floor: {on_x}");
        assert!((floored.iter().map(|o| o.prior).sum::<f32>() - plain.iter().map(|o| o.prior).sum::<f32>()).abs() < 1e-4);
        for (a, b) in plain.iter().zip(&floored).filter(|(a, _)| !through(a)) {
            assert!((b.prior - 0.5 * a.prior).abs() < 1e-6, "the others keep half: {} {}", a.prior, b.prior);
        }
    });
}

#[test]
fn a_nominated_action_is_verified_world_by_world() {
    on_big_stack(|| {
        let start = bolts();
        let game = start.game.lock().unwrap().clone();
        let tiny = TractEvaluator::load(&tiny(), table()).unwrap();
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let o = VerifyOptions { worlds: 4, simulations: 64, batch: 4, ..VerifyOptions::default() };
        let v = explore::verify(&game, "lightning bolt", &tiny, &prior, &o).unwrap();
        assert!(v["Action"].as_str().unwrap().contains("Lightning Bolt"), "{v}");
        assert!(v["ActionSearchPrior"].as_f64().unwrap() >= o.floor as f64 - 1e-5, "the floor reached the root: {v}");
        assert_eq!(v["PerWorld"].as_array().unwrap().len(), 4);
        let paired = v["Paired"].as_u64().unwrap();
        assert!(paired <= 4);
        if paired >= 2 {
            let (m, lo, hi) = (v["QDifference"]["Mean"].as_f64().unwrap(), v["QDifference"]["Low"].as_f64().unwrap(), v["QDifference"]["High"].as_f64().unwrap());
            assert!(lo <= m && m <= hi, "{v}");
        }
        assert!(v["Options"].as_array().unwrap().len() >= 2);
        // The same seed, the same verdict.
        assert_eq!(explore::verify(&game, "lightning bolt", &tiny, &prior, &o).unwrap(), v);
        assert!(explore::verify(&game, "no such play", &tiny, &prior, &o).is_err());
        assert_eq!(explore::verify(&game, "#0", &tiny, &prior, &o).unwrap()["Options"], v["Options"]);
    });
}

#[test]
fn the_shortcut_audit_runs_over_traced_games() {
    on_big_stack(|| {
        let base = std::env::temp_dir().join(format!("cantrip-audit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let prior = Prior::from_decks(&root().join("decks")).unwrap();
        let deck = |n: &str| selfplay::load_deck(&root().join("decks").join(format!("{n}.txt"))).unwrap();
        let options = selfplay::Options { model: "tiny".into(), simulations: 8, sampled: 4, max_turns: 4, traces: true, ..selfplay::Options::default() };
        let run = Run {
            out: base.join("selfplay"),
            games: 2,
            limit: None,
            lanes: 2,
            seed: 61,
            pairings: vec![Pairing { names: ["rw-aggro".into(), "ub-reanimator".into()], lists: [deck("rw-aggro"), deck("ub-reanimator")] }],
            producer: "selfplay:tiny@8".into(),
            shard_mb: 64,
            shard_games: 100,
            resume: false,
            govern: Default::default(),
            grant: None,
            host: None,
        };
        let load = || Ok(Box::new(TractEvaluator::load(&tiny(), table()).map_err(|e| e.to_string())?) as Box<dyn Evaluator>);
        selfplay::run(&run, &options, &prior, load).unwrap();
        let traces = cantrip_ai_host::regret::load_traces(&base.join("selfplay/traces")).unwrap().into_iter().collect();
        // The tiny model is far from sure of anything: a wide bound finds positions a shortcut would pass.
        let o = AuditOptions { positions: 3, simulations: 32, worlds: 2, batch: 4, mode: InfoMode::Decklist, shortcut: 0.9, min_visits: 1, ..AuditOptions::default() };
        let out = base.join("audit.jsonl");
        let summary = explore::audit(&traces, None, &o, &prior, 2, &out, load).unwrap();
        assert!(summary["Errors"].as_object().unwrap().is_empty(), "{summary}");
        let lines: Vec<serde_json::Value> = std::fs::read_to_string(&out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(summary["Audited"], lines.len(), "{summary}");
        assert_eq!(lines.len(), 3, "{summary}");
        for l in &lines {
            assert_eq!(l["floored"], true, "{l}");
            assert!(l["pass"].is_object(), "a shortcut position offers a pass: {l}");
        }
        eprintln!("audit: {summary}");
        let _ = std::fs::remove_dir_all(&base);
    });
}
