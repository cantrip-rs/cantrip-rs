//! The host's requests (docs/host-protocol.md) and the games they are about: what the stdio sidecar (`bin/host.rs`)
//! serves and the browser's engine (`cantrip-wasm`) serves, one JSON request in and one JSON reply out. The stdio host
//! keeps its recorder of games for the AI player around this; nothing here records.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::protocol::{AnswerPayload, AttackMessage, BlockMessage, ViewHandle};
use crate::session::{Deal, Rejection, SeatVariantSpec, Session};
use crate::view::printed;

#[derive(Deserialize)]
#[serde(tag = "Cmd")]
pub enum Request {
    NewGame {
        #[serde(rename = "Seats")]
        seats: Vec<SeatSpec>,
        #[serde(rename = "Seed")]
        seed: u64,
        #[serde(rename = "Shuffle", default = "yes")]
        shuffle: bool,
        #[serde(rename = "FirstPlayer", default)]
        first_player: Option<i32>,
        #[serde(rename = "Chooser", default)]
        chooser: Option<i32>,
        /// A Limited game's basic lands, which each sideboard holds any number of (`Setup::basic_lands_outside`).
        #[serde(rename = "BasicLandsOutside", default)]
        basic_lands_outside: Vec<String>,
        /// The format's minimum deck size, which the server knows from the table's format (`Setup::minimum_deck_size`, CR
        /// 100.2a-b): always said, never inferred. A scenario's is its own.
        #[serde(rename = "MinimumDeckSize", default)]
        minimum_deck_size: Option<u32>,
        /// Explicit format rule switch; a 100-card deck alone does not make Commander.
        #[serde(rename = "Commander", default)]
        commander: bool,
        /// A `.cantrip` scenario to play instead of a dealt game: its decks, order and dice are the scenario's, and the
        /// seats named in `Driven` are answered by the table, every other by the script (docs/scenarios-in-the-browser.md).
        #[serde(rename = "Scenario", default)]
        scenario: Option<ScenarioSpec>,
        /// Whether each step also projects the game for a viewer who is no player (`Public`), for an in-person table's
        /// shared boards (docs/host-protocol.md, "The public view"). Off, nothing of it is worked out.
        #[serde(rename = "Public", default)]
        public: bool,
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
    /// What the rule of an outstanding choice of cards says of a partial selection (docs/host-protocol.md, "Checking a
    /// selection"): nothing advances.
    Check {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Selected", default)]
        selected: Vec<ViewHandle>,
    },
    /// What the rules make of a declaration of blockers for an outstanding declaration (docs/host-protocol.md, "Checking a
    /// declaration of blockers"): nothing advances.
    CheckBlocks {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Blocks", default)]
        blocks: Vec<BlockMessage>,
    },
    /// What the rules make of a declaration of attackers for an outstanding declaration (docs/host-protocol.md, "Checking a
    /// declaration of attackers"): nothing advances.
    CheckAttacks {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Attacks", default)]
        attacks: Vec<AttackMessage>,
    },
    Concede {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
    },
    /// The game ends from outside its rules: won by `Winner`'s seat, or a draw with none.
    End {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Winner", default)]
        winner: Option<i32>,
    },
    View {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
    },
    Oracle {
        #[serde(rename = "Name")]
        name: String,
    },
    /// Which cards with companion in a sideboard the deck fulfils the condition of (CR 702.139a), for the deck editor.
    Companions {
        #[serde(rename = "Deck")]
        deck: Vec<String>,
        #[serde(rename = "Sideboard", default)]
        sideboard: Vec<String>,
        #[serde(rename = "MinimumDeckSize")]
        minimum_deck_size: u32,
    },
    Close {
        #[serde(rename = "GameId")]
        game_id: u32,
    },
    /// A recording host only: a heuristic's value of the outstanding request's position for its seat (`Heuristic`, the
    /// tactical bot's model in log-odds), recorded with the game (`cantrip_ai::record::Dealt::heuristics`). Changes
    /// nothing in the game.
    #[cfg(feature = "ai-record")]
    Note {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Heuristic")]
        heuristic: f32,
    },
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
pub struct ScenarioSpec {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Text")]
    pub text: String,
    #[serde(rename = "Driven", default)]
    pub driven: Vec<i32>,
}

#[derive(Deserialize)]
pub struct SeatSpec {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Deck")]
    pub deck: Vec<String>,
    /// The cards the seat owns outside the game (CR 100.4, 400.11a).
    #[serde(rename = "Sideboard", default)]
    pub sideboard: Vec<String>,
    /// The companion chosen as the deck was built (CR 702.139a): dealt into the sideboard beside it, and revealed before the
    /// game if the deck fulfils its condition (CR 103.2b). The server has refused a deck whose companion it doesn't.
    #[serde(rename = "Companion", default)]
    pub companion: Option<String>,
    /// Commanders are outside the library.
    #[serde(rename = "Commanders", default)]
    pub commanders: Vec<String>,
    /// What a variant sets of the seat's start (`SeatVariantSpec`): its starting life, its hand sizes and the emblems it
    /// begins the game with (CR 103.4, 103.5a, 902.3). Absent, the format's.
    #[serde(rename = "StartingLife", default)]
    pub starting_life: Option<i32>,
    #[serde(rename = "StartingHandSize", default)]
    pub starting_hand_size: Option<i32>,
    #[serde(rename = "MaximumHandSize", default)]
    pub maximum_hand_size: Option<i32>,
    #[serde(rename = "Emblems", default)]
    pub emblems: Vec<String>,
    /// The deck's name, where the table has one: the recorder's archetype label.
    #[serde(rename = "DeckName", default)]
    #[cfg_attr(not(feature = "ai-record"), allow(dead_code))]
    pub deck_name: Option<String>,
}

impl Request {
    /// The game a request is about, for one that names one.
    pub fn game_id(&self) -> Option<u32> {
        match self {
            Request::Start { game_id }
            | Request::Answer { game_id, .. }
            | Request::Check { game_id, .. }
            | Request::CheckBlocks { game_id, .. }
            | Request::CheckAttacks { game_id, .. }
            | Request::Concede { game_id, .. }
            | Request::End { game_id, .. }
            | Request::View { game_id, .. }
            | Request::Close { game_id } => Some(*game_id),
            #[cfg(feature = "ai-record")]
            Request::Note { game_id, .. } => Some(*game_id),
            Request::NewGame { .. } | Request::Oracle { .. } | Request::Companions { .. } => None,
        }
    }
}

/// A refusal as a reply: `{"Rejected":{"Code":..,"Detail":..}}`.
pub fn rejected(r: Rejection) -> Value {
    json!({ "Rejected": { "Code": r.code, "Detail": r.detail } })
}

/// A success as a reply: `{"Ok":..}`.
pub fn ok(value: Value) -> Value {
    json!({ "Ok": value })
}

/// A request read from its line, or the refusal of an unreadable one.
pub fn parse(line: &str) -> Result<Request, Value> {
    serde_json::from_str::<Request>(line).map_err(|e| rejected(Rejection::new("InvalidSetup", format!("unreadable request: {e}"))))
}

/// What `handle` replied, or a refusal if the engine failed on the request. Where a panic aborts (the browser's engine),
/// a failure ends the engine instead, for the page to report.
pub fn guarded(handle: impl FnOnce() -> Value) -> Value {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(handle))
        .unwrap_or_else(|_| rejected(Rejection::new("IllegalAnswer", "the engine failed on that request")))
}

/// The games being played, by id, and what each request does to them.
pub struct Tables {
    games: HashMap<u32, Session>,
    next_game: u32,
}

impl Default for Tables {
    fn default() -> Tables {
        Tables { games: HashMap::new(), next_game: 1 }
    }
}

impl Tables {
    /// One request line answered: the reply line.
    pub fn call(&mut self, line: &str) -> String {
        let reply = match parse(line) {
            Ok(request) => guarded(|| self.handle(request)),
            Err(refused) => refused,
        };
        serde_json::to_string(&reply).expect("a reply serialises")
    }

