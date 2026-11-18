//! The search (`cantrip_ai::search`), on the tiny model under `tests/data/tiny` and on a fake network (uniform priors,
//! a fixed value) where a test needs to know what the network says:
//!
//! - its answers over a sample of the corpus's requests are the engine validator's to accept, in every mode;
//! - with nothing from the network to go on, it finds a win in one (the opponent's choices on the way are its own,
//!   their values backed up from its side), and knows a lost position for lost;
//! - a budget of simulations is spent exactly, and a deadline is read from the clock it is given, between the requests
//!   a line plays with no choice too, so the search stops at it whatever a line costs;
//! - the same position, seed and budget give the same answer and visits;
//! - two games a seat cannot tell apart give it, in W worlds, the same answer and visits (Decklist and Hidden);
//! - branch trimming cuts a move that is clearly worse, keeps the best, and can be turned off.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::eval::{Evaluator, FeatureBatch, Outputs, TractEvaluator};
use cantrip_ai::net::Choice;
use cantrip_ai::record::table;
use cantrip_ai::search::{self, Clock, Config, WallClock};
use cantrip_ai::{EvalError, Knowledge, Prior};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::factor::Token;
use cantrip_script::GameScript;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn prior() -> &'static Prior {
    static PRIOR: std::sync::OnceLock<Prior> = std::sync::OnceLock::new();
    PRIOR.get_or_init(|| Prior::from_decks(&root().join("decks")).expect("the decklists"))
}

/// Every option alike, and the same value for whoever is asked.
struct Uniform(f32);

impl Evaluator for Uniform {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        Ok(Outputs {
            value: vec![self.0; b.rows],
            policy: b.options.iter().map(|&(_, n)| vec![0.0; n]).collect(),
            aux: vec![[0.0; 2]; b.rows],
        })
    }
}

/// A clock that moves a millisecond each time it is read.
struct Ticking {
    at: Cell<Instant>,
}

impl Clock for Ticking {
    fn now(&self) -> Instant {
        let t = self.at.get();
        self.at.set(t + Duration::from_millis(1));
        t
    }
}

/// The game a scenario's text hands over, with its request.
fn handed(text: &str, name: &str) -> (Game, Request) {
    let script = GameScript::parse(text, name).expect("the script parses");
    let result = cantrip_script::run(&script)
        .unwrap_or_else(|f| panic!("{name}: {}\n{}", f.message, f.report));
    assert!(result.handed_over, "{name}: handed over");
    let game = result.game;
    let request = game.pending().cloned().expect("a request is outstanding");
    (game, request)
}

fn run(
    game: &Game,
    request: &Request,
    mode: InfoMode,
    evaluator: &dyn Evaluator,
    config: &Config,
) -> cantrip_ai::net::Decision {
    let k = Knowledge::of(game, request.player);
    search::search(
        game,
        request,
        &k,
        mode,
        table(),
        evaluator,
        config,
        prior(),
        &CancelGuard::default(),
        &WallClock,
    )
    .expect("a search answers")
}

/// Bot1 holds a Lightning Bolt and a Mountain to cast it with; Bot2 is at 3 life with an Island untapped (its one
/// choice when the Bolt is cast: to tap it or not, both losing).
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

/// Bot1 has a Bolt and both players are at 3: the target choice is a win at Bot2 and a loss at Bot1.
const BOLT_EITHER_WAY: &str = "\
[Bot1 Deck]
Mountain
Lightning Bolt
Mountain x16

[Bot2 Deck]
Island x18

[Life]
Bot1: 3
Bot2: 3

[Actions]
Bot1.Hand.Mountain.Play
Bot2.Hand.Island.Play
[T3 Bot1.Main1]
[Handover Bot1.Pass]

[Expect]
Turn = 3
";

/// Bot2's Lightning Bolt is on its way to Bot1, at 3 life, who can only tap a Mountain or let it resolve.
const LOST: &str = "\
[Bot1 Deck]
Mountain x18

[Bot2 Deck]
Mountain
Lightning Bolt
Mountain x16

[Life]
Bot1: 3

[Actions]
Bot1.Hand.Mountain.Play
Bot2.Hand.Mountain.Play
Bot2.Battlefield.Mountain.Activate
Bot2.Hand.Lightning Bolt.Cast(Bot1)
[Handover Bot1.Pass]

[Expect]
Turn = 2
";

/// Bot1 in its third turn's main phase with the choices of a land, a Bolt and a creature to come; Bot2's hidden hand
/// and library are `bot2`, a deck of four Llanowar Elves and fourteen Forests in some order (it plays no land: the
/// card it draws is the eighth and is discarded at its end step, a Forest in either order below).
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

/// Plays Bot1 by search and Bot2 by the network alone until the game ends: Bot1's win, in how many of its answers.
fn play_out(text: &str, name: &str, sims: usize, evaluator: &dyn Evaluator) -> (Outcome, usize) {
    let (mut game, _) = handed(text, name);
    let mut answers = 0;
    for _ in 0..200 {
        if game.is_over() {
            break;
        }
        let Some(r) = game.pending().cloned() else {
            break;
        };
        let k = Knowledge::of(&game, r.player);
        let d = if r.player == PlayerId(0) {
            answers += 1;
            search::search(
                &game,
                &r,
                &k,
                InfoMode::Full,
                table(),
                evaluator,
                &Config::simulations(sims),
                prior(),
                &CancelGuard::default(),
                &WallClock,
            )
            .unwrap()
        } else {
            cantrip_ai::net::answer(
                &game,
                &r,
                &k,
                InfoMode::Full,
                table(),
                evaluator,
                Choice::Best,
            )
            .unwrap()
        };
        if std::env::var_os("SEARCH_TEST_DEBUG").is_some() {
            eprintln!("{:?} {:?} -> {:?}", r.player, cantrip_ai::kind_name(&r.kind), d.answer);
        }
        game.answer(r.id, r.player, d.answer)
            .expect("the answer is taken");
    }
    (game.state.outcome, answers)
}

#[test]
fn a_win_in_one_is_found_with_nothing_from_the_network() {
    let (game, request) = handed(WIN_IN_ONE, "win in one");
    let d = run(
        &game,
        &request,
        InfoMode::Full,
        &Uniform(0.0),
        &Config::simulations(200),
    );
    let s = d.search.as_ref().expect("searched");
    // The root's value is its visits' mean: the win, and the other options' share of the visits at 0.
    assert!(s.root_value > 0.3, "the root is a win for the seat: {s:#?}");
    match &d.answer {
        Answer::Act(a) => {
            assert_eq!(a.kind, ActionKind::CastSpell, "{s:#?}");
            assert_eq!(game.name(a.source.unwrap()), "Lightning Bolt");
        }
        other => panic!("the Bolt is cast, not {other:?}: {s:#?}"),
    }
    // And played on, every answer of Bot1's searched, the game is Bot1's at once: the Bolt, its target, its mana.
    let (outcome, answers) = play_out(WIN_IN_ONE, "win in one", 200, &Uniform(0.0));
    assert_eq!(outcome, Outcome::Won(PlayerId(0)));
    assert!(answers <= 4, "won in {answers} answers");
}

#[test]
fn a_guarded_action_is_not_offered_in_the_search() {
    let (game, request) = handed(WIN_IN_ONE, "guarded win");
    let mut guard = CancelGuard::default();
    guard.ban(&game, "Lightning Bolt");
    let k = Knowledge::of(&game, request.player);
    for mode in [InfoMode::Full, InfoMode::Decklist] {
        let d = search::search(
            &game,
            &request,
            &k,
            mode,
            table(),
            &Uniform(0.0),
            &Config::simulations(200),
            prior(),
            &guard,
            &WallClock,
        )
        .expect("a search answers");
        assert!(
            matches!(d.answer, Answer::Pass),
            "the guarded Bolt is not cast in {mode:?}: {d:#?}"
        );
    }
}

#[test]
fn a_very_bad_move_is_pruned_and_the_winning_move_kept() {
    let (game, request) = handed(BOLT_EITHER_WAY, "prune bad");
    // Searched in steps (the target a node of its own), where the losing target is descended again and cut.
    let d = run(
        &game,
        &request,
        InfoMode::Full,
        &Uniform(0.0),
        &Config { flatten: 0, ..Config::simulations(200) },
    );
    let s = d.search.as_ref().expect("searched");
    assert!(s.pruned > 0, "something was cut: {s:#?}");
    assert_eq!(
        s.pruned,
        s.pruned_immediate + s.pruned_shallow,
        "every cut is counted once: {s:#?}"
    );
    match &d.answer {
        Answer::Act(a) => {
            assert_eq!(a.kind, ActionKind::CastSpell, "{s:#?}");
            assert_eq!(game.name(a.source.unwrap()), "Lightning Bolt");
            assert_eq!(
                a.targets,
                [Target::Player(PlayerId(1))],
                "the Bolt is aimed at Bot2: {s:#?}"
            );
        }
        other => panic!("the Bolt is cast, not {other:?}: {s:#?}"),
    }
}

/// A priority searched as whole answers: the Bolt at each target its own option, the right one taken.
#[test]
fn a_spell_and_its_target_are_one_option() {
    let (game, request) = handed(BOLT_EITHER_WAY, "whole");
    let d = run(&game, &request, InfoMode::Full, &Uniform(0.0), &Config::simulations(200));
    let s = d.search.as_ref().expect("searched");
    assert!(s.flattened > 0, "the priority was searched as whole answers: {s:#?}");
    assert!(s.root_edges >= 3, "the Bolt at each player, and passing: {s:#?}");
    match &d.answer {
        Answer::Act(a) => {
            assert_eq!(game.name(a.source.unwrap()), "Lightning Bolt");
            assert_eq!(a.targets, [Target::Player(PlayerId(1))], "{s:#?}");
        }
        other => panic!("the Bolt is cast, not {other:?}: {s:#?}"),
    }
    assert_eq!(s.waste_leaves, 0, "nothing below a cancel or a mana ability: {s:#?}");
}

