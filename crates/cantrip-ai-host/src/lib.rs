//! The AI player as a process of its own beside a table (docs/ai-host.md): it keeps a mirror of each game the table
//! hosts, made the way the native host makes it (`cantrip_host::session::Session`, from the same decklists, seed or
//! scenario) and kept in step by every command the table is sent, and answers a seat's request with a network
//! (`cantrip_ai::net`) when asked.
//!
//! A mirror is the same game in the same handle spaces as the table's: the same code run on the same commands in the
//! same order, the views included (a view mints handles). So an answer the network makes is written in the handles the
//! seat's client holds (`Session::payload`), and the table takes it as it would the client's.
//!
//! The protocol is JSON lines like the native host's: one reply per command, in order, and besides them a line of its
//! own for each answer asked for (`AiResult`), written when it is ready, from a worker thread. `serve` is the loop;
//! `AiHost::handle` is one command.

pub mod advisor;
pub mod regret;
pub mod selfplay;

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cantrip_ai::net::{Choice, Payment};
use cantrip_ai::record::table;
use cantrip_ai::search::{self, WallClock};
use cantrip_ai::shard::{mode_name, parse_mode};
use cantrip_ai::{Evaluator, Knowledge, Prior, TractEvaluator};
use cantrip_host::protocol::AnswerPayload;
use cantrip_host::session::{Deal, Rejection, Session};
use cantrip_host::view::{player_at, seat};
use cantrip_obs::InfoMode;
use serde::Deserialize;
use serde_json::{Value, json};

/// Card code recurses, so the loop and the workers run on threads with room (as the native host's does).
pub const STACK_SIZE: usize = 256 << 20;

