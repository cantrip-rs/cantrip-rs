//! The search's memory bound (`search::Config::memory`), with the counting allocator installed as the AI host installs
//! it: a search stops once its tree holds the bound, as at its deadline, and a kept tree holding more than half of it is
//! let go rather than searched on.

use std::path::Path;
use std::time::{Duration, Instant};

use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::eval::{Evaluator, FeatureBatch, Outputs};
use cantrip_ai::record::table;
use cantrip_ai::search::{self, Config, Kept, Reuse, WallClock};
use cantrip_ai::{EvalError, Knowledge, Prior};
use cantrip_obs::InfoMode;
use cantrip_script::GameScript;

#[global_allocator]
static ALLOCATOR: cantrip_ai::alloc::Counting = cantrip_ai::alloc::Counting;

/// Every option alike, every position even.
struct Uniform;

impl Evaluator for Uniform {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        Ok(Outputs {
            value: vec![0.0; b.rows],
            policy: b.options.iter().map(|&(_, n)| vec![0.0; n]).collect(),
            aux: vec![[0.0; 2]; b.rows],
        })
    }
}

/// Bot1 in its third turn's main phase with a land, a Bolt and a creature to come; Bot2 an Elves deck.
const POSITION: &str = "\
[Bot1 Deck]
Mountain
Lightning Bolt
Mountain
Goblin Welder
Mountain x14

[Bot2 Deck]
Llanowar Elves x4
Forest x14

[Actions]
Bot1.Hand.Mountain.Play
[T3 Bot1.Main1]
[Handover Bot1.Pass]

[Expect]
Turn = 3
";

#[test]
fn a_search_stops_at_its_memory_bound_and_a_kept_tree_past_half_of_it_is_let_go() {
    let script = GameScript::parse(POSITION, "memory").expect("the script parses");
    let result = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}\n{}", f.message, f.report));
    let game = result.game;
    let request = game.pending().cloned().expect("a request is outstanding");
    let k = Knowledge::of(&game, request.player);
    let prior = Prior::from_decks(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decks")).expect("the decklists");
    let bound = 4 << 20;
    let config = Config {
        simulations: Some(1_000_000),
        deadline: Some(Instant::now() + Duration::from_secs(600)),
        memory: Some(bound),
        reuse: Reuse::Add,
        ..Config::default()
    };
    let mut kept: Option<Kept> = None;
    let search = |kept: &mut Option<Kept>| {
        search::search_keeping(&game, &request, &k, InfoMode::Decklist, table(), &Uniform, &config, &prior, &CancelGuard::default(), &WallClock, kept)
            .expect("a search answers")
            .search
            .expect("searched")
    };
    let s = search(&mut kept);
    assert!(s.counted, "the counting allocator is installed");
    assert!(s.memory_bound && !s.kept_dropped, "{s:#?}");
    assert!(s.simulations > 0 && s.simulations < 1_000_000, "{s:#?}");
    // It stops at the first check past the bound: within one simulation's growth of it.
    assert!(s.tree_bytes < 2 * bound, "the tree went well past its bound: {} bytes", s.tree_bytes);
    // The tree it keeps holds more than half the bound: the next search lets it go and begins afresh.
    assert!(kept.is_some());
    let again = search(&mut kept);
    assert!(again.kept_dropped && again.reused.is_none(), "{again:#?}");
    assert!(again.memory_bound && again.simulations > 0, "{again:#?}");
    // Without a bound the same search runs on past it.
    let unbounded = Config { simulations: Some(s.simulations * 4), memory: None, reuse: Reuse::None, ..config.clone() };
    let d = search::search(&game, &request, &k, InfoMode::Decklist, table(), &Uniform, &unbounded, &prior, &CancelGuard::default(), &WallClock)
        .expect("a search answers");
    let u = d.search.expect("searched");
    assert!(!u.memory_bound && u.simulations == s.simulations * 4 && u.tree_bytes > bound, "{u:#?}");
}
