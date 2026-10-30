//! The information-set search (`cantrip_ai::issearch`), on the tiny model under `tests/data/tiny` and on a fake network
//! (uniform priors, a fixed value) where a test needs to know what the network says:
//!
//! - with nothing from the network to go on, it finds a win in one, and its answer is the engine's to accept;
//! - two games the seat cannot tell apart (they differ only in the opponent's hidden hand and library) give it the
//!   same answer, root strategy and root value: what it reads of the game is the seat's to know.

use std::path::{Path, PathBuf};

use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::eval::{Evaluator, FeatureBatch, Outputs, TractEvaluator};
use cantrip_ai::issearch::{self, Config};
use cantrip_ai::record::table;
use cantrip_ai::{EvalError, Knowledge};
use cantrip_engine::*;
use cantrip_script::GameScript;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny")
}

/// Every option alike, and the same value for whoever is asked.
struct Uniform(f32);

impl Evaluator for Uniform {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        Ok(Outputs { value: vec![self.0; b.rows], policy: b.options.iter().map(|&(_, n)| vec![0.0; n]).collect(), aux: vec![[0.0; 2]; b.rows] })
    }
}

fn handed(text: &str, name: &str) -> (Game, Request) {
    let script = GameScript::parse(text, name).expect("the script parses");
    let result = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{name}: {}\n{}", f.message, f.report));
    assert!(result.handed_over, "{name}: handed over");
    let game = result.game;
    let request = game.pending().cloned().expect("a request is outstanding");
    (game, request)
}

/// Bot1 holds a Lightning Bolt and a Mountain to cast it with; Bot2 is at 3 life.
const WIN_IN_ONE: &str = "\
[Bot1 Deck]
Mountain
Lightning Bolt
Mountain x16

[Bot2 Deck]
Island x18

[Life]
Bot2: 3

[Actions]
Bot1.Hand.Mountain.Play
Bot2.Hand.Island.Play
[T3 Bot1.Main1]
[Handover Bot1.Pass]

[Expect]
Turn = 3
";

/// Bot1 in its third turn's main phase; Bot2's hidden hand and library are `bot2`, in some order.
fn unseen(bot2: &str) -> String {
    format!(
        "\
[Bot1 Deck]
Mountain
Lightning Bolt
Mountain
Goblin Welder
Mountain x14

[Bot2 Deck]
{bot2}

[Actions]
Bot1.Hand.Mountain.Play
[T3 Bot1.Main1]
[Handover Bot1.Pass]

[Expect]
Turn = 3
"
    )
}

fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(256 << 20).spawn(f).unwrap().join().unwrap()
}

#[test]
fn a_win_in_one_is_found_with_nothing_from_the_network() {
    on_big_stack(|| {
        let (game, request) = handed(WIN_IN_ONE, "win in one");
        let k = Knowledge::of(&game, request.player);
        let config = Config { worlds: 8, hands: 2, evaluations: 256, ..Config::default() };
        let (d, s) = issearch::search(&game, &request, &k, table(), &Uniform(0.0), &config, &CancelGuard::default()).unwrap();
        assert!(s.root_value > 0.3, "the root is a win for the seat: {s:#?}");
        assert!(check_answer(&game, &request, &d.answer).is_ok());
        // Played on, every answer of Bot1's searched, the game is Bot1's at once (the Bolt, or the land and then the
        // Bolt: both win this turn).
        let mut game = game;
        let mut answers = 0;
        for _ in 0..200 {
            let Some(r) = game.pending().cloned().filter(|_| !game.is_over()) else { break };
            let k = Knowledge::of(&game, r.player);
            let answer = if r.player == PlayerId(0) {
                answers += 1;
                issearch::search(&game, &r, &k, table(), &Uniform(0.0), &config, &CancelGuard::default()).unwrap().0.answer
            } else {
                cantrip_ai::net::answer(&game, &r, &k, cantrip_obs::InfoMode::Decklist, table(), &Uniform(0.0), cantrip_ai::net::Choice::Best).unwrap().answer
            };
            game.answer(r.id, r.player, answer).expect("the answer is taken");
        }
        assert_eq!(game.state.outcome, Outcome::Won(PlayerId(0)));
        assert!(answers <= 6, "won in {answers} answers");
        assert_eq!(s.own_worlds * 2, s.worlds, "half the worlds keep the real hand");
    });
}

#[test]
fn games_a_seat_cannot_tell_apart_are_searched_alike() {
    on_big_stack(|| {
        let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
        let (a, ra) = handed(&unseen("Llanowar Elves x4\nForest x14"), "unseen A");
        let (b, rb) = handed(&unseen("Forest x14\nLlanowar Elves x4"), "unseen B");
        let hand = |g: &Game| g.state.zone(Zone::hand(PlayerId(1))).iter().map(|&id| g.cards.get(g.state.obj(id).card).name).collect::<Vec<_>>();
        assert_ne!(hand(&a), hand(&b));
        let config = Config { worlds: 8, hands: 2, evaluations: 128, ..Config::default() };
        let run = |g: &Game, r: &Request| issearch::search(g, r, &Knowledge::of(g, r.player), table(), &evaluator, &config, &CancelGuard::default()).unwrap();
        let (da, sa) = run(&a, &ra);
        let (db, sb) = run(&b, &rb);
        assert_eq!(format!("{:?}", da.answer), format!("{:?}", db.answer));
        assert_eq!(sa.root, sb.root);
        assert_eq!(sa.root_value, sb.root_value);
        assert_eq!((sa.nodes, sa.infosets, sa.evaluations), (sb.nodes, sb.infosets, sb.evaluations));
    });
}