    pub fn get(&self, game_id: u32) -> Option<&Session> {
        self.games.get(&game_id)
    }

    /// Takes a game away, as closing it does.
    pub fn remove(&mut self, game_id: u32) -> Option<Session> {
        self.games.remove(&game_id)
    }

    pub fn handle(&mut self, request: Request) -> Value {
        match request {
            Request::NewGame { seats, seed, shuffle, first_player, chooser, basic_lands_outside, minimum_deck_size, commander, scenario, public } => {
                let made = match scenario {
                    Some(spec) => Session::scenario(&spec.text, &spec.name, seats.into_iter().map(|s| s.name).collect(), &spec.driven),
                    None if minimum_deck_size.is_none() => {
                        Err(Rejection::new("InvalidSetup", "a dealt game says its format's minimum deck size (MinimumDeckSize)"))
                    }
                    None => {
                        let companions = seats.iter().map(|s| s.companion.clone()).collect();
                        let commanders = seats.iter().map(|s| s.commanders.clone()).collect();
                        let variant = seats
                            .iter()
                            .map(|s| SeatVariantSpec {
                                starting_life: s.starting_life,
                                starting_hand_size: s.starting_hand_size,
                                maximum_hand_size: s.maximum_hand_size,
                                emblems: s.emblems.clone(),
                            })
                            .collect();
                        let deal = Deal {
                            seats: seats.into_iter().map(|s| (s.name, s.deck, s.sideboard)).collect(),
                            basic_lands_outside,
                            companions,
                            commanders,
                            commander,
                            minimum_deck_size: minimum_deck_size.unwrap_or_default(),
                            seed,
                            shuffle,
                            first_player,
                            chooser,
                            variant,
                        };
                        Session::new(deal)
                    }
                };
                match made {
                    Ok(mut session) => {
                        if public {
                            session.show_public();
                        }
                        let id = self.next_game;
                        self.next_game += 1;
                        let decklists = session.decklists();
                        self.games.insert(id, session);
                        ok(json!({ "GameId": id, "Decklists": decklists }))
                    }
                    Err(r) => rejected(r),
                }
            }
            Request::Start { game_id } => self.with(game_id, |g| Ok(g.step())),
            Request::Answer { game_id, request_id, seat, answer } => self.with(game_id, |g| g.answer(request_id, seat, &answer)),
            Request::Check { game_id, request_id, seat, selected } => self.with(game_id, |g| g.check(request_id, seat, &selected)),
            Request::CheckBlocks { game_id, request_id, seat, blocks } => self.with(game_id, |g| g.check_blocks(request_id, seat, &blocks)),
            Request::CheckAttacks { game_id, request_id, seat, attacks } => self.with(game_id, |g| g.check_attacks(request_id, seat, &attacks)),
            Request::Concede { game_id, seat } => self.with(game_id, |g| g.concede(seat)),
            Request::End { game_id, winner } => self.with(game_id, |g| g.end(winner)),
            Request::View { game_id, seat } => self.with(game_id, |g| g.view(seat).map(|v| json!({ "View": v }))),
            Request::Oracle { name } => {
                let card = cantrip_engine::card(&name).map(|id| printed(cantrip_engine::cards().get(id)));
                ok(json!({ "Card": card }))
            }
            Request::Companions { deck, sideboard, minimum_deck_size } => match crate::session::companions(&deck, &sideboard, minimum_deck_size) {
                Ok(checks) => ok(json!({ "Companions": checks })),
                Err(r) => rejected(r),
            },
            // Noted only by a recording host, which answers it before it gets here.
            #[cfg(feature = "ai-record")]
            Request::Note { .. } => ok(json!({})),
            Request::Close { game_id } => {
                self.games.remove(&game_id);
                ok(json!({}))
            }
        }
    }

    fn with(&mut self, game_id: u32, f: impl FnOnce(&mut Session) -> Result<Value, Rejection>) -> Value {
        match self.games.get_mut(&game_id) {
            Some(session) => match f(session) {
                Ok(v) => ok(v),
                Err(r) => rejected(r),
            },
            None => rejected(Rejection::new("TableClosed", format!("no game {game_id}"))),
        }
    }
}