#[test]
fn branch_trimming_can_be_turned_off() {
    let (game, request) = handed(BOLT_EITHER_WAY, "no prune");
    let config = Config {
        simulations: Some(200),
        prune: false,
        ..Config::default()
    };
    let d = run(&game, &request, InfoMode::Full, &Uniform(0.0), &config);
    assert_eq!(d.search.as_ref().expect("searched").pruned, 0);
}

#[test]
fn a_lost_position_is_lost_from_the_seats_side() {
    let (game, request) = handed(LOST, "lost");
    assert_eq!(request.player, PlayerId(0));
    // Bot1's only option besides passing is tapping a Mountain for mana, which atomic play does not offer on its own:
    // searched as the factoring offers it.
    let d = run(
        &game,
        &request,
        InfoMode::Full,
        &Uniform(0.0),
        &Config { atomic: false, ..Config::simulations(64) },
    );
    let s = d.search.as_ref().expect("searched");
    assert!(s.simulations > 0, "{s:#?}");
    assert!(s.root_value < -0.9, "every line loses: {s:#?}");
}

#[test]
fn a_value_for_whoever_is_asked_backs_up_from_each_seats_side() {
    // A network that says every position is even for whoever is asked: what any line comes to is even, whichever
    // seat's leaves it ends in, so the root is even too (the win in one aside, whose terminal is +1 for the seat).
    let (game, request) = handed(&unseen("Llanowar Elves x4\nForest x14"), "even");
    let d = run(
        &game,
        &request,
        InfoMode::Full,
        &Uniform(0.25),
        &Config::simulations(400),
    );
    let s = d.search.as_ref().expect("searched");
    // A value of +0.25 for whoever is asked: a Bot2 leaf is -0.25 from Bot1's side, so the root lies between them,
    // and below +0.25 only if Bot2's leaves were reached and backed up negated.
    assert!(
        s.root_value < 0.25 - 1e-3 && s.root_value > -0.25 - 1e-3,
        "{s:#?}"
    );
}

#[test]
fn a_budget_of_simulations_is_spent_exactly() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let (game, request) = handed(WIN_IN_ONE, "budget");
    for n in [1, 7, 37] {
        let d = run(
            &game,
            &request,
            InfoMode::Full,
            &evaluator,
            &Config::simulations(n),
        );
        assert_eq!(d.search.as_ref().unwrap().simulations, n, "{n} simulations");
    }
    // No budget: the network alone.
    let d = run(
        &game,
        &request,
        InfoMode::Full,
        &evaluator,
        &Config::default(),
    );
    assert!(d.search.is_none());
}

#[test]
fn a_deadline_is_read_from_the_clock_given() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let (game, request) = handed(&unseen("Llanowar Elves x4\nForest x14"), "deadline");
    let start = Instant::now();
    let clock = Ticking {
        at: Cell::new(start),
    };
    // Read once to start, then before each batch, each simulation and each request a line plays with no choice: the
    // sixtieth read is past the deadline, and the search stops at it.
    let deadline = start + Duration::from_millis(60);
    let config = Config {
        deadline: Some(deadline),
        batch: 4,
        ..Config::default()
    };
    let k = Knowledge::of(&game, request.player);
    let d = search::search(
        &game,
        &request,
        &k,
        InfoMode::Full,
        table(),
        &evaluator,
        &config,
        prior(),
        &CancelGuard::default(),
        &clock,
    )
    .unwrap();
    let s = d.search.as_ref().unwrap();
    assert!(s.batches > 0 && s.simulations >= s.batches, "{s:#?}");
    assert!(
        clock.at.get() <= deadline + Duration::from_millis(3),
        "the search went on past its deadline: {:?}",
        clock.at.get() - deadline
    );
    // A deadline already past: no simulation, the network's answer, taken in time.
    let late = Config {
        deadline: Some(start),
        network_deadline: Some(Instant::now() + Duration::from_secs(600)),
        ..Config::default()
    };
    let d = search::search(
        &game,
        &request,
        &k,
        InfoMode::Full,
        table(),
        &evaluator,
        &late,
        prior(),
        &CancelGuard::default(),
        &Ticking {
            at: Cell::new(start),
        },
    )
    .unwrap();
    let s = d.search.as_ref().unwrap();
    assert_eq!(s.simulations, 0);
    assert!(!d.late && !d.fallback, "{d:?}");
    check_answer(&game, &request, &d.answer).expect("the network's answer is legal");
}

/// The network, uniform, counting its evaluations.
struct Counted {
    evaluations: AtomicUsize,
}

impl Evaluator for Counted {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        self.evaluations.fetch_add(1, Ordering::Relaxed);
        Uniform(0.0).evaluate(b)
    }
}

/// A clock that stands still until the network has been asked once, then passes the deadline at its `after`th read.
struct AfterEvaluation<'a> {
    at: Instant,
    evaluator: &'a Counted,
    reads: Cell<usize>,
    after: usize,
}

impl Clock for AfterEvaluation<'_> {
    fn now(&self) -> Instant {
        if self.evaluator.evaluations.load(Ordering::Relaxed) == 0 {
            return self.at;
        }
        self.reads.set(self.reads.get() + 1);
        if self.reads.get() >= self.after {
            self.at + Duration::from_secs(3600)
        } else {
            self.at
        }
    }
}

/// Bot1's end step with a Lightning Bolt and a Mountain to cast it with; Bot2 holds only Bolts and has no land. Cast or
/// not, the line runs on through Bot2's turn with no choice for anyone until Bot1's next main phase.
const FORCED_TURN: &str = "[Bot1 Deck]
Mountain
Lightning Bolt
Mountain x16

[Bot2 Deck]
Lightning Bolt x18

[Actions]
Bot1.Hand.Mountain.Play
[T3 Bot1.End]
[Handover Bot1.Pass]

[Expect]
Turn = 3
";

/// A line the search plays out stops between its requests once the deadline has passed (a line can run a turn or more
/// with no choice on the way, every request observed), and so does the batch it was being chosen for; the edge is left
/// unplayed.
#[test]
fn a_line_stops_between_its_forced_requests_at_the_deadline() {
    let (game, request) = handed(FORCED_TURN, "deadline in a line");
    let k = Knowledge::of(&game, request.player);
    let searched = |config: &Config, evaluator: &Counted, clock: &dyn Clock| {
        search::search(&game, &request, &k, InfoMode::Full, table(), evaluator, config, prior(), &CancelGuard::default(), clock)
            .expect("a search answers")
    };
    // Played out, a line from the root passes requests with no choice: the forks' game trees grow by them.
    let free = Counted { evaluations: AtomicUsize::new(0) };
    let d = searched(&Config { simulations: Some(2), batch: 1, ..Config::default() }, &free, &WallClock);
    let played = d.search.as_ref().unwrap().game_nodes;
    assert!(played > 2, "a line from the root plays requests with no choice: {:#?}", d.search);
    // The deadline passes at the third read after the root's evaluation: the read before the next batch, the read before
    // its first simulation, and then the first request the line plays with no choice.
    let start = Instant::now();
    let evaluator = Counted { evaluations: AtomicUsize::new(0) };
    let clock = AfterEvaluation { at: start, evaluator: &evaluator, reads: Cell::new(0), after: 3 };
    let config = Config { deadline: Some(start + Duration::from_secs(1)), batch: 1, ..Config::default() };
    let d = searched(&config, &evaluator, &clock);
    let s = d.search.as_ref().unwrap();
    // The root alone was evaluated by the search; with nothing visited below it, the network answers.
    assert_eq!((s.evaluations, s.simulations), (1, 1), "{s:#?}");
    assert_eq!(evaluator.evaluations.load(Ordering::Relaxed), 2, "{s:#?}");
    assert!(s.game_nodes < played && s.game_nodes <= 2, "the line stopped at its first request with no choice: {s:#?}");
    check_answer(&game, &request, &d.answer).expect("the answer is legal");
}

#[test]
fn the_same_position_seed_and_budget_give_the_same_answer() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let (game, request) = handed(&unseen("Llanowar Elves x4\nForest x14"), "determinism");
    for mode in [InfoMode::Full, InfoMode::Decklist] {
        let config = Config {
            simulations: Some(48),
            worlds: 3,
            ..Config::default()
        };
        let a = run(&game, &request, mode, &evaluator, &config);
        let b = run(&game, &request, mode, &evaluator, &config);
        assert_eq!(
            format!("{:?}", a.answer),
            format!("{:?}", b.answer),
            "{mode:?}"
        );
        let (sa, sb) = (a.search.unwrap(), b.search.unwrap());
        assert_eq!(sa.visits, sb.visits, "{mode:?}");
        assert_eq!(
            (sa.simulations, sa.nodes, sa.positions),
            (sb.simulations, sb.nodes, sb.positions),
            "{mode:?}"
        );
    }
}

#[test]
fn games_a_seat_cannot_tell_apart_are_searched_alike() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let (a, ra) = handed(&unseen("Llanowar Elves x4\nForest x14"), "unseen A");
    let (b, rb) = handed(&unseen("Forest x14\nLlanowar Elves x4"), "unseen B");
    // The games differ where Bot1 cannot see.
    let hand = |g: &Game| {
        g.state
            .zone(Zone::hand(PlayerId(1)))
            .iter()
            .map(|&id| g.cards.get(g.state.obj(id).card).name)
            .collect::<Vec<_>>()
    };
    assert_ne!(hand(&a), hand(&b));
    for mode in [InfoMode::Decklist, InfoMode::Hidden] {
        let config = Config {
            simulations: Some(64),
            worlds: 2,
            ..Config::default()
        };
        let da = run(&a, &ra, mode, &evaluator, &config);
        let db = run(&b, &rb, mode, &evaluator, &config);
        assert_eq!(
            format!("{:?}", da.answer),
            format!("{:?}", db.answer),
            "{mode:?}"
        );
        let (sa, sb) = (da.search.unwrap(), db.search.unwrap());
        assert_eq!(sa.worlds, 2);
        assert_eq!(sa.visits, sb.visits, "{mode:?}");
        assert_eq!(
            (sa.simulations, sa.nodes, sa.positions),
            (sb.simulations, sb.nodes, sb.positions),
            "{mode:?}"
        );
        assert_eq!(sa.root_value, sb.root_value, "{mode:?}");
    }
    // In Full mode the seat sees them apart, and the search may too (not asserted: only that it runs).
    let _ = run(
        &a,
        &ra,
        InfoMode::Full,
        &evaluator,
        &Config::simulations(16),
    );
}