#[derive(Deserialize)]
#[serde(tag = "Cmd")]
enum Command {
    /// The native host's `NewGame`, and: the table's id for it (`GameId`, else the next of the host's own), whether
    /// every mirrored command is to be answered with the table's whole reply (`Echo`), and the commands to replay at
    /// once (`Replay`), which bring a mirror made late to where the table is.
    NewGame {
        #[serde(rename = "GameId", default)]
        game_id: Option<u32>,
        #[serde(rename = "Seats")]
        seats: Vec<SeatSpec>,
        #[serde(rename = "Seed", default)]
        seed: u64,
        #[serde(rename = "Shuffle", default = "yes")]
        shuffle: bool,
        #[serde(rename = "FirstPlayer", default)]
        first_player: Option<i32>,
        #[serde(rename = "Chooser", default)]
        chooser: Option<i32>,
        #[serde(rename = "BasicLandsOutside", default)]
        basic_lands_outside: Vec<String>,
        /// The format's minimum deck size (`Setup::minimum_deck_size`, CR 100.2a-b), as the native host takes it: a dealt
        /// game says it.
        #[serde(rename = "MinimumDeckSize", default)]
        minimum_deck_size: Option<u32>,
        #[serde(rename = "Scenario", default)]
        scenario: Option<ScenarioSpec>,
        #[serde(rename = "Echo", default)]
        echo: bool,
        #[serde(rename = "Replay", default)]
        replay: Vec<Mirrored>,
    },
    Start {
        #[serde(rename = "GameId")]
        game_id: u32,
    },
    Answer {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Answer")]
        answer: AnswerPayload,
    },
    Concede {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
    },
    View {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
    },
    Close {
        #[serde(rename = "GameId")]
        game_id: u32,
    },
    /// The native host's checks of a selection or a declaration (docs/host-protocol.md): nothing advances, but a check
    /// may mint a seat's handles, so a mirror a person's client plays beside takes them too (the advisor's table does).
    Check {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Selected", default)]
        selected: Vec<cantrip_host::protocol::ViewHandle>,
    },
    CheckBlocks {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Blocks", default)]
        blocks: Vec<cantrip_host::protocol::BlockMessage>,
    },
    CheckAttacks {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Attacks", default)]
        attacks: Vec<cantrip_host::protocol::AttackMessage>,
    },
    /// Answer the request outstanding, which must be `Seat`'s, with the network: replied to at once (`Started`), and
    /// answered by an `AiResult` line when the answer is made.
    AiStart {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        /// A model by its directory's name; the first loaded if none.
        #[serde(rename = "Model", default)]
        model: Option<String>,
        /// A perfect-information value model by its directory's name, asked of each leaf's world as a full state
        /// (`search::search_valued`); none: the leaf values are `Model`'s.
        #[serde(rename = "ValueModel", default)]
        value_model: Option<String>,
        /// `Full`, `Decklist` or `Hidden`; `Full` if none.
        #[serde(rename = "Mode", default)]
        mode: Option<String>,
        /// Milliseconds from now after which no step asks the network; none, no limit.
        #[serde(rename = "Deadline", default)]
        deadline: Option<u64>,
        /// Sample at this temperature instead of taking the best option.
        #[serde(rename = "Temperature", default)]
        temperature: Option<f32>,
        #[serde(rename = "SampleSeed", default)]
        sample_seed: Option<u64>,
        /// Search with this many simulations (`cantrip_ai::search`); 0 or none, and no `SearchMs`, the network alone.
        #[serde(rename = "Simulations", default)]
        simulations: Option<usize>,
        /// Search for at most this many milliseconds from now (`Deadline` stays the limit for the network's answer
        /// when the search makes none).
        #[serde(rename = "SearchMs", default)]
        search_ms: Option<u64>,
        /// The most the seat's search tree may hold, in MiB, its kept part included (`search::Config::memory`); 0, no
        /// bound; none, the default.
        #[serde(rename = "MemoryMb", default)]
        memory_mb: Option<usize>,
        /// Worlds a Decklist or Hidden search deals (Full plays in one); 8 if none.
        #[serde(rename = "Worlds", default)]
        worlds: Option<usize>,
        /// The search's seed; else one from the game and request ids.
        #[serde(rename = "SearchSeed", default)]
        search_seed: Option<u64>,
        /// The search's exploration constant, prior temperature and batch, if not the defaults.
        #[serde(rename = "CPuct", default)]
        c_puct: Option<f32>,
        #[serde(rename = "PriorTemperature", default)]
        prior_temperature: Option<f32>,
        #[serde(rename = "Batch", default)]
        batch: Option<usize>,
        /// Branch trimming, and the thresholds it uses, if not the defaults (`cantrip_ai::search`).
        #[serde(rename = "Prune", default)]
        prune: Option<bool>,
        #[serde(rename = "PruneBad", default)]
        prune_bad: Option<f32>,
        #[serde(rename = "PruneMargin", default)]
        prune_margin: Option<f32>,
        #[serde(rename = "PruneAfter", default)]
        prune_after: Option<u32>,
        #[serde(rename = "PruneAfterReplies", default)]
        prune_after_replies: Option<u32>,
        /// Whether a cut is held only while the trim holds (`search::Config::prune_reversible`), if not the default.
        #[serde(rename = "PruneReversible", default)]
        prune_reversible: Option<bool>,
        /// Go on from the tree kept from this seat's last search (`search::Reuse`): `TopUp` or `Add`; none, afresh.
        #[serde(rename = "Reuse", default)]
        reuse: Option<String>,
        /// The AI's requests atomic (`cantrip_ai::atomic`, on by default), and the bound on a priority's whole answers
        /// (`search::Config::flatten`), if not the defaults.
        /// What a kept tree follows of the other seat's answers (`search::Follow`: Own, Passes, All).
        #[serde(rename = "Follow", default)]
        follow: Option<String>,
        #[serde(rename = "Atomic", default)]
        atomic: Option<bool>,
        #[serde(rename = "Flatten", default)]
        flatten: Option<usize>,
        /// The autotapper before it was told the plan (`search::Config::pay_classic`), and the share within which a cast
        /// is split by class of payment (`search::Config::pay_split`), if not the defaults: for measuring the new one.
        #[serde(rename = "PayClassic", default)]
        pay_classic: Option<bool>,
        #[serde(rename = "PaySplit", default)]
        pay_split: Option<f32>,
        /// The advisor's report of the decision beside the answer (`advisor`, an `Analysis` in the `AiResult`): the
        /// search's root per world, the candidates as payloads, the belief, and the real game's hidden state apart.
        /// Changes no answer; play and self-play never ask it.
        #[serde(rename = "Report", default)]
        report: bool,
    },
    /// The models loaded and the backend.
    Models,
    /// Words for the answer given last in the game (`advisor::describe`), the seat that gave it and the request it
    /// answered: how the advisor names a play a person made through their own client.
    LastAnswer {
        #[serde(rename = "GameId")]
        game_id: u32,
    },
    /// The game so far as a trace (`cantrip_ai::trace`), made from the answers of the mirror's game and checked to deal
    /// and answer again: for the advisor, which keeps it with a session. Only a dealt game has one.
    Trace {
        #[serde(rename = "GameId")]
        game_id: u32,
    },
}

