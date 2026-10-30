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
    let d = run(
        &game,
        &request,
        InfoMode::Full,
        &Uniform(0.0),
        &Config::simulations(200),
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
    let d = run(
        &game,
        &request,
        InfoMode::Full,
        &Uniform(0.0),
        &Config::simulations(64),
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

/// A decklist of `decks/standard`, its main deck only.
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
        if let Some((n, card)) = line.split_once(' ')
            && let Ok(n) = n.parse::<usize>()
        {
            out.extend(std::iter::repeat_n(card.trim().to_string(), n));
        }
    }
    out
}

/// Every world a search deals shows the seat the game it is in: over whole games of both seats searching, mulligans
/// included (the tiny network mulligans often: hands shuffled back, cards bottomed), each world's root observation is
/// the real one. A hand card a mulligan shuffled back, or a revealed card put into a library, is that card in every
/// world; a library the seat is searching keeps its order; and a run that drew on the generator before its request (a
/// mulligan's shuffle and redraw) is answered without worlds, since every world would deal it again.
#[test]
fn every_world_dealt_shows_the_seat_the_game_it_is_in() {
    let evaluator = TractEvaluator::load(&tiny(), table()).unwrap();
    let decks = [deck("dimir-midrange.txt"), deck("boros-dragons.txt")];
    let (mut searched, mut checked, mut mismatched) = (0usize, 0usize, 0usize);
    for seed in 0..2u64 {
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
                check_worlds: true,
                ..Config::simulations(16)
            };
            let k = &ks[p];
            let d = payments[p]
                .decide(&game, &request, k, InfoMode::Decklist, |request, guard| {
                    search::search(&game, request, k, InfoMode::Decklist, table(), &evaluator, &config, prior(), guard, &WallClock)
                })
                .unwrap();
            if let Some(s) = d.search.as_ref().filter(|s| s.worlds_checked > 0) {
                searched += 1;
                checked += s.worlds_checked;
                mismatched += s.worlds_mismatched;
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