#[derive(Default, Debug)]
struct Tally {
    requests: usize,
    searched: usize,
    refused: Vec<String>,
}

fn corpus_play(path: &Path, evaluator: &dyn Evaluator, tally: &Mutex<Tally>) {
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let Ok(script) = GameScript::load(path) else {
        return;
    };
    let Ok(result) = cantrip_script::run(&script) else {
        return;
    };
    let mut game = result.game;
    let nodes = game.tree.path(game.at);
    let answers: Vec<_> = nodes
        .iter()
        .filter_map(|&n| game.tree.answer(n).map(|(_, p, _)| (n, *p)))
        .collect();
    for (k, (node, player)) in answers.iter().enumerate().step_by(3) {
        let Some(parent) = game.tree.parent(*node) else {
            continue;
        };
        game.goto(parent);
        let Some(request) = game.pending().cloned() else {
            continue;
        };
        if request.player != *player {
            continue;
        }
        let knowledge = Knowledge::of(&game, *player);
        let mode = [InfoMode::Full, InfoMode::Decklist, InfoMode::Hidden][(k / 3) % 3];
        let choice = if k % 2 == 0 {
            Choice::Best
        } else {
            Choice::Sample {
                temperature: 1.0,
                seed: k as u64 + 1,
            }
        };
        let config = Config {
            simulations: Some(12),
            worlds: 2,
            batch: 4,
            choice,
            seed: k as u64,
            ..Config::default()
        };
        let made = search::search(
            &game,
            &request,
            &knowledge,
            mode,
            table(),
            evaluator,
            &config,
            prior(),
            &CancelGuard::default(),
            &WallClock,
        );
        let mut t = tally.lock().unwrap();
        t.requests += 1;
        match made {
            Ok(d) => {
                if let Err(r) = check_answer(&game, &request, &d.answer) {
                    t.refused.push(format!(
                        "{name} answer {k} {mode:?}: {:?} refused: {}",
                        d.answer, r.0
                    ));
                    continue;
                }
                assert!((-1.0..=1.0).contains(&d.value), "{name}: value {}", d.value);
                t.searched += d.search.as_ref().is_some_and(|s| s.simulations > 0) as usize;
            }
            Err(e) => t.refused.push(format!(
                "{name} answer {k} {mode:?} ({:?}): {e}",
                request.kind
            )),
        }
    }
}

/// The cards merged with a permanent (CR 730.2) have no row of their own, but they are in the game, so a world's deal of the
/// seat's unseen cards (checked exactly against its deck less what it has seen under `CANTRIP_VERIFY`) must count them.
#[test]
fn a_mutated_permanents_merged_cards_are_accounted_for_in_every_world() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let tally = Mutex::new(Tally::default());
    corpus_play(&root().join("scenarios/snapdax-apex-of-the-hunt-mutate.cantrip"), &evaluator, &tally);
    let t = tally.into_inner().unwrap();
    assert!(t.refused.is_empty(), "{:#?}", t.refused);
}

#[test]
fn searched_answers_are_accepted() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let mut sample: Vec<PathBuf> = std::fs::read_dir(root().join("scenarios"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "cantrip"))
        .collect();
    sample.sort();
    let sample: Vec<PathBuf> = sample.into_iter().step_by(90).collect();
    let tally = Mutex::new(Tally::default());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            let (sample, tally, next, evaluator) = (&sample, &tally, &next, &evaluator);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(s, move || {
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(path) = sample.get(i) else { break };
                        corpus_play(path, evaluator, tally);
                    }
                })
                .unwrap();
        }
    });
    let t = tally.into_inner().unwrap();
    println!(
        "{} scenarios, {} requests, {} searched, {} refused",
        sample.len(),
        t.requests,
        t.searched,
        t.refused.len()
    );
    for r in t.refused.iter().take(20) {
        println!("  {r}");
    }
    assert!(t.requests > 100 && t.searched > 30, "{t:?}");
    assert!(t.refused.is_empty(), "{:#?}", t.refused);
}

/// A deck file's main deck, a card a line ("4 Name").
fn deck(name: &str) -> Vec<String> {
    let text = std::fs::read_to_string(root().join("decks/standard").join(name)).expect("the decklist");
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.eq_ignore_ascii_case("sideboard") || line.eq_ignore_ascii_case("companion") {
            break;
        }
        match line.split_once(' ') {
            Some((n, card)) if n.parse::<usize>().is_ok() => out.extend(std::iter::repeat_n(card.to_string(), n.parse().unwrap())),
            _ => out.push(line.to_string()),
        }
    }
    out
}

/// Every world a search deals shows the seat the game it is in: over whole games of both seats searching afresh at each
/// request (no tree kept, so every world is dealt fresh), mulligans included (the tiny network mulligans often: hands
/// shuffled back, cards bottomed), each world's root observation is the real one (`Config::reuse_check` on fresh
/// worlds). A hand card a mulligan shuffled back, or a revealed card put into a library, is that card in every world; a
/// library the seat is searching keeps its order; and a run that drew on the generator before its request (a mulligan's
/// shuffle and redraw) is answered without worlds, since every world would deal it again.
#[test]
fn every_world_dealt_shows_the_seat_the_game_it_is_in() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let decks = [deck("dimir-midrange.txt"), deck("izzet-spellementals.txt")];
    let (mut searched, mut checked, mut mismatched) = (0usize, 0usize, 0usize);
    // Games until enough worlds are checked: how soon a game ends hangs on the decks and the tiny network's mulligans.
    for seed in 0..12u64 {
        if searched > 20 && checked > 100 {
            break;
        }
        let mut game = deal(&[decks[0].clone(), decks[1].clone()], 77_000 + seed, true, None).unwrap();
        let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
        let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
        for _ in 0..1500 {
            if game.is_over() || game.state.turn.number > 12 {
                break;
            }
            let Some(request) = game.pending().cloned() else { break };
            let p = request.player.index();
            ks[p].update(&game);
            let config = Config {
                worlds: 4,
                seed: request.id as u64,
                reuse: search::Reuse::None,
                reuse_check: true,
                ..Config::simulations(16)
            };
            let k = &ks[p];
            let d = payments[p]
                .decide(&game, &request, k, InfoMode::Decklist, |request, guard| {
                    search::search(&game, request, k, InfoMode::Decklist, table(), &evaluator, &config, prior(), guard, &WallClock)
                })
                .unwrap();
            if let Some(s) = d.search.as_ref().filter(|s| s.fresh_checked > 0) {
                assert!(s.reused.is_none(), "no tree is kept: {s:#?}");
                searched += 1;
                checked += s.fresh_checked;
                mismatched += s.fresh_mismatched;
            }
            let board = cantrip_ai::cancel::board_hash(&game.state);
            payments[p].played(&game, board, &request, &d.answer);
            game.answer(request.id, request.player, d.answer).expect("the answer is taken");
        }
    }
    println!("{searched} searches, {checked} worlds dealt, {mismatched} unlike the game");
    assert!(searched > 20 && checked > 100, "too few worlds dealt to check: {searched} searches, {checked} worlds");
    assert_eq!(mismatched, 0, "worlds the seat could tell from its game, of {checked} over {searched} searches");
}

/// Tree reuse is checked, not assumed (`search::ReuseCheck`): over whole games of both seats searching with their trees
/// kept (atomic, `Reuse::Add`, every answer followed, as self-play plays), each re-rooted world's root is built fresh in
/// it and must be the same - the request, the seat's knowledge and guard, no payment run, every other seat's knowledge
/// and guard, the observation, the network's input, the options, their priors and the value - and no cut at the root
/// may outlive its trim. And every world dealt fresh shows the seat the game it is in: its root observation is the real
/// one, mulligans included (the tiny network mulligans often: hands shuffled back, cards bottomed - to an empty hand at
/// the cap, RX-6, which ends a game early, so games are played until there are enough searches to check).
#[test]
fn a_kept_tree_is_the_tree_a_fresh_search_would_grow() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let decks = [deck("dimir-midrange.txt"), deck("izzet-spellementals.txt")];
    let (mut rerooted, mut checked, mut fresh, mut fresh_mismatched) = (0usize, 0usize, 0usize, 0usize);
    let mut mismatches = std::collections::BTreeMap::<String, usize>::new();
    for seed in 0..12u64 {
        if rerooted > 20 && checked > 50 && fresh > 100 {
            break;
        }
        let mut game = deal(&[decks[0].clone(), decks[1].clone()], 77_000 + seed, true, None).unwrap();
        let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
        let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
        let mut kept: Vec<Option<search::Kept>> = vec![None, None];
        for _ in 0..1500 {
            if game.is_over() || game.state.turn.number > 12 {
                break;
            }
            let Some(request) = game.pending().cloned() else { break };
            let p = request.player.index();
            ks[p].update(&game);
            let config = Config {
                worlds: 4,
                seed: request.id as u64,
                reuse: search::Reuse::Add,
                reuse_follow: search::Follow::All,
                reuse_check: true,
                ..Config::simulations(96)
            };
            let k = &ks[p];
            let keep = &mut kept[p];
            let d = payments[p]
                .decide(&game, &request, k, InfoMode::Decklist, |request, guard| {
                    search::search_keeping(&game, request, k, InfoMode::Decklist, table(), &evaluator, &config, prior(), guard, &WallClock, keep)
                })
                .unwrap();
            if let Some(s) = d.search.as_ref() {
                fresh += s.fresh_checked;
                fresh_mismatched += s.fresh_mismatched;
            }
            if let Some(r) = d.search.as_ref().and_then(|s| s.reused.as_ref()) {
                rerooted += 1;
                let c = r.check.as_ref().expect("the check ran");
                checked += c.worlds;
                for (what, n) in &c.mismatches {
                    *mismatches.entry(what.clone()).or_default() += n;
                }
            }
            let board = cantrip_ai::cancel::board_hash(&game.state);
            payments[p].played(&game, board, &request, &d.answer);
            game.answer(request.id, request.player, d.answer).expect("the answer is taken");
        }
    }
    assert!(rerooted > 20 && checked > 50, "too few trees re-rooted to check: {rerooted} searches, {checked} worlds");
    assert!(mismatches.is_empty(), "kept trees differ from fresh ones: {mismatches:?} over {checked} worlds");
    assert!(fresh > 100, "too few fresh worlds dealt to check: {fresh}");
    assert_eq!(fresh_mismatched, 0, "fresh worlds the seat could tell from its game, of {fresh}");
}