/// A command `Replay` replays: the mirrored commands of a game, without its id.
#[derive(Deserialize)]
#[serde(tag = "Cmd")]
enum Mirrored {
    Start,
    Answer {
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Answer")]
        answer: AnswerPayload,
    },
    Concede {
        #[serde(rename = "Seat")]
        seat: i32,
    },
    View {
        #[serde(rename = "Seat")]
        seat: i32,
    },
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct ScenarioSpec {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Text")]
    text: String,
    #[serde(rename = "Driven", default)]
    driven: Vec<i32>,
}

#[derive(Deserialize)]
struct SeatSpec {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Deck")]
    deck: Vec<String>,
    #[serde(rename = "Sideboard", default)]
    sideboard: Vec<String>,
    /// The companion chosen as the deck was built (CR 702.139a), as the native host takes it.
    #[serde(rename = "Companion", default)]
    companion: Option<String>,
    /// The decklist's name, where the table has one: a trace's deck name.
    #[serde(rename = "DeckName", default)]
    deck_name: Option<String>,
}

/// A model, by the name of the directory it was loaded from.
pub struct Loaded {
    pub name: String,
    pub evaluator: Arc<dyn Evaluator>,
}

/// One mirrored game: the session, and what each seat knows of it (kept up to date as it is asked).
struct Mirror {
    session: Session,
    knowledge: Vec<Knowledge>,
    /// Each seat's payment in progress: a payment is decided at its first request and kept to.
    payments: Vec<Payment>,
    echo: bool,
    /// A dealt game's seed, deck names and decklists, which a trace deals again (`Trace`); none for a scenario.
    deal: Option<(u64, Vec<String>, Vec<Vec<String>>)>,
    /// The request the last mirrored answer answered (`LastAnswer`).
    answered: Option<cantrip_engine::Request>,
    /// Each seat's search tree kept for its next request (`search::Reuse`).
    kept: Vec<Option<search::Kept>>,
}

/// An answer asked for, for a worker.
struct Job {
    game_id: u32,
    mirror: Arc<Mutex<Mirror>>,
    seat: i32,
    request_id: u32,
    model: usize,
    /// The perfect-information value model the leaves are valued by, if any (`search::search_valued`).
    value_model: Option<usize>,
    mode: InfoMode,
    choice: Choice,
    received: Instant,
    deadline: Option<Instant>,
    /// The search's budget and settings (`searches()` false: the network alone).
    search: search::Config,
    prior: Arc<Prior>,
    /// The advisor's report asked for (`advisor`), and the belief model it reads, if the host has one.
    report: bool,
    belief: Option<Arc<TractEvaluator>>,
}

/// Where the lines go: stdout for the binary, a buffer for a test.
pub type Sink = Arc<dyn Fn(&Value) + Send + Sync>;

pub struct AiHost {
    games: HashMap<u32, Arc<Mutex<Mirror>>>,
    next_game: u32,
    models: Arc<Vec<Loaded>>,
    backend: String,
    /// The metagame a Hidden search deals opponents' decks from (`--prior`); empty, Hidden searches are refused.
    prior: Arc<Prior>,
    /// The network whose belief head the advisor's report reads (`with_belief`); none, the report has no belief.
    belief: Option<Arc<TractEvaluator>>,
    jobs: Option<Sender<Job>>,
    workers: Vec<JoinHandle<()>>,
}

fn rejected(r: Rejection) -> Value {
    json!({ "Rejected": { "Code": r.code, "Detail": r.detail } })
}

fn ok(value: Value) -> Value {
    json!({ "Ok": value })
}

impl AiHost {
    /// A host answering with `models` (the first the default) on `workers` threads, writing `AiResult` lines to `sink`.
    pub fn new(models: Vec<Loaded>, backend: &str, workers: usize, sink: Sink) -> AiHost {
        let models = Arc::new(models);
        let (send, receive) = channel::<Job>();
        let receive = Arc::new(Mutex::new(receive));
        let workers = (0..workers.max(1))
            .map(|i| {
                let (receive, models, sink) =
                    (Arc::clone(&receive), Arc::clone(&models), Arc::clone(&sink));
                std::thread::Builder::new()
                    .name(format!("cantrip-ai-host-{i}"))
                    .stack_size(STACK_SIZE)
                    .spawn(move || work(&receive, &models, &sink))
                    .expect("a worker starts")
            })
            .collect();
        AiHost {
            games: HashMap::new(),
            next_game: 1,
            models,
            backend: backend.to_string(),
            prior: Arc::new(Prior::new()),
            belief: None,
            jobs: Some(send),
            workers,
        }
    }

