//! The search's memory bound (`search::Config::memory`), with the counting allocator installed as the AI host installs
//! it: a search stops once its tree holds the bound, as at its deadline.

use std::path::Path;
use std::time::{Duration, Instant};

use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::eval::{Evaluator, FeatureBatch, Outputs};
use cantrip_ai::record::table;
use cantrip_ai::search::{self, Config, WallClock};
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
fn a_search_stops_at_its_memory_bound() {
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
        ..Config::default()
    };
    let search = |config: &Config| {
        search::search(&game, &request, &k, InfoMode::Decklist, table(), &Uniform, config, &prior, &CancelGuard::default(), &WallClock)
            .expect("a search answers")
            .search
            .expect("searched")
    };
    let s = search(&config);
    assert!(s.counted, "the counting allocator is installed");
    assert!(s.memory_bound, "{s:#?}");
    assert!(s.simulations > 0 && s.simulations < 1_000_000, "{s:#?}");
    // It stops at the first check past the bound: within one simulation's growth of it.
    assert!(s.tree_bytes < 2 * bound, "the tree went well past its bound: {} bytes", s.tree_bytes);
    // Without a bound the same search runs on past it.
    let u = search(&Config { simulations: Some(s.simulations * 4), memory: None, ..config.clone() });
    assert!(!u.memory_bound && u.simulations == s.simulations * 4 && u.tree_bytes > bound, "{u:#?}");
}