/// Bot1 at its fourth main phase (turn 7) with Island, Island, Mountain, Mountain in play and Lightning Strike ({1}{R}), Shock
/// ({R}) and blue cards in hand.
const STRIKE_THEN_SHOCK: &str = "\
[Bot1 Deck]
Island
Mountain
Island
Mountain
Lightning Strike
Shock
Counterspell
Negate
Opt
Island x12

[Bot2 Deck]
Island x18

[Actions]
Bot1.Hand.Island.Play
[T3 Bot1.Main1]
Bot1.Hand.Mountain.Play
[T5 Bot1.Main1]
Bot1.Hand.Island.Play
[T7 Bot1.Main1]
Bot1.Hand.Mountain.Play
[Handover Bot1.Pass]

[Expect]
Turn = 7
";

/// Casts Lightning Strike at Bot2 with `follow` as the plan (`pay::FollowUps`), the old autotapper if `classic`: the
/// Mountains left untapped.
fn strike_leaves(follow: &[(ManaCost, f32)], classic: bool) -> usize {
    let (mut game, request) = handed(STRIKE_THEN_SHOCK, "strike then shock");
    let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
    let strike = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == "Lightning Strike"))
        .expect("Lightning Strike is castable");
    let mut cast = Action::of(strike);
    cast.targets = vec![Target::Player(PlayerId(1))];
    game.answer(request.id, request.player, Answer::Act(cast)).expect("the cast is taken");
    let me = PlayerId(0);
    cantrip_ai::pay::with_classic(classic, || {
        cantrip_ai::pay::with_follow_ups(me, follow, || {
            for _ in 0..20 {
                let Some(r) = game.pending().cloned() else { break };
                if r.player != me || !matches!(r.kind, RequestKind::PayCosts { .. } | RequestKind::ChooseTarget { .. }) {
                    break;
                }
                let a = match &r.kind {
                    RequestKind::ChooseTarget { .. } => Answer::Targets(vec![Target::Player(PlayerId(1))]),
                    _ => cantrip_ai::pay::pay(&game, &r, None).expect("the autotapper pays"),
                };
                game.answer(r.id, r.player, a).expect("the payment is taken");
            }
        })
    });
    game.state
        .battlefield()
        .iter()
        .filter(|&&id| game.name(id) == "Mountain" && !game.state.obj(id).tapped)
        .count()
}

/// The autotapper keeps open what the plan wants next: told Shock is wanted after Lightning
/// Strike, it pays {1}{R} with a Mountain and an Island and leaves a Mountain for Shock, where the hand's blue symbols
/// alone (the old autotapper) would keep both Islands; told Counterspell is wanted, it keeps both Islands.
#[test]
fn the_autotapper_keeps_open_what_the_plan_wants_next() {
    let shock = ManaCost { symbols: [0, 0, 0, 1, 0, 0], ..ManaCost::ZERO };
    let counterspell = ManaCost { symbols: [0, 2, 0, 0, 0, 0], ..ManaCost::ZERO };
    assert_eq!(strike_leaves(&[], true), 0, "the old autotapper keeps the Islands for the hand's blue cards");
    assert_eq!(strike_leaves(&[(shock, 0.8), (counterspell, 0.1)], false), 1, "Shock wanted: a Mountain stays open");
    assert_eq!(strike_leaves(&[(shock, 0.1), (counterspell, 0.8)], false), 0, "Counterspell wanted: both Islands stay open");
}

// The review's search options (2026-11-13): the root shared over the worlds, Gumbel, the evaluation cache, lazy inner
// steps, budget snapshots, the early stop, exploration at the root, capped lines valued, forced requests unobserved,
// time use, and what the search reports for recording.

/// A network whose every output is a function of one row's input alone (its real tokens: the padding a batch adds is
/// left out), so a row's outputs are the same in any batch: what the evaluation cache's exactness is measured against.
struct Hashed {
    rows: AtomicUsize,
}

impl Hashed {
    fn new() -> Hashed {
        Hashed { rows: AtomicUsize::new(0) }
    }
}

impl Evaluator for Hashed {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        use std::hash::{Hash, Hasher};
        self.rows.fetch_add(b.rows, Ordering::Relaxed);
        let t = b.tokens;
        let nums = cantrip_obs::encode::NUMS;
        let mut out = Outputs { value: Vec::new(), policy: Vec::new(), aux: vec![[0.0; 2]; b.rows] };
        for r in 0..b.rows {
            let kind = &b.kind[r * t..(r + 1) * t];
            let len = kind.iter().rposition(|&k| k != 0).map_or(0, |i| i + 1);
            let mut h = std::collections::hash_map::DefaultHasher::new();
            kind[..len].hash(&mut h);
            b.card[r * t..r * t + len].hash(&mut h);
            for x in &b.nums[r * t * nums..(r * t + len) * nums] {
                x.to_bits().hash(&mut h);
            }
            b.legal[r * t..r * t + len].hash(&mut h);
            b.options[r].hash(&mut h);
            let base = h.finish();
            let unit = |x: u64| ((x >> 11) as f64 / (1u64 << 53) as f64) as f32;
            out.value.push(unit(base) - 0.5);
            out.policy.push((0..b.options[r].1).map(|i| 4.0 * unit(base.rotate_left(7 * i as u32 + 3) ^ i as u64) - 2.0).collect());
        }
        Ok(out)
    }
}

/// A network sure of one option at every step (`index`, when the step has it): `logit` above the rest, `value` for
/// whoever is asked. At a priority's first step option 0 is passing.
struct Peaked {
    index: usize,
    logit: f32,
    value: f32,
}

impl Evaluator for Peaked {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        Ok(Outputs {
            value: vec![self.value; b.rows],
            policy: b.options.iter().map(|&(_, n)| (0..n).map(|i| if i == self.index { self.logit } else { 0.0 }).collect()).collect(),
            aux: vec![[0.0; 2]; b.rows],
        })
    }
}

/// The search's visible result, for comparing two searches that must be the same.
fn fingerprint(d: &cantrip_ai::net::Decision) -> String {
    let s = d.search.as_ref().expect("searched");
    format!(
        "{:?} {:?} {} {} {} {} {:?}",
        d.answer, s.visits, s.root_value, s.simulations, s.nodes, s.positions, s.root_options
    )
}

/// The root's first step as recorded.
fn root_step(s: &search::Searched) -> &search::RootStep {
    s.visits.iter().find(|v| v.prefix == s.root_prefix).expect("the root's first step is recorded")
}

fn bolt_at(game: &Game, d: &cantrip_ai::net::Decision, target: PlayerId) -> bool {
    matches!(&d.answer, Answer::Act(a) if a.kind == ActionKind::CastSpell
        && game.name(a.source.unwrap()) == "Lightning Bolt" && a.targets == [Target::Player(target)])
}

fn root_visits(s: &search::Searched) -> u32 {
    s.root_options.iter().map(|o| o.visits).sum()
}

/// The evaluation cache is exact: on a network whose outputs are a function of each row's input, a search with it is
/// the same search as without it, and it is used (the worlds' roots are one input). Counting the repeated inputs alone
/// changes nothing either.
#[test]
fn the_evaluation_cache_changes_no_search() {
    let (game, request) = handed(&unseen("Llanowar Elves x4\nForest x14"), "cache");
    for mode in [InfoMode::Full, InfoMode::Decklist] {
        let base = Config { worlds: 4, ..Config::simulations(96) };
        let plain = Hashed::new();
        let a = run(&game, &request, mode, &plain, &base);
        let cached = Hashed::new();
        let b = run(&game, &request, mode, &cached, &Config { eval_cache: 1 << 16, ..base.clone() });
        let counted = run(&game, &request, mode, &Hashed::new(), &Config { count_duplicates: true, ..base.clone() });
        assert_eq!(fingerprint(&a), fingerprint(&b), "{mode:?}");
        assert_eq!(fingerprint(&a), fingerprint(&counted), "{mode:?}");
        let (sa, sb, sc) = (a.search.as_ref().unwrap(), b.search.as_ref().unwrap(), counted.search.as_ref().unwrap());
        assert_eq!(sb.duplicate_rows, sc.duplicate_rows, "{mode:?}");
        assert_eq!(sb.cache_hits, sb.duplicate_rows, "every repeated input was served by the cache: {mode:?}");
        assert_eq!(sb.rows + sb.cache_hits, sa.rows, "{mode:?}: {sb:#?}");
        assert_eq!(cached.rows.load(Ordering::Relaxed), sb.rows, "{mode:?}");
        if mode == InfoMode::Decklist {
            assert!(sb.cache_hits >= 3, "the four worlds' roots are one input: {sb:#?}");
        }
        println!("{mode:?}: {} rows, {} repeated, {} distinct", sa.rows, sc.duplicate_rows, sc.distinct_rows);
    }
}