    /// The metagame a Hidden search deals from.
    pub fn with_prior(mut self, prior: Prior) -> AiHost {
        self.prior = Arc::new(prior);
        self
    }

    /// The network whose belief head the advisor's report reads, on the CPU (`advisor`).
    pub fn with_belief(mut self, belief: TractEvaluator) -> AiHost {
        self.belief = Some(Arc::new(belief));
        self
    }

    /// Waits for the answers asked for to be written.
    pub fn finish(mut self) {
        drop(self.jobs.take());
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }

    /// One line of the protocol: its reply.
    pub fn line(&mut self, line: &str) -> Value {
        match serde_json::from_str::<Command>(line) {
            Ok(command) => {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.handle(command)))
                    .unwrap_or_else(|_| {
                        rejected(Rejection::new(
                            "IllegalAnswer",
                            "the engine failed on that request",
                        ))
                    })
            }
            Err(e) => rejected(Rejection::new(
                "InvalidSetup",
                format!("unreadable request: {e}"),
            )),
        }
    }

    fn handle(&mut self, command: Command) -> Value {
        match command {
            Command::NewGame {
                game_id,
                seats,
                seed,
                shuffle,
                first_player,
                chooser,
                basic_lands_outside,
                minimum_deck_size,
                scenario,
                echo,
                replay,
            } => {
                let traced = scenario.is_none().then(|| {
                    (
                        seed,
                        seats.iter().map(|s| s.deck_name.clone().unwrap_or_else(|| s.name.clone())).collect(),
                        seats.iter().map(|s| s.deck.clone()).collect(),
                    )
                });
                let made = match scenario {
                    Some(spec) => Session::scenario(
                        &spec.text,
                        &spec.name,
                        seats.into_iter().map(|s| s.name).collect(),
                        &spec.driven,
                    ),
                    None if minimum_deck_size.is_none() => Err(Rejection::new(
                        "InvalidSetup",
                        "a dealt game says its format's minimum deck size (MinimumDeckSize)",
                    )),
                    None => Session::new(Deal {
                        companions: seats.iter().map(|s| s.companion.clone()).collect(),
                        seats: seats
                            .into_iter()
                            .map(|s| (s.name, s.deck, s.sideboard))
                            .collect(),
                        basic_lands_outside,
                        minimum_deck_size: minimum_deck_size.unwrap_or_default(),
                        seed,
                        shuffle,
                        first_player,
                        chooser,
                    }),
                };
                let mut session = match made {
                    Ok(s) => s,
                    Err(r) => return rejected(r),
                };
                let id = game_id.unwrap_or(self.next_game);
                self.next_game = self.next_game.max(id + 1);
                let mut last = Value::Null;
                for (i, command) in replay.into_iter().enumerate() {
                    let done = match command {
                        Mirrored::Start => Ok(session.step()),
                        Mirrored::Answer {
                            request_id,
                            seat,
                            answer,
                        } => session.answer(request_id, seat, &answer),
                        Mirrored::Concede { seat } => session.concede(seat),
                        Mirrored::View { seat } => session.view(seat).map(|v| json!({ "View": v })),
                    };
                    match done {
                        Ok(v) => last = v,
                        Err(r) => {
                            return rejected(Rejection::new(
                                r.code,
                                format!("replaying command {i}: {}", r.detail),
                            ));
                        }
                    }
                }
                let seats: Vec<Knowledge> = session
                    .game()
                    .state
                    .seats()
                    .map(|p| Knowledge::of(session.game(), p))
                    .collect();
                let players = seats.len();
                self.games.insert(
                    id,
                    Arc::new(Mutex::new(Mirror {
                        session,
                        payments: vec![Payment::default(); seats.len()],
                        knowledge: seats,
                        echo,
                        deal: traced,
                        answered: None,
                        kept: (0..players).map(|_| None).collect(),
                    })),
                );
                let mut reply = json!({ "GameId": id });
                if echo && !last.is_null() {
                    reply["Last"] = last;
                }
                ok(reply)
            }
            Command::Start { game_id } => self.mirror(game_id, |s| Ok(s.step())),
            Command::Answer {
                game_id,
                request_id,
                seat,
                answer,
            } => {
                // The answer actually given is what the seat's cancel guard learns from, whoever gave it. A cancel is
                // noted before it is applied: the undo takes back the objects of the cast it names.
                use cantrip_host::protocol::{ActionKindView, AnswerPayload};
                let cancel = match &answer {
                    AnswerPayload::CancelAnnouncementAnswer(_) => Some(cantrip_engine::Answer::Cancel),
                    AnswerPayload::TakeActionAnswer(a) if a.kind == ActionKindView::CancelCasting => Some(cantrip_engine::Answer::Act(
                        cantrip_engine::Action::of(&cantrip_engine::ActionOption::simple(cantrip_engine::ActionKind::CancelCasting)),
                    )),
                    _ => None,
                };
                let board = self.games.get(&game_id).map(|m| {
                    let mut m = m.lock().unwrap_or_else(|e| e.into_inner());
                    m.answered = m.session.pending_request();
                    let board = cantrip_ai::cancel::board_hash(&m.session.game().state);
                    if let Some(cancel) = &cancel {
                        let Mirror { session, payments, answered, .. } = &mut *m;
                        if let Some(request) = answered.as_ref()
                            && request.id == request_id
                            && let Some(payment) = payments.get_mut(request.player.index())
                        {
                            let game = session.game();
                            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| payment.played(game, board, request, cancel)));
                        }
                    }
                    board
                });
                let reply = self.mirror(game_id, |s| s.answer(request_id, seat, &answer));
                if cancel.is_none()
                    && reply.get("Ok").is_some()
                    && let (Some(board), Some(m)) = (board, self.games.get(&game_id))
                {
                    let mut m = m.lock().unwrap_or_else(|e| e.into_inner());
                    let Mirror { session, payments, answered, .. } = &mut *m;
                    let game = session.game();
                    if let (Some((id, player, given)), Some(request)) = (game.tree.answer(game.at), answered.as_ref())
                        && *id == request.id
                        && let Some(payment) = payments.get_mut(player.index())
                    {
                        // Bookkeeping of the AI's own: never what the mirror replies.
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| payment.played(game, board, request, given)));
                    }
                }
                reply
            }
            Command::LastAnswer { game_id } => {
                let Some(mirror) = self.games.get(&game_id) else {
                    return rejected(Rejection::new("TableClosed", format!("no game {game_id}")));
                };
                let m = mirror.lock().unwrap_or_else(|e| e.into_inner());
                let game = m.session.game();
                match (game.tree.answer(game.at), &m.answered) {
                    (Some((id, player, answer)), Some(request)) if request.id == *id => ok(json!({
                        "RequestId": id,
                        "Seat": player.0 + 1,
                        "Label": advisor::describe(game, request, answer),
                    })),
                    _ => ok(json!({})),
                }
            }
            Command::Concede { game_id, seat } => self.mirror(game_id, |s| s.concede(seat)),
            Command::Check { game_id, request_id, seat, selected } => {
                self.mirror(game_id, |s| s.check(request_id, seat, &selected))
            }
            Command::CheckBlocks { game_id, request_id, seat, blocks } => {
                self.mirror(game_id, |s| s.check_blocks(request_id, seat, &blocks))
            }
            Command::CheckAttacks { game_id, request_id, seat, attacks } => {
                self.mirror(game_id, |s| s.check_attacks(request_id, seat, &attacks))
            }
            Command::View { game_id, seat } => {
                self.mirror(game_id, |s| s.view(seat).map(|v| json!({ "View": v })))
            }
            Command::Close { game_id } => {
                self.games.remove(&game_id);
                ok(json!({}))
            }
            Command::Trace { game_id } => {
                let Some(mirror) = self.games.get(&game_id) else {
                    return rejected(Rejection::new("TableClosed", format!("no game {game_id}")));
                };
                let m = mirror.lock().unwrap_or_else(|e| e.into_inner());
                let Some((seed, decks, lists)) = m.deal.clone() else {
                    return rejected(Rejection::new("InvalidSetup", "a scenario has no trace"));
                };
                match advisor::trace_of(m.session.game(), seed, decks, lists) {
                    Ok(trace) => ok(json!({ "Trace": trace })),
                    Err(e) => rejected(Rejection::new("InvalidSetup", e)),
                }
            }
            Command::Models => ok(json!({
                "Backend": self.backend,
                "Models": self.models.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
            })),
            Command::AiStart {
                value_model,
                game_id,
                seat: seat_number,
                model,
                mode,
                deadline,
                temperature,
                sample_seed,
                simulations,
                search_ms,
                memory_mb,
                worlds,
                search_seed,
                c_puct,
                prior_temperature,
                batch,
                prune,
                prune_bad,
                prune_margin,
                prune_after,
                prune_after_replies,
                prune_reversible,
                reuse,
                follow,
                atomic,
                flatten,
                pay_classic,
                pay_split,
                report,
            } => {
                let received = Instant::now();
                let Some(mirror) = self.games.get(&game_id).cloned() else {
                    return rejected(Rejection::new("TableClosed", format!("no game {game_id}")));
                };
                let model = match model {
                    None => 0,
                    Some(name) => match self.models.iter().position(|m| m.name == name) {
                        Some(i) => i,
                        None => {
                            return rejected(Rejection::new(
                                "InvalidSetup",
                                format!("no model {name}"),
                            ));
                        }
                    },
                };
                let value_model = match value_model {
                    None => None,
                    Some(name) => match self.models.iter().position(|m| m.name == name) {
                        Some(i) => Some(i),
                        None => return rejected(Rejection::new("InvalidSetup", format!("no value model {name}"))),
                    },
                };
                if self.models.is_empty() {
                    return rejected(Rejection::new("InvalidSetup", "no model is loaded"));
                }
                let mode = match mode.as_deref().map(parse_mode) {
                    None => InfoMode::Full,
                    Some(Some(m)) => m,
                    Some(None) => {
                        return rejected(Rejection::new(
                            "InvalidSetup",
                            format!("no information mode {}", mode.unwrap_or_default()),
                        ));
                    }
                };
                let request_id = {
                    let m = mirror.lock().unwrap_or_else(|e| e.into_inner());
                    match m.session.pending_request() {
                        Some(r) if seat(r.player) == seat_number => r.id,
                        Some(r) => {
                            return rejected(Rejection::new(
                                "NotYourTurnToAnswer",
                                format!("request {} belongs to seat {}", r.id, seat(r.player)),
                            ));
                        }
                        None => {
                            return rejected(Rejection::new(
                                "StaleReply",
                                "no request is outstanding",
                            ));
                        }
                    }
                };
                let choice = match temperature {
                    Some(t) => Choice::Sample {
                        temperature: t,
                        seed: sample_seed.unwrap_or(request_id as u64),
                    },
                    None => Choice::Best,
                };
                let deadline = deadline.map(|ms| received + Duration::from_millis(ms));
                let search_deadline = search_ms
                    .filter(|&ms| ms > 0)
                    .map(|ms| received + Duration::from_millis(ms));
                let defaults = search::Config::default();
                let search = search::Config {
                    simulations: simulations.filter(|&n| n > 0),
                    // The search stops by the earlier of its own budget and the hard limit.
                    deadline: match (search_deadline, simulations.filter(|&n| n > 0)) {
                        (Some(d), _) => Some(deadline.map_or(d, |h| h.min(d))),
                        (None, Some(_)) => deadline,
                        (None, None) => None,
                    },
                    network_deadline: deadline,
                    memory: match memory_mb {
                        None => defaults.memory,
                        Some(0) => None,
                        Some(mb) => Some(mb << 20),
                    },
                    worlds: worlds.filter(|&w| w > 0).unwrap_or(defaults.worlds),
                    c_puct: c_puct.unwrap_or(defaults.c_puct),
                    prior_temperature: prior_temperature.unwrap_or(defaults.prior_temperature),
                    batch: batch.filter(|&b| b > 0).unwrap_or(defaults.batch),
                    seed: search_seed
                        .unwrap_or(((game_id as u64) << 32) ^ request_id as u64 ^ 0x5ea2c4),
                    choice,
                    prune: prune.unwrap_or(defaults.prune),
                    prune_bad: prune_bad.unwrap_or(defaults.prune_bad),
                    prune_margin: prune_margin.unwrap_or(defaults.prune_margin),
                    prune_after: prune_after.unwrap_or(defaults.prune_after),
                    prune_after_replies: prune_after_replies
                        .unwrap_or(defaults.prune_after_replies),
                    prune_reversible: prune_reversible.unwrap_or(defaults.prune_reversible),
                    atomic: atomic.unwrap_or(defaults.atomic),
                    pay_classic: pay_classic.unwrap_or(defaults.pay_classic),
                    pay_split: pay_split.unwrap_or(defaults.pay_split),
                    reuse_follow: match follow.as_deref() {
                        None => defaults.reuse_follow,
                        Some("Own") => search::Follow::Own,
                        Some("Passes") => search::Follow::Passes,
                        Some("All") => search::Follow::All,
                        Some(other) => {
                            return rejected(Rejection::new("InvalidSetup", format!("no follow {other}: Own, Passes or All")));
                        }
                    },
                    flatten: flatten.unwrap_or(defaults.flatten),
                    reuse: match reuse.as_deref() {
                        None => defaults.reuse,
                        Some("None") => search::Reuse::None,
                        Some("TopUp") => search::Reuse::TopUp,
                        Some("Add") => search::Reuse::Add,
                        Some(other) => {
                            return rejected(Rejection::new("InvalidSetup", format!("no reuse {other}: TopUp, Add or None")));
                        }
                    },
                    report,
                    ..defaults
                };
                if search.searches() && mode == InfoMode::Hidden && self.prior.is_empty() {
                    return rejected(Rejection::new(
                        "InvalidSetup",
                        "a Hidden search needs the metagame's decks (--prior DIR)",
                    ));
                }
                let job = Job {
                    game_id,
                    mirror,
                    seat: seat_number,
                    request_id,
                    model,
                    value_model,
                    mode,
                    choice,
                    received,
                    deadline,
                    search,
                    prior: Arc::clone(&self.prior),
                    report,
                    belief: self.belief.clone(),
                };
                if let Some(jobs) = &self.jobs {
                    let _ = jobs.send(job);
                }
                ok(json!({ "Started": { "RequestId": request_id, "Mode": mode_name(mode) } }))
            }
        }
    }

    /// A mirrored command: run on the mirror as the table runs it, its reply the table's whole reply if the game echoes,
    /// else only whether it was taken.
    fn mirror(
        &mut self,
        game_id: u32,
        f: impl FnOnce(&mut Session) -> Result<Value, Rejection>,
    ) -> Value {
        let Some(mirror) = self.games.get(&game_id) else {
            return rejected(Rejection::new("TableClosed", format!("no game {game_id}")));
        };
        let mut m = mirror.lock().unwrap_or_else(|e| e.into_inner());
        let echo = m.echo;
        match f(&mut m.session) {
            Ok(v) if echo => ok(v),
            Ok(_) => ok(json!({})),
            Err(r) => rejected(r),
        }
    }
}