/// The root evaluated once: every world's root is the same observation, so one evaluation serves them all and counts as
/// one simulation.
#[test]
fn the_root_is_evaluated_once_for_every_world() {
    let (game, request) = handed(&unseen("Llanowar Elves x4\nForest x14"), "shared root");
    let a = run(&game, &request, InfoMode::Decklist, &Hashed::new(), &Config { worlds: 4, batch: 4, ..Config::simulations(4) });
    let b = run(
        &game,
        &request,
        InfoMode::Decklist,
        &Hashed::new(),
        &Config { worlds: 4, batch: 4, shared_root: true, ..Config::simulations(4) },
    );
    let (sa, sb) = (a.search.unwrap(), b.search.unwrap());
    // Four simulations: the four roots, or one root and three leaves below it.
    assert_eq!((sa.simulations, sb.simulations), (4, 4));
    assert_eq!(root_visits(&sa), 0, "each world's root its own simulation: {sa:#?}");
    assert_eq!(root_visits(&sb), 3, "the root once, then three simulations below it: {sb:#?}");
    let one = run(&game, &request, InfoMode::Decklist, &Hashed::new(), &Config { worlds: 1, ..Config::simulations(1) });
    let priors = |s: &search::Searched| s.root_options.iter().map(|o| o.prior).collect::<Vec<_>>();
    assert_eq!(priors(&sb), priors(one.search.as_ref().unwrap()));
}

/// One bandit at the root: the worlds' visits of an option are read together, so the win in one takes most of a small
/// budget spread over eight worlds, and the answer is still the win.
#[test]
fn the_root_is_one_bandit_over_the_worlds() {
    let (game, request) = handed(WIN_IN_ONE, "bandit");
    let config = Config { worlds: 8, root_bandit: true, ..Config::simulations(48) };
    let d = run(&game, &request, InfoMode::Decklist, &Uniform(0.0), &config);
    assert!(bolt_at(&game, &d, PlayerId(1)), "{d:#?}");
    let s = d.search.as_ref().unwrap();
    assert_eq!(s.simulations, 48);
    let win = s.root_options.iter().find(|o| o.q.is_some_and(|q| q > 0.99)).expect("the win is an option");
    assert!(win.visits * 2 > root_visits(s), "the win takes most of the root's visits: {s:#?}");
    // The same search again is the same search.
    assert_eq!(fingerprint(&d), fingerprint(&run(&game, &request, InfoMode::Decklist, &Uniform(0.0), &config)));
}

/// Gumbel at the root: the win in one is found, the recorded target is the completed-Q policy (every option some mass,
/// scaled to `TARGET_SCALE`), and the draw is the search's seed's.
#[test]
fn gumbel_finds_the_win_and_records_its_target() {
    let (game, request) = handed(WIN_IN_ONE, "gumbel");
    for (mode, worlds) in [(InfoMode::Full, 1), (InfoMode::Decklist, 4)] {
        let g = search::Gumbel::default();
        let config = Config { worlds, gumbel: Some(g), ..Config::simulations(48) };
        let d = run(&game, &request, mode, &Uniform(0.0), &config);
        assert!(bolt_at(&game, &d, PlayerId(1)), "{mode:?}: {d:#?}");
        let s = d.search.as_ref().unwrap();
        assert_eq!(s.target, search::Target::Gumbel);
        let step = root_step(s);
        let total: u64 = step.visits.iter().map(|&n| u64::from(n)).sum();
        assert!((total as f32 - search::TARGET_SCALE).abs() < 0.01 * search::TARGET_SCALE, "{step:?}");
        assert!(step.visits.iter().filter(|&&n| n > 0).count() > 1, "the target puts mass on more than one option: {step:?}");
        let shares: f32 = s.root_options.iter().filter_map(|o| o.target).sum();
        assert!((shares - 1.0).abs() < 1e-3, "{:?}", s.root_options);
        // With the draw: the same seed draws the same.
        let noisy = Config { gumbel: Some(search::Gumbel { noise: true, ..g }), seed: 99, ..config.clone() };
        let a = run(&game, &request, mode, &Uniform(0.0), &noisy);
        assert_eq!(fingerprint(&a), fingerprint(&run(&game, &request, mode, &Uniform(0.0), &noisy)), "{mode:?}");
        check_answer(&game, &request, &a.answer).expect("legal");
    }
}

/// A flattened priority's inner steps are evaluated when the node is selected again: fewer rows for the same budget,
/// and the search still finds what it found.
#[test]
fn lazy_inner_steps_cost_fewer_rows() {
    let (game, request) = handed(BOLT_EITHER_WAY, "lazy");
    let eager = run(&game, &request, InfoMode::Full, &Uniform(0.0), &Config::simulations(200));
    let lazy = run(&game, &request, InfoMode::Full, &Uniform(0.0), &Config { lazy_inner: true, ..Config::simulations(200) });
    assert!(bolt_at(&game, &lazy, PlayerId(1)), "{lazy:#?}");
    let (se, sl) = (eager.search.unwrap(), lazy.search.unwrap());
    assert_eq!(se.simulations, sl.simulations);
    assert!(sl.rows < se.rows, "lazy {} rows, eager {}", sl.rows, se.rows);
    assert!(sl.lazy_refined + sl.lazy_pending_rows > 0, "{sl:#?}");
}

/// Budget snapshots: the root's visits at each count asked, and at the end.
#[test]
fn budget_snapshots_are_taken() {
    let (game, request) = handed(WIN_IN_ONE, "snapshots");
    let d = run(&game, &request, InfoMode::Full, &Uniform(0.0), &Config { snapshots: vec![32, 8], ..Config::simulations(96) });
    let s = d.search.unwrap();
    assert_eq!(s.snapshots.len(), 3, "{:#?}", s.snapshots);
    assert!(s.snapshots[0].simulations >= 8 && s.snapshots[1].simulations >= 32, "{:#?}", s.snapshots);
    assert_eq!(s.snapshots[2].simulations, 96);
    let sum = |k: usize| s.snapshots[k].options.iter().map(|o| o.1).sum::<u32>();
    assert!(sum(0) <= sum(1) && sum(1) <= sum(2), "{:#?}", s.snapshots);
    assert!(s.snapshots[2].best.is_some());
}

/// The early stop: a root with one option visited after the count, none cut and nothing close, stops; one with several
/// visited does not.
#[test]
fn a_search_with_nothing_to_compare_stops_early() {
    let (game, request) = handed(WIN_IN_ONE, "early stop");
    let sure = Peaked { index: 1, logit: 25.0, value: 0.0 };
    let config = Config { early_stop: 16, batch: 4, ..Config::simulations(256) };
    let d = run(&game, &request, InfoMode::Full, &sure, &config);
    let s = d.search.as_ref().unwrap();
    assert!(s.early_stopped && s.simulations < 64, "{s:#?}");
    // One option holds the visits, but for a detour of virtual loss's in the first batch.
    let most = s.root_options.iter().map(|o| o.visits).max().unwrap();
    assert!(root_visits(s) - most < 4, "{s:#?}");
    // A batch of one makes no detour: one option visited.
    let d = run(&game, &request, InfoMode::Full, &sure, &Config { batch: 1, ..config.clone() });
    let s = d.search.as_ref().unwrap();
    assert!(s.early_stopped && s.simulations < 64, "{s:#?}");
    assert_eq!(s.root_options.iter().filter(|o| o.visits > 0).count(), 1, "{s:#?}");
    let d = run(&game, &request, InfoMode::Full, &Uniform(0.0), &config);
    let s = d.search.as_ref().unwrap();
    assert!(!s.early_stopped && s.simulations == 256, "{s:#?}");
}

/// The root's priors for exploration: a floor lifts every option, noise moves them alike in every world, and the
/// network's own are kept beside them.
#[test]
fn the_roots_priors_take_a_floor_and_noise() {
    let (game, request) = handed(WIN_IN_ONE, "floor");
    let sure = Peaked { index: 1, logit: 25.0, value: 0.0 };
    let d = run(&game, &request, InfoMode::Full, &sure, &Config { root_floor: 0.5, ..Config::simulations(16) });
    let s = d.search.unwrap();
    let k = s.root_options.len() as f32;
    assert!(k >= 3.0, "{:?}", s.root_options);
    for o in &s.root_options {
        assert!(o.prior >= 0.5 / k - 1e-6, "{o:?}");
    }
    assert!(s.root_options.iter().any(|o| o.net_prior < 1e-6), "{:?}", s.root_options);
    let noisy = Config { worlds: 3, root_noise: 0.25, report: true, ..Config::simulations(16) };
    let d = run(&game, &request, InfoMode::Decklist, &Uniform(0.0), &noisy);
    let s = d.search.unwrap();
    let sum: f32 = s.root_options.iter().map(|o| o.prior).sum();
    assert!((sum - 1.0).abs() < 1e-4, "{:?}", s.root_options);
    assert!(s.root_options.iter().any(|o| (o.prior - o.net_prior).abs() > 1e-4), "{:?}", s.root_options);
    let worlds = &s.report.as_ref().unwrap().worlds;
    let priors = |w: &search::WorldReport| w.edges.iter().map(|e| e.prior).collect::<Vec<_>>();
    assert!(worlds.windows(2).all(|p| priors(&p[0]) == priors(&p[1])), "every world draws the same noise: {worlds:#?}");
}

/// KataGo's forced playouts at the root: every option with a prior gets its forced visits, and the recorded target
/// takes back the ones the values do not justify (the most visited keeps all of its).
#[test]
fn forced_playouts_are_taken_and_pruned_from_the_target() {
    let (game, request) = handed(WIN_IN_ONE, "forced");
    let leaning = Peaked { index: 1, logit: 3.0, value: 0.0 };
    let d = run(&game, &request, InfoMode::Full, &leaning, &Config { forced_playouts: 2.0, ..Config::simulations(128) });
    let s = d.search.as_ref().unwrap();
    assert_eq!(s.target, search::Target::Pruned);
    let n: u32 = root_visits(s);
    for o in s.root_options.iter().filter(|o| !o.dead && !o.pruned) {
        let forced = (2.0 * o.prior * n as f32).sqrt();
        assert!(o.visits as f32 + 1.0 >= forced.floor(), "{o:?} under its forced visits {forced}");
    }
    let step = root_step(s);
    let mut real: BTreeMap<Token, u32> = BTreeMap::new();
    for o in &s.root_options {
        *real.entry(o.path[0]).or_default() += if o.pruned { 0 } else { o.visits };
    }
    for (t, &v) in step.tokens.iter().zip(&step.visits) {
        assert!(v <= real.get(t).copied().unwrap_or(0), "the target never adds visits: {step:?} {real:?}");
    }
    let most = real.values().copied().max().unwrap();
    assert!(step.visits.contains(&most), "the most visited keeps its visits: {step:?} {real:?}");
}

/// The shortcut audit: at a priority the shortcut passes, a full search with a floor under the priors looks at every
/// option, and says how the best of them compares with passing.
#[test]
fn the_shortcut_audit_searches_what_the_shortcut_passes() {
    let (game, request) = handed(WIN_IN_ONE, "audit");
    let pass_sure = Peaked { index: 0, logit: 25.0, value: 0.0 };
    let d = run(&game, &request, InfoMode::Full, &pass_sure, &Config::simulations(64));
    assert!(matches!(d.answer, Answer::Pass), "the shortcut passes: {d:#?}");
    let k = Knowledge::of(&game, request.player);
    let audit = search::audit_shortcut(
        &game,
        &request,
        &k,
        InfoMode::Full,
        table(),
        &pass_sure,
        &Config { root_floor: 0.2, ..Config::simulations(200) },
        prior(),
        &CancelGuard::default(),
        &WallClock,
    )
    .expect("the audit searches")
    .expect("the shortcut's rule holds here");
    let s = audit.decision.search.as_ref().unwrap();
    assert!(s.shortcut_would && !s.shortcut, "{s:#?}");
    assert!(s.root_options.iter().filter(|o| o.visits > 0).count() > 1, "{s:#?}");
    assert!(audit.margin.is_some_and(|m| m > 0.5), "the win in one beats passing: {audit:#?}");
    assert!(bolt_at(&game, &audit.decision, PlayerId(1)), "{audit:#?}");
}

/// A line capped at the turn cap takes the network's value there (decision 8), not a draw.
#[test]
fn a_capped_line_takes_the_networks_value() {
    let (game, request) = handed(FORCED_TURN, "capped");
    let capped = Config { turn_cap: 0, ..Config::simulations(16) };
    let d = run(&game, &request, InfoMode::Full, &Uniform(0.25), &capped);
    let s = d.search.as_ref().unwrap();
    assert!(s.capped_valued > 0, "{s:#?}");
    assert!((s.root_value.abs() - 0.25).abs() < 1e-4, "every line ends at the cap, valued 0.25 for whoever is asked there: {s:#?}");
    let d = run(&game, &request, InfoMode::Full, &Uniform(0.25), &Config { cap_value: false, ..capped });
    let s = d.search.as_ref().unwrap();
    assert_eq!((s.capped_valued, s.root_value), (0, 0.0), "a draw without it: {s:#?}");
}

/// A priority with nothing but passing is passed without observing it: the same search, with fewer observations.
#[test]
fn forced_passes_are_played_unobserved_and_change_nothing() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    for (text, name) in [(FORCED_TURN.to_string(), "forced turn"), (unseen("Llanowar Elves x4\nForest x14"), "unseen")] {
        let (game, request) = handed(&text, name);
        for mode in [InfoMode::Full, InfoMode::Decklist] {
            let on = Config { worlds: 2, ..Config::simulations(48) };
            let a = run(&game, &request, mode, &evaluator, &on);
            let b = run(&game, &request, mode, &evaluator, &Config { skip_forced_observe: false, ..on });
            assert_eq!(fingerprint(&a), fingerprint(&b), "{name} {mode:?}");
            let (sa, sb) = (a.search.unwrap(), b.search.unwrap());
            assert!(sa.forced_unobserved > 0 && sb.forced_unobserved == 0, "{name} {mode:?}");
            println!(
                "{name} {mode:?}: {} passes unobserved; observing {:?} against {:?}, playing {:?} against {:?}",
                sa.forced_unobserved, sa.observe_time, sb.observe_time, sa.play_time, sb.play_time
            );
        }
    }
}

/// Time use at play: past its share of the time, a calm root stops the search before the deadline.
#[test]
fn time_use_stops_a_calm_search_early() {
    let (game, request) = handed(WIN_IN_ONE, "time use");
    let start = Instant::now();
    let clock = Ticking { at: Cell::new(start) };
    let deadline = start + Duration::from_millis(2000);
    let calm = search::TimeUse { min_share: 0.05, entropy: 10.0, swing: 10.0 };
    let config = Config { deadline: Some(deadline), time_use: Some(calm), ..Config::default() };
    let k = Knowledge::of(&game, request.player);
    let d = search::search(&game, &request, &k, InfoMode::Full, table(), &Uniform(0.0), &config, prior(), &CancelGuard::default(), &clock)
        .unwrap();
    let s = d.search.as_ref().unwrap();
    assert!(s.time_stopped, "{s:#?}");
    assert!(clock.at.get() < start + Duration::from_millis(400), "stopped soon after its share: {:?}", clock.at.get() - start);
}

/// What the search reports for recording: each root option once with its visits and values, the sub-steps the search
/// chose marked so, and the recorded visits at the root the options' visits.
#[test]
fn the_search_reports_its_root_options_and_steps() {
    let (game, request) = handed(WIN_IN_ONE, "report");
    let d = run(&game, &request, InfoMode::Full, &Uniform(0.0), &Config::simulations(96));
    let s = d.search.as_ref().unwrap();
    assert_eq!(s.target, search::Target::Visits);
    assert!(!s.steps.is_empty() && s.steps[0].searched, "{:#?}", s.steps);
    assert_eq!(s.steps.iter().filter(|o| o.searched).count(), s.visits.len(), "{:#?}", s.steps);
    let step = root_step(s);
    let recorded: u32 = step.visits.iter().sum();
    let options: u32 = s.root_options.iter().filter(|o| !o.pruned).map(|o| o.visits).sum();
    assert_eq!(recorded, options, "{s:#?}");
    for o in &s.root_options {
        assert_eq!(o.q.is_some(), o.visits > 0, "{o:?}");
        if let Some(q) = o.q {
            assert_eq!(o.completed_q, q);
        }
    }
}

#[test]
fn search_settings_are_set_by_name() {
    let mut c = Config::default();
    for pair in [
        "PriorTemperature=1.5",
        "Worlds=4.0",
        "CPuct=2",
        "Prune=false",
        "GumbelNoise=true",
        "EvalCache=65536",
        "Snapshots=256,1024",
        "TimeEntropy=0.3",
        "RootFloor=0.05",
    ] {
        c.set_pair(pair).unwrap_or_else(|e| panic!("{pair}: {e}"));
    }
    assert_eq!((c.prior_temperature, c.worlds, c.c_puct, c.prune), (1.5, 4, 2.0, false));
    assert!(c.gumbel.is_some_and(|g| g.noise && g.considered == 16));
    assert_eq!((c.eval_cache, c.snapshots.clone(), c.root_floor), (65536, vec![256, 1024], 0.05));
    assert!(c.time_use.is_some_and(|t| t.entropy == 0.3));
    c.set("Gumbel", "false").unwrap();
    assert!(c.gumbel.is_none());
    assert!(c.set("NoSuchSetting", "1").is_err());
    assert!(c.set("Worlds", "four").is_err());
}

/// A measurement, not a check (run by name): the share of rows a search sends whose network input it already sent -
/// what the evaluation cache saves - over whole games of both seats searching in Decklist mode with their trees kept,
/// as self-play plays, on the tiny model. `CANTRIP_DUP_GAMES` games (2), `CANTRIP_DUP_SIMS` simulations (64). Also
/// checks the cache against the tract evaluator itself: the same search with and without it.
#[test]
#[ignore]
fn measure_duplicate_inputs() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let decks = [deck("dimir-midrange.txt"), deck("boros-dragons.txt")];
    let games: u64 = std::env::var("CANTRIP_DUP_GAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
    let sims: usize = std::env::var("CANTRIP_DUP_SIMS").ok().and_then(|v| v.parse().ok()).unwrap_or(64);
    let (mut rows, mut repeated, mut searches, mut differ) = (0usize, 0usize, 0usize, 0usize);
    for seed in 0..games {
        let mut game = deal(&[decks[0].clone(), decks[1].clone()], 91_000 + seed, true, None).unwrap();
        let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
        let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
        let mut trees: Vec<Option<search::Kept>> = vec![None, None];
        for _ in 0..3000 {
            if game.is_over() || game.state.turn.number > 20 {
                break;
            }
            let Some(request) = game.pending().cloned() else { break };
            let p = request.player.index();
            ks[p].update(&game);
            let config = Config { seed: request.id as u64, count_duplicates: true, ..Config::simulations(sims) };
            let k = &ks[p];
            let tree = &mut trees[p];
            let d = payments[p]
                .decide(&game, &request, k, InfoMode::Decklist, |request, guard| {
                    search::search_keeping(game_ref(&game), request, k, InfoMode::Decklist, table(), &evaluator, &config, prior(), guard, &WallClock, tree)
                })
                .unwrap();
            if let Some(s) = d.search.as_ref().filter(|s| s.simulations > 0) {
                searches += 1;
                rows += s.rows;
                repeated += s.duplicate_rows;
                // Now and then, the same search fresh with and without the cache, on tract itself.
                if searches % 25 == 1 {
                    let fresh = Config { reuse: search::Reuse::None, count_duplicates: false, ..config.clone() };
                    let a = search::search(&game, &request, k, InfoMode::Decklist, table(), &evaluator, &fresh, prior(), &CancelGuard::default(), &WallClock).unwrap();
                    let b = search::search(&game, &request, k, InfoMode::Decklist, table(), &evaluator, &Config { eval_cache: 1 << 16, ..fresh }, prior(), &CancelGuard::default(), &WallClock).unwrap();
                    if a.search.is_some() && fingerprint(&a) != fingerprint(&b) {
                        differ += 1;
                    }
                }
            }
            let board = cantrip_ai::cancel::board_hash(&game.state);
            payments[p].played(&game, board, &request, &d.answer);
            game.answer(request.id, request.player, d.answer).expect("the answer is taken");
        }
    }
    println!(
        "{games} games, {searches} searches at {sims}: {rows} rows, {repeated} repeated ({:.1}%); tract with the cache differed in {differ} of {} searches checked",
        100.0 * repeated as f64 / rows.max(1) as f64,
        searches.div_ceil(25)
    );
}