fn work(receive: &Mutex<Receiver<Job>>, models: &[Loaded], sink: &Sink) {
    loop {
        let job = receive.lock().map(|r| r.recv());
        let Ok(Ok(job)) = job else { break };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| answer_job(&job, models)))
            .unwrap_or_else(|_| json!({ "GameId": job.game_id, "Seat": job.seat, "RequestId": job.request_id, "Error": "the network's answer panicked" }));
        sink(&json!({ "AiResult": result }));
    }
}

/// The `AiResult` of a job: the network's answer, or the prompt's default answer if it made none (`Fallback`).
fn answer_job(job: &Job, models: &[Loaded]) -> Value {
    let started = Instant::now();
    let mut m = job.mirror.lock().unwrap_or_else(|e| e.into_inner());
    let Mirror {
        session,
        knowledge,
        payments,
        kept,
        ..
    } = &mut *m;
    let base = json!({ "GameId": job.game_id, "Seat": job.seat, "RequestId": job.request_id });
    let request = match session.pending_request() {
        Some(r) if r.id == job.request_id => r,
        _ => {
            let mut v = base;
            v["Error"] = json!("the request was answered before the network was asked it");
            return v;
        }
    };
    let Some(p) = player_at(job.seat) else {
        return base;
    };
    let k = &mut knowledge[p.index()];
    k.update(session.game());
    payments[p.index()].atomic = job.search.atomic;
    payments[p.index()].classic = job.search.pay_classic;
    let evaluator = models[job.model].evaluator.as_ref();
    let value = job.value_model.map(|i| models[i].evaluator.as_ref());
    let decided = if job.search.searches() {
        let game = session.game();
        let keep = &mut kept[p.index()];
        payments[p.index()].decide(game, &request, k, job.mode, |request, guard| {
            search::search_valued(
                game,
                request,
                k,
                job.mode,
                table(),
                evaluator,
                value,
                &job.search,
                &job.prior,
                guard,
                &WallClock,
                keep,
            )
        })
    } else {
        payments[p.index()].answer_by(
            session.game(),
            &request,
            k,
            job.mode,
            table(),
            evaluator,
            job.choice,
            job.deadline,
        )
    };
    let analysis = job
        .report
        .then(|| advisor::analysis(session, &request, k, job.mode, &decided, job.belief.as_deref()));
    let mut v = base;
    if let Some(a) = analysis {
        v["Analysis"] = a;
    }
    match decided {
        Ok(d) => {
            let (payload, minted) = session.payload(&d.answer, &request);
            v["Answer"] = serde_json::to_value(payload).expect("an answer serialises");
            v["Factored"] = json!(format!("{:?}", d.factored));
            v["Fallback"] = json!(d.fallback);
            v["Late"] = json!(d.late);
            v["Minted"] = json!(minted);
            v["Value"] = json!(d.value);
            v["Evaluations"] = json!(d.evaluations);
            v["Steps"] = json!(d.steps);
            v["Rejected"] = json!(d.rejected.len());
            v["EncodeUs"] = json!(d.encode.as_micros() as u64);
            v["EvaluateUs"] = json!(d.evaluate.as_micros() as u64);
            if let Some(s) = &d.search {
                v["Search"] = json!({
                    "Simulations": s.simulations,
                    "Evaluations": s.evaluations,
                    "Batches": s.batches,
                    "Nodes": s.nodes,
                    "Positions": s.positions,
                    "GameNodes": s.game_nodes,
                    "TreeBytes": s.tree_bytes,
                    "Counted": s.counted,
                    "MemoryBound": s.memory_bound,
                    "KeptDropped": s.kept_dropped,
                    "RootValue": s.root_value,
                    "NetValue": s.net_value,
                    "Worlds": s.worlds,
                    "Shortcut": s.shortcut,
                    "Refused": s.refused,
                    "Pruned": s.pruned,
                    "PrunedImmediate": s.pruned_immediate,
                    "PrunedShallow": s.pruned_shallow,
                    "LineDecisions": s.shape.line_decisions,
                    "LineTurns": s.shape.line_turns,
                    "DeepestDecisions": s.shape.deepest_decisions,
                    "DeepestTurns": s.shape.deepest_turns,
                    "SearchUs": s.elapsed.as_micros() as u64,
                });
            }
        }
        Err(e) => {
            v["Error"] = json!(e.0);
            v["Fallback"] = json!(true);
            if let Some(payload) = session.default_payload() {
                v["Answer"] = serde_json::to_value(payload).expect("an answer serialises");
            }
        }
    }
    v["QueuedUs"] = json!((started - job.received).as_micros() as u64);
    v["TotalUs"] = json!(job.received.elapsed().as_micros() as u64);
    v
}