fn game_ref(g: &Game) -> &Game {
    g
}

/// A measurement (run by name): whether the tract evaluator's outputs for a row depend on the batch it is in (the
/// tiny model's 48 reference positions, alone and all together). The evaluation cache is exact for an evaluator whose
/// outputs are a function of the row's input (the GPU server's are bit-identical per input); one whose sums move with
/// the batch's shape gives a cached row the bits of its first batch.
#[test]
#[ignore]
fn measure_tract_batch_invariance() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let r = cantrip_ai::eval::reference::Reference::load(&tiny().join("reference")).unwrap();
    let all: Vec<&cantrip_obs::encode::Encoded> = r.positions.iter().collect();
    let together = evaluator.evaluate(&FeatureBatch::new(&all)).unwrap();
    let (mut same, mut largest) = (0usize, 0.0f32);
    for (i, e) in r.positions.iter().enumerate() {
        let alone = evaluator.evaluate(&FeatureBatch::new(&[e])).unwrap();
        let bits = |o: &Outputs, k: usize| (o.value[k].to_bits(), o.policy[k].iter().map(|x| x.to_bits()).collect::<Vec<_>>());
        if bits(&alone, 0) == bits(&together, i) {
            same += 1;
        }
        largest = largest.max((alone.value[0] - together.value[i]).abs());
    }
    println!("tract: {same} of {} rows bit-identical alone and in a batch of all; largest value difference {largest:e}", r.positions.len());
}

/// A measurement (run by name, `--ignored --nocapture`): how often a searched sub-step has two legal options with an
/// equal token (interchangeable options, which make the same answer: `Searched::equal_tokens`), over whole games of the
/// tiny network searching in both seats, every request (Full and Decklist, several standard pairings).
#[test]
#[ignore]
fn measure_equal_tokens_at_searched_steps() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    // Every standard list against the next, in name order.
    let mut names: Vec<String> = std::fs::read_dir(root().join("decks/standard"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.ends_with(".txt"))
        .filter(|n| deck(n).len() >= 40)
        .collect();
    names.sort();
    let pairings: Vec<(String, String)> = names.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0].clone(), c[1].clone())).collect();
    let (mut requests, mut steps, mut equal) = (0usize, 0usize, 0usize);
    for (k, (a, b)) in pairings.iter().enumerate() {
        for (mode, worlds) in [(InfoMode::Full, 1), (InfoMode::Decklist, 2)] {
            let mut game = deal(&[deck(a), deck(b)], 91_000 + k as u64, true, None).unwrap();
            let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
            let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
            for _ in 0..3000 {
                if game.is_over() || game.state.turn.number > 16 {
                    break;
                }
                let Some(request) = game.pending().cloned() else { break };
                let p = request.player.index();
                ks[p].update(&game);
                let config = Config { worlds, seed: request.id as u64, ..Config::simulations(24) };
                let kn = &ks[p];
                let d = payments[p]
                    .decide(&game, &request, kn, mode, |request, guard| {
                        search::search(&game, request, kn, mode, table(), &evaluator, &config, prior(), guard, &WallClock)
                    })
                    .unwrap();
                if let Some(s) = d.search.as_ref().filter(|s| !s.visits.is_empty()) {
                    requests += 1;
                    steps += s.visits.len();
                    equal += s.equal_tokens;
                }
                let board = cantrip_ai::cancel::board_hash(&game.state);
                payments[p].played(&game, board, &request, &d.answer);
                game.answer(request.id, request.player, d.answer).expect("the answer is taken");
            }
        }
    }
    println!(
        "equal tokens: {equal} of {steps} searched sub-steps ({:.3}%), over {requests} searched requests",
        100.0 * equal as f64 / steps.max(1) as f64
    );
    assert!(requests > 100, "too few searches to measure: {requests}");
}

/// A measurement (run by name with `CANTRIP_SEARCH_DUMP=FILE`): every search of whole games under several
/// configurations, written as what a search returns and records (the answer, the root's value, each recorded sub-step's
/// tokens, visits and choice, and the root's options with their priors, visits, values and targets), bit for bit: two
/// builds that must search alike write the same file.
#[test]
#[ignore]
fn dump_searches() {
    let Some(out) = std::env::var_os("CANTRIP_SEARCH_DUMP") else { return };
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let configs: Vec<(&str, InfoMode, Config)> = vec![
        ("default", InfoMode::Full, Config::simulations(32)),
        ("worlds", InfoMode::Decklist, Config { worlds: 3, ..Config::simulations(48) }),
        ("bandit", InfoMode::Decklist, Config { worlds: 3, root_bandit: true, ..Config::simulations(48) }),
        ("gumbel", InfoMode::Decklist, Config { worlds: 2, gumbel: Some(search::Gumbel { noise: true, ..search::Gumbel::default() }), ..Config::simulations(48) }),
        ("forced", InfoMode::Full, Config { forced_playouts: 2.0, ..Config::simulations(48) }),
        ("noise", InfoMode::Full, Config { root_noise: 0.25, root_floor: 0.1, ..Config::simulations(32) }),
        ("lazy", InfoMode::Full, Config { lazy_inner: true, ..Config::simulations(48) }),
    ];
    let bits = |x: f32| format!("{:08x}", x.to_bits());
    let mut text = String::new();
    for (name, mode, base) in &configs {
        for (k, (a, b)) in [("dimir-midrange.txt", "boros-dragons.txt"), ("izzet-elementals.txt", "golgari-midrange.txt")].iter().enumerate() {
            let mut game = deal(&[deck(a), deck(b)], 95_000 + k as u64, true, None).unwrap();
            let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
            let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
            let mut kept: Vec<Option<search::Kept>> = vec![None, None];
            for _ in 0..600 {
                if game.is_over() || game.state.turn.number > 10 {
                    break;
                }
                let Some(request) = game.pending().cloned() else { break };
                let p = request.player.index();
                ks[p].update(&game);
                let config = Config { seed: request.id as u64, ..base.clone() };
                let kn = &ks[p];
                let keep = &mut kept[p];
                let d = payments[p]
                    .decide(&game, &request, kn, *mode, |request, guard| {
                        search::search_keeping(&game, request, kn, *mode, table(), &evaluator, &config, prior(), guard, &WallClock, keep)
                    })
                    .unwrap();
                text += &format!("{name} {k} {} {:?}\n", request.id, d.answer);
                if let Some(s) = &d.search {
                    text += &format!("  value {} sims {} nodes {}\n", bits(s.root_value), s.simulations, s.nodes);
                    for r in &s.visits {
                        text += &format!("  step {:?} {:?} {:?} {:?}\n", r.prefix, r.tokens, r.visits, r.chosen);
                    }
                    for o in &s.root_options {
                        text += &format!(
                            "  option {:?} {:?} {} {} {} {} {} {:?}\n",
                            o.path, o.keep, bits(o.prior), bits(o.net_prior), o.visits, bits(o.value_sum), bits(o.completed_q),
                            o.target.map(bits)
                        );
                    }
                }
                let board = cantrip_ai::cancel::board_hash(&game.state);
                payments[p].played(&game, board, &request, &d.answer);
                game.answer(request.id, request.player, d.answer).expect("the answer is taken");
            }
        }
    }
    std::fs::write(out, text).unwrap();
}

/// Each recorded sub-step's options carry their index in the step and their root values (`RootStep::options`, `q`):
/// the values the root's options give, summed over every option whose path runs through it (what a recorder read from
/// the options before), bit for bit.
#[test]
fn a_recorded_step_carries_its_options_indices_and_values() {
    let (game, request) = handed(BOLT_EITHER_WAY, "indices");
    for (mode, config) in [
        (InfoMode::Full, Config::simulations(64)),
        (InfoMode::Decklist, Config { worlds: 3, root_bandit: true, ..Config::simulations(96) }),
        (InfoMode::Decklist, Config { worlds: 2, gumbel: Some(search::Gumbel::default()), ..Config::simulations(64) }),
    ] {
        let d = run(&game, &request, mode, &Uniform(0.0), &config);
        let s = d.search.as_ref().unwrap();
        assert!(!s.visits.is_empty());
        assert_eq!(s.equal_tokens, 0);
        for r in &s.visits {
            assert_eq!((r.options.len(), r.q.len()), (r.tokens.len(), r.tokens.len()), "{r:?}");
            let rest = r.prefix.strip_prefix(&s.root_prefix[..]);
            for (k, t) in r.tokens.iter().enumerate() {
                let want = rest.map_or(f32::NAN, |rest| {
                    let (mut n, mut w) = (0u64, 0.0f64);
                    for o in &s.root_options {
                        if o.path.len() > rest.len() && o.path[..rest.len()] == *rest && o.path[rest.len()] == *t {
                            n += u64::from(o.visits);
                            w += f64::from(o.value_sum);
                        }
                    }
                    if n > 0 { (w / n as f64) as f32 } else { f32::NAN }
                });
                assert!(want.to_bits() == r.q[k].to_bits() || (want.is_nan() && r.q[k].is_nan()), "{mode:?} {r:?}: {want}");
            }
        }
    }
}

/// Gumbel reads the network's own priors: its logits and its target are the network's, not the root's floor or noise
/// (exploration PUCT reads), so a Gumbel search with a floor and noise at the root searches and records as one without.
#[test]
fn gumbel_reads_the_networks_own_priors() {
    let (game, request) = handed(BOLT_EITHER_WAY, "gumbel raw");
    let leaning = Peaked { index: 1, logit: 2.0, value: 0.0 };
    for (mode, worlds) in [(InfoMode::Full, 1), (InfoMode::Decklist, 3)] {
        let g = search::Gumbel { noise: true, ..search::Gumbel::default() };
        let plain = Config { worlds, gumbel: Some(g), seed: 5, ..Config::simulations(64) };
        let explored = Config { root_floor: 0.5, root_noise: 0.25, ..plain.clone() };
        let (a, b) = (run(&game, &request, mode, &leaning, &plain), run(&game, &request, mode, &leaning, &explored));
        let (sa, sb) = (a.search.as_ref().unwrap(), b.search.as_ref().unwrap());
        assert_eq!(format!("{:?}", a.answer), format!("{:?}", b.answer), "{mode:?}");
        assert_eq!(sa.visits, sb.visits, "{mode:?}");
        assert!(sb.root_options.iter().any(|o| (o.prior - o.net_prior).abs() > 1e-3), "the floor moved the priors: {:?}", sb.root_options);
        for (x, y) in sa.root_options.iter().zip(&sb.root_options) {
            assert_eq!((x.visits, x.target, x.net_prior), (y.visits, y.target, y.net_prior), "{mode:?}");
        }
    }
}

/// Bot1 at its fourth main phase with Island, Island, Mountain, Mountain in play and Lightning Strike, Shock, Opt and
/// Brainstorm in hand: once a blue spell is cast, Lightning Strike can be paid to keep the Island (the other) or a
/// Mountain (Shock) open, a cast the search splits by class of payment (`Config::pay_split`).
const STRIKE_SHOCK_OPT: &str = "[Bot1 Deck]
Island
Mountain
Island
Mountain
Lightning Strike
Shock
Opt
Brainstorm
Island x12

[Bot2 Deck]
Island x18

[Actions]
Bot1.Hand.Island.Play
[T3 Bot1.Main1]
Bot1.Hand.Mountain.Play
[T5 Bot1.Main1]
Bot1.Hand.Island.Play
[T7 Bot1.Main1]
Bot1.Hand.Mountain.Play
[Handover Bot1.Pass]

[Expect]
Turn = 7
";

/// A lazily expanded node (`Config::lazy_inner`), once refined, has the edges an eager expansion makes: a cast split by
/// class of payment (`Config::pay_split`) is split alike, from the options' first-step priors in both. Checked where
/// a kept tree's lazily expanded node becomes the root: the reuse check builds the same root fresh, eagerly, and
/// compares the edges and their priors (`search::ReuseCheck`), over a turn of casts the search splits.
#[test]
fn a_lazy_node_refined_has_the_edges_of_an_eager_one() {
    let (mut rerooted, mut checked, mut splits) = (0usize, 0usize, 0usize);
    let mut mismatches = std::collections::BTreeMap::<String, usize>::new();
    for seed in 0..12u64 {
        let (mut game, _) = handed(STRIKE_SHOCK_OPT, "lazy and eager");
        // Even priors: the blue spell and Shock weigh alike, so the plan does not settle Lightning Strike's payment.
        let evaluator = Uniform(0.0);
        let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
        let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
        let mut kept: Vec<Option<search::Kept>> = vec![None, None];
        for _ in 0..200 {
            if game.is_over() || game.state.turn.number > 12 {
                break;
            }
            let Some(request) = game.pending().cloned() else { break };
            let p = request.player.index();
            ks[p].update(&game);
            let config = Config {
                seed: seed * 1000 + request.id as u64,
                lazy_inner: true,
                reuse: search::Reuse::Add,
                reuse_follow: search::Follow::All,
                reuse_check: true,
                choice: Choice::Sample { temperature: 1.0, seed: seed * 7 + request.id as u64 },
                ..Config::simulations(160)
            };
            let kn = &ks[p];
            let keep = &mut kept[p];
            let d = payments[p]
                .decide(&game, &request, kn, InfoMode::Full, |request, guard| {
                    search::search_keeping(&game, request, kn, InfoMode::Full, table(), &evaluator, &config, prior(), guard, &WallClock, keep)
                })
                .unwrap();
            if let Some(s) = d.search.as_ref() {
                splits += s.pay_splits;
                if let Some(r) = s.reused.as_ref() {
                    rerooted += 1;
                    let c = r.check.as_ref().expect("the check ran");
                    checked += c.worlds;
                    for (what, n) in &c.mismatches {
                        *mismatches.entry(what.clone()).or_default() += n;
                    }
                }
            }
            let board = cantrip_ai::cancel::board_hash(&game.state);
            payments[p].played(&game, board, &request, &d.answer);
            game.answer(request.id, request.player, d.answer).expect("the answer is taken");
        }
    }
    println!("{rerooted} re-rooted searches, {checked} worlds checked, {splits} casts split by payment");
    assert!(rerooted > 8 && splits > 0, "too little to check: {rerooted} searches, {checked} worlds, {splits} splits");
    assert!(mismatches.is_empty(), "lazily expanded roots differ from eager ones: {mismatches:?} over {checked} worlds");
}

/// A measurement (run by name with `CANTRIP_SPLIT_DUMP=FILE`): the turn of `STRIKE_SHOCK_OPT`, searched eagerly at
/// every request on even and on leaning priors, written as each search's root options with their classes of payment.
#[test]
#[ignore]
fn dump_split_searches() {
    let Some(out) = std::env::var_os("CANTRIP_SPLIT_DUMP") else { return };
    let mut text = String::new();
    let evaluators: [(&str, &dyn Evaluator); 3] =
        [("even", &Uniform(0.0)), ("leaning", &Peaked { index: 1, logit: 1.0, value: 0.0 }), ("hashed", &Hashed::new())];
    for (name, evaluator) in evaluators {
        for seed in 0..4u64 {
            let (mut game, _) = handed(STRIKE_SHOCK_OPT, "split dump");
            let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
            for _ in 0..80 {
                if game.is_over() || game.state.turn.number > 10 {
                    break;
                }
                let Some(request) = game.pending().cloned() else { break };
                let p = request.player.index();
                let kn = Knowledge::of(&game, request.player);
                let config = Config {
                    seed: seed * 1000 + request.id as u64,
                    choice: Choice::Sample { temperature: 1.0, seed: seed * 7 + request.id as u64 },
                    ..Config::simulations(96)
                };
                let d = payments[p]
                    .decide(&game, &request, &kn, InfoMode::Full, |request, guard| {
                        search::search(&game, request, &kn, InfoMode::Full, table(), evaluator, &config, prior(), guard, &WallClock)
                    })
                    .unwrap();
                text += &format!("{name} {seed} {} {:?}\n", request.id, d.answer);
                if let Some(s) = &d.search {
                    text += &format!("  splits {}\n", s.pay_splits);
                    for o in &s.root_options {
                        text += &format!("  option {:?} {:?} {:08x} {}\n", o.path, o.keep, o.prior.to_bits(), o.visits);
                    }
                }
                let board = cantrip_ai::cancel::board_hash(&game.state);
                payments[p].played(&game, board, &request, &d.answer);
                game.answer(request.id, request.player, d.answer).expect("the answer is taken");
            }
        }
    }
    std::fs::write(out, text).unwrap();
}

/// Both seats with lands and Shocks from their fourth turn: decisions in each other's turns, so a kept tree's root
/// moves from turn to turn.
const SHOCKS_BOTH_WAYS: &str = "[Bot1 Deck]
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

/// A kept tree's capped lines (`Config::turn_cap`: valued at the cap, then terminals) are not the tree a fresh search
/// would grow once the root moves on: a capped node never becomes the root (its request was never expanded), and a line
/// capped at the old horizon that the new one reaches is played again to it (`Reused::uncapped`; the opening itself is
/// `search::uncap`'s unit test). The reuse check finds every re-rooted root as a fresh search builds it, over turns of
/// caps of none and one turn.
#[test]
fn a_kept_trees_capped_lines_move_with_the_horizon() {
    let (mut rerooted, mut uncapped, mut capped) = (0usize, 0usize, 0usize);
    let mut mismatches = std::collections::BTreeMap::<String, usize>::new();
    let mut lost = std::collections::BTreeMap::<String, usize>::new();
    for (seed, turn_cap) in (0..10u64).map(|s| (s, (s % 2) as u32)) {
        let (mut game, _) = handed(SHOCKS_BOTH_WAYS, "capped lines");
        let evaluator = Uniform(0.0);
        let mut ks: Vec<Knowledge> = game.state.seats().map(Knowledge::new).collect();
        let mut payments = vec![cantrip_ai::net::Payment::default(); 2];
        let mut kept: Vec<Option<search::Kept>> = vec![None, None];
        for _ in 0..300 {
            if game.is_over() || game.state.turn.number > 13 {
                break;
            }
            let Some(request) = game.pending().cloned() else { break };
            let p = request.player.index();
            ks[p].update(&game);
            let config = Config {
                seed: seed * 1000 + request.id as u64,
                turn_cap,
                reuse: search::Reuse::Add,
                reuse_follow: search::Follow::All,
                reuse_check: true,
                choice: Choice::Sample { temperature: 1.0, seed: seed * 7 + request.id as u64 },
                ..Config::simulations(160)
            };
            let kn = &ks[p];
            let keep = &mut kept[p];
            let d = payments[p]
                .decide(&game, &request, kn, InfoMode::Full, |request, guard| {
                    search::search_keeping(&game, request, kn, InfoMode::Full, table(), &evaluator, &config, prior(), guard, &WallClock, keep)
                })
                .unwrap();
            if let Some(s) = d.search.as_ref() {
                capped += s.capped_valued;
                if let Some(r) = s.reused.as_ref() {
                    rerooted += 1;
                    uncapped += r.uncapped;
                    for (what, n) in &r.lost {
                        *lost.entry(what.clone()).or_default() += n;
                    }
                    for (what, n) in &r.check.as_ref().expect("the check ran").mismatches {
                        *mismatches.entry(what.clone()).or_default() += n;
                    }
                }
            }
            let board = cantrip_ai::cancel::board_hash(&game.state);
            payments[p].played(&game, board, &request, &d.answer);
            game.answer(request.id, request.player, d.answer).expect("the answer is taken");
        }
    }
    println!("{rerooted} re-rooted searches, {capped} lines capped, {uncapped} played again past the old cap; lost {lost:?}");
    assert!(rerooted > 8 && capped > 0, "too little to check: {rerooted} searches, {capped} capped");
    assert!(mismatches.is_empty(), "re-rooted roots differ from fresh ones: {mismatches:?}");
}
