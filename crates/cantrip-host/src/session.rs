//! One hosted game: the engine, a handle space and an event stream per seat, and the one question outstanding.
//!
//! The engine asks one question at a time and its request ids are never reused, so a request id is the engine's own: an
//! answer to an old id is `StaleReply`, an answer from the wrong seat `NotYourTurnToAnswer`.
//!
//! # What is published, and when
//!
//! A frame that asks a question is re-run from its start once it is answered, and its log is cut back to where the run
//! began (`Game::answer`). So only the log up to the start of the run that is waiting is published; the entries of a
//! run in progress (a spell being announced and paid for) appear when it completes, which is also what the C# engine
//! tells: nothing about an announcement is told until the spell is paid for. The views say no more than the events: while
//! a run is in progress, every seat but the one answering keeps the view it was last sent, so an opponent sees no spell
//! being announced, no land tapped to pay for it, and nothing when a cast that cannot be paid for is taken back.

use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;

use cantrip_engine::undo::Undo;
use cantrip_script::{Advance, Expectation, Expected, GameScript, Intent, Scripted};
use cantrip_engine::*;
use serde_json::{Value, json};

use crate::answer::{Untranslatable, payload_of, translate};
use crate::events::{Proto, Teller, Tracker};
use crate::handles::HandleSpace;
use crate::prompt::prompt;
use crate::protocol::*;
use crate::view::*;

/// A refusal, with the code `PhaseRulesHost` maps to its `ErrorCode`.
pub struct Rejection {
    pub code: &'static str,
    pub detail: String,
}

impl Rejection {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Rejection {
        Rejection { code, detail: detail.into() }
    }
}

pub struct Session {
    game: Game,
    names: Vec<String>,
    decklists: Vec<Vec<ObjectView>>,
    /// One handle space per seat, index 0 for seat 1.
    handles: Vec<HandleSpace>,
    tracker: Tracker,
    /// How far through the engine's log everything has been published.
    published: usize,
    /// Game-wide event numbering, shared by every seat's copy of an event.
    sequence: u64,
    conceded: Vec<bool>,
    /// What happened that no engine event says: the answers to the opening's questions.
    told: Vec<Proto>,
    /// What each seat is being shown beyond its zones, as of the last step.
    extra: Vec<HashSet<ObjectId>>,
    /// Of what each seat is being shown, as of the last step, the cards whose face it is shown: those revealed (CR 701.20a)
    /// and those it looked at (CR 701.20e), not what its question names. A face-down permanent among them is shown as what it
    /// really is for as long as that lasts, though it stays face down (CR 701.40g, 701.58g).
    faces: Vec<HashSet<ObjectId>>,
    /// The view each seat was last sent: what a seat other than the one answering keeps while a run is in progress.
    sent: Vec<Option<PlayerView>>,
    /// The mana abilities that can still be taken back (`cantrip_engine::undo`).
    undoable: Undo<Told>,
    /// A scenario being played at this table (`Session::scenario`), or none for a game dealt from decks.
    scenario: Option<Scenario>,
    /// The game as a viewer who is no player sees it (`PUBLIC`), for an in-person table's shared boards
    /// (docs/in-person.md), or none: nothing of it is worked out unless the game was dealt with it (`Session::show_public`).
    public: Option<PublicSide>,
}

/// The public's side of a game: its own handle space, and the view it was last sent, which it keeps while any seat's run is
/// in progress (`deferred`), since no run is ever its to see.
struct PublicSide {
    handles: HandleSpace,
    sent: Option<PlayerView>,
}

/// A `.cantrip` scenario played at a table (docs/scenarios-in-the-browser.md): the script answers for every seat but the
/// driven ones, and says what a driven seat is meant to answer, which it must. Its runner keeps a game of its own, which
/// the session swaps with the live one for each move (`std::mem::swap`), so each is the one game in turn.
struct Scenario {
    scripted: Scripted,
    driven: Vec<PlayerId>,
    /// The question a driven seat is asked now, and what the script means.
    expected: Option<Expected>,
    /// Checks of what a client shows that came up since the last step, for the driving pages.
    checks: Vec<Expectation>,
    /// How it ended: `Ok` with what the pages are still to judge, or the failure's report.
    verdict: Option<Result<Vec<Expectation>, String>>,
}

/// What the host keeps beside a mana ability that can be taken back (`cantrip_engine::undo`): what was told of it, and
/// the tracking and handles as they were before it.
struct Told {
    tracker: Tracker,
    /// Each seat's handles: a sacrificed permanent's handle moved to the card in the graveyard, which taking it back
    /// unmakes.
    handles: Vec<HandleSpace>,
    /// The public's handles, when the game has a public view.
    public: Option<HandleSpace>,
    published: usize,
    /// The sequences of the entries told of it.
    told: std::ops::Range<u64>,
}

/// The cards with companion in `sideboard`, in its order, each with whether `deck` - a starting deck, its cards by name -
/// fulfils its condition (CR 702.139a-b, 103.2b), `minimum` being the format's minimum deck size: the rules' own judgement of
/// what a deck could reveal, for the deck editor. A card the engine does not have is refused, as a deal refuses it.
pub fn companions(deck: &[String], sideboard: &[String], minimum: u32) -> Result<Vec<CompanionCheckView>, Rejection> {
    let cards = cards();
    let id = |name: &String| {
        card(name)
            .map(|id| cards.fronts[id.0 as usize].unwrap_or(id))
            .ok_or_else(|| Rejection::new("InvalidSetup", format!("{name} is not a card the engine has")))
    };
    let deck = deck.iter().map(id).collect::<Result<Vec<_>, _>>()?;
    let outside = sideboard.iter().map(id).collect::<Result<Vec<_>, _>>()?;
    let setup = Setup { decks: vec![deck, Vec::new()], sideboards: vec![outside], minimum_deck_size: minimum, shuffle: false, ..Setup::default() };
    let game = Game::new(cards, &setup);
    let player = PlayerId(0);
    let fulfilled = eligible_companions(&game, player);
    Ok(game
        .state
        .zone(Zone::sideboard(player))
        .iter()
        .filter(|&&id| cards.get(game.state.obj(id).card).companion.is_some())
        .map(|&id| CompanionCheckView { card: game.name(id).to_string(), fulfilled: fulfilled.contains(&id) })
        .collect())
}

/// Dealing a game: the seats' names and decks, and the conditions beyond the seed.
pub struct Deal {
    /// Each seat's name, deck (top first) and sideboard: the cards it owns outside the game (CR 100.4, 400.11a).
    pub seats: Vec<(String, Vec<String>, Vec<String>)>,
    /// A Limited game's basic lands, which each sideboard holds any number of (`Setup::basic_lands_outside`).
    pub basic_lands_outside: Vec<String>,
    /// Each seat's companion, chosen as its deck was built (`Setup::companions`), by seat; a seat past the end has none.
    pub companions: Vec<Option<String>>,
    /// The format's minimum deck size, as the server says (`Setup::minimum_deck_size`, CR 100.2a-b).
    pub minimum_deck_size: u32,
    pub seed: u64,
    pub shuffle: bool,
    pub first_player: Option<i32>,
    pub chooser: Option<i32>,
}

impl Session {
    pub fn new(deal: Deal) -> Result<Session, Rejection> {
        Session::prepared(deal, |_| {})
    }

    /// A dealt game, `prepare`d before it begins: a test's way into a position no deal reaches.
    fn prepared(deal: Deal, prepare: impl FnOnce(&mut Game)) -> Result<Session, Rejection> {
        if deal.seats.len() < 2 {
            return Err(Rejection::new("InvalidSetup", "a game needs at least two seats"));
        }
        if deal.seats.len() > MAX_SEATS {
            return Err(Rejection::new("InvalidSetup", format!("a game seats at most {MAX_SEATS}")));
        }
        let cards = cards();
        let known = |name: &str, card_name: &str| {
            card(card_name).ok_or_else(|| Rejection::new("InvalidSetup", format!("{name}'s deck names {card_name}, which is not a card the engine has")))
        };
        let mut decks = Vec::new();
        let mut sideboards = Vec::new();
        let mut decklists = Vec::new();
        for (name, deck, sideboard) in &deal.seats {
            // A sideboard is the cards the seat owns outside the game (CR 400.11a), which a card may bring in: a card the
            // engine does not have refuses the deal as one in the deck does.
            let mut outside = Vec::new();
            for card_name in sideboard {
                let id = card(card_name)
                    .ok_or_else(|| Rejection::new("InvalidSetup", format!("{name}'s sideboard names {card_name}, which is not a card the engine has")))?;
                outside.push(cards.fronts[id.0 as usize].unwrap_or(id));
            }
            sideboards.push(outside);
            let mut list = Vec::new();
            let mut shown = Vec::new();
            for card_name in deck {
                let id = known(name, card_name)?;
                let front = cards.fronts[id.0 as usize].unwrap_or(id);
                shown.push(printed(cards.get(front)));
                list.push(front);
            }
            decks.push(list);
            decklists.push(shown);
        }
        let chooser = deal.chooser.and_then(player_at);
        let first = deal.first_player.and_then(player_at);
        let opening_chooser = chooser.or(first.filter(|_| !deal.shuffle));
        let basic_lands_outside = deal
            .basic_lands_outside
            .iter()
            .map(|c| card(c).ok_or_else(|| Rejection::new("InvalidSetup", format!("the basic land {c} is not a card the engine has"))))
            .collect::<Result<Vec<_>, _>>()?;
        let mut companions = Vec::new();
        for (at, companion) in deal.companions.iter().enumerate() {
            let name = deal.seats.get(at).map_or("", |s| s.0.as_str());
            companions.push(match companion {
                None => None,
                Some(c) => Some(card(c).map(|id| cards.fronts[id.0 as usize].unwrap_or(id)).ok_or_else(|| {
                    Rejection::new("InvalidSetup", format!("{name}'s companion {c} is not a card the engine has"))
                })?),
            });
        }
        let setup = Setup {
            decks,
            sideboards,
            basic_lands_outside,
            companions,
            minimum_deck_size: deal.minimum_deck_size,
            seed: deal.seed,
            shuffle: deal.shuffle,
            opening_chooser,
        };
        let mut game = Game::new(cards, &setup);
        prepare(&mut game);
        game.run();
        let seats = deal.seats.len();
        let mut session = Session {
            tracker: Tracker::new(&game),
            names: deal.seats.iter().map(|(n, _, _)| n.clone()).collect(),
            decklists,
            handles: (0..seats).map(|_| HandleSpace::new()).collect(),
            published: 0,
            sequence: 0,
            conceded: vec![false; seats],
            told: Vec::new(),
            extra: vec![HashSet::new(); seats],
            faces: vec![HashSet::new(); seats],
            sent: vec![None; seats],
            undoable: Undo::default(),
            scenario: None,
            public: None,
            game,
        };
        // A first player named outright is not asked: the question is answered as it is posed.
        if let Some(first) = first.filter(|_| chooser.is_none())
            && let Some(request) = session.game.pending().cloned()
            && matches!(request.kind, RequestKind::ChooseStartingPlayer { .. })
        {
            session.game.answer(request.id, request.player, Answer::Player(first)).map_err(|e| Rejection::new("InvalidSetup", e.0))?;
            session.told.push(Proto::StartingPlayer { chooser: request.player, player: first });
        }
        Ok(session)
    }

    /// A table playing the scenario `text`: dealt as the scenario runner deals it, its seats named `names`, the `driven`
    /// seats answered by the table's clients and every other by the script.
    pub fn scenario(text: &str, name: &str, names: Vec<String>, driven: &[i32]) -> Result<Session, Rejection> {
        let script = GameScript::parse(text, name).map_err(|e| Rejection::new("InvalidSetup", format!("{name}: {e}")))?;
        let cards = cards();
        let mut decklists = Vec::new();
        for i in 0..script.decks.len() {
            let mut shown = Vec::new();
            for card_name in script.decklist(i) {
                let id = card(&card_name)
                    .ok_or_else(|| Rejection::new("InvalidSetup", format!("{name} names {card_name}, which is not a card the engine has")))?;
                shown.push(printed(cards.get(cards.fronts[id.0 as usize].unwrap_or(id))));
            }
            decklists.push(shown);
        }
        let seats = decklists.len();
        let names: Vec<String> = (0..seats).map(|i| names.get(i).cloned().unwrap_or_else(|| format!("Bot{}", i + 1))).collect();
        let mut scripted = Scripted::deal(script).map_err(|f| Rejection::new("InvalidSetup", f.report))?;
        let game = scripted.game().clone();
        let mut session = Session {
            tracker: Tracker::new(&game),
            names,
            decklists,
            handles: (0..seats).map(|_| HandleSpace::new()).collect(),
            published: 0,
            sequence: 0,
            conceded: vec![false; seats],
            told: Vec::new(),
            extra: vec![HashSet::new(); seats],
            faces: vec![HashSet::new(); seats],
            sent: vec![None; seats],
            undoable: Undo::default(),
            scenario: Some(Scenario {
                scripted,
                driven: driven.iter().filter_map(|&s| player_at(s)).collect(),
                expected: None,
                checks: Vec::new(),
                verdict: None,
            }),
            public: None,
            game,
        };
        session.play_scenario();
        Ok(session)
    }

    /// The game is projected for the public too (`PUBLIC`) from its first step on: each step carries `Public`, its view,
    /// events and the map from its handles to each seat's (docs/host-protocol.md). Said as the game is dealt, before it starts.
    pub fn show_public(&mut self) {
        self.public = Some(PublicSide { handles: HandleSpace::new(), sent: None });
    }

    /// A table for a game already under way: a test's way into a position over cards no deal holds (test cards beside the
    /// engine's). Nothing of it is published yet; its seats are named North, South and on.
    #[cfg(test)]
    fn of_game(game: Game) -> Session {
        let seats = game.state.seats().count();
        Session {
            tracker: Tracker::new(&game),
            names: (0..seats).map(|i| ["North", "South", "East", "West"].get(i).map_or_else(|| format!("Seat {}", i + 1), |n| n.to_string())).collect(),
            decklists: vec![Vec::new(); seats],
            handles: (0..seats).map(|_| HandleSpace::new()).collect(),
            published: 0,
            sequence: 0,
            conceded: vec![false; seats],
            told: Vec::new(),
            extra: vec![HashSet::new(); seats],
            faces: vec![HashSet::new(); seats],
            sent: vec![None; seats],
            undoable: Undo::default(),
            scenario: None,
            public: None,
            game,
        }
    }

    /// Plays the scenario on to the next question to a driven seat, gathering the checks of what clients show on the
    /// way, or to its end and its verdict.
    fn play_scenario(&mut self) {
        let Some(scenario) = self.scenario.as_mut() else { return };
        scenario.expected = None;
        std::mem::swap(&mut self.game, scenario.scripted.game());
        loop {
            match scenario.scripted.advance(&scenario.driven) {
                Ok(Advance::Ask(expected)) => {
                    scenario.expected = Some(expected);
                    break;
                }
                Ok(Advance::Check(check)) => scenario.checks.push(check),
                Ok(Advance::Done) => {
                    let verdict = scenario.scripted.verdict().map(|_| scenario.scripted.client_expectations()).map_err(|f| f.report);
                    scenario.verdict = Some(verdict);
                    break;
                }
                Err(f) => {
                    scenario.verdict = Some(Err(f.report));
                    break;
                }
            }
        }
        std::mem::swap(&mut self.game, scenario.scripted.game());
        // What the script's own seats answered is not this table's to take back.
        if let Some(expected) = &scenario.expected
            && scenario.scripted.undo_offered(expected.request.player).is_none()
        {
            self.undoable.clear();
        }
    }

    /// The scenario's part of a step: what each driven seat is meant to answer, the checks for the pages, and the verdict.
    fn scenario_step(&mut self) -> Option<Value> {
        let scenario = self.scenario.as_mut()?;
        let expected = scenario.expected.as_ref().map(|e| {
            let p = answering(&self.game, &e.request);
            let answer = match &e.intent {
                Intent::Undo => Value::Null,
                Intent::Answer(a) => {
                    serde_json::to_value(payload_of(a, &e.request, &mut self.handles[p.index()], &self.game)).expect("an answer serialises")
                }
            };
            json!({ "Seat": seat(p), "RequestId": e.request.id, "Answer": answer, "Undo": matches!(e.intent, Intent::Undo), "Line": e.line })
        });
        let checks: Vec<Value> = scenario.checks.drain(..).map(|c| json!({ "Line": c.line, "Key": c.key, "Value": c.value })).collect();
        let verdict = scenario.verdict.as_ref().map(|v| match v {
            Ok(client) => json!({
                "Passed": true,
                "Client": client.iter().map(|c| json!({ "Line": c.line, "Key": c.key, "Value": c.value })).collect::<Vec<_>>(),
            }),
            Err(report) => json!({ "Passed": false, "Report": report }),
        });
        Some(json!({ "Expected": expected, "Checks": checks, "Verdict": verdict }))
    }

    /// A driven seat's answer to a scenario's question: given through the script, which fails the scenario if it is not
    /// the answer the script means.
    fn answer_scenario(&mut self, request: &Request, payload: &AnswerPayload) -> Result<Value, Rejection> {
        let undo = matches!(payload, AnswerPayload::TakeActionAnswer(TakeActionAnswer { kind: ActionKindView::UndoManaAbility, .. }));
        let intent = if undo {
            Intent::Undo
        } else {
            Intent::Answer(translate(payload, request, &self.handles[answering(&self.game, request).index()], &self.game).map_err(|e| match e {
                Untranslatable::UnknownHandle(d) => Rejection::new("UnknownHandle", d),
                Untranslatable::Illegal(d) => Rejection::new("IllegalAnswer", d),
            })?)
        };
        let noted = match &intent {
            Intent::Answer(a) => Some(Undo::<Told>::before(&self.game, request.player, a)),
            Intent::Undo => None,
        };
        let kept = noted.as_ref().filter(|n| n.is_mana()).map(|_| self.kept());
        let scenario = self.scenario.as_mut().expect("a scenario");
        std::mem::swap(&mut self.game, scenario.scripted.game());
        let given = scenario.scripted.answered(intent);
        std::mem::swap(&mut self.game, scenario.scripted.game());
        if let Err(f) = given {
            scenario.verdict = Some(Err(f.report));
            scenario.expected = None;
            return Ok(self.step());
        }
        let mut pushed = false;
        if undo {
            // The script's game went back already; the table's handles, tracking and told events follow it.
            if let Some(told) = self.undoable.take_back(&mut self.game, request.player) {
                self.taken_back(told);
            }
        } else if let Some(noted) = noted
            && self.undoable.may_take_back(&self.game, &noted)
            && let Some(mut kept) = kept
        {
            let first = self.sequence + 1 + self.told.len() as u64;
            kept.told = first..first;
            self.undoable.push(noted, kept);
            pushed = true;
        }
        self.play_scenario();
        let step = self.step();
        if pushed && let Some(told) = self.undoable.last_kept() {
            told.told.end = self.sequence + 1;
        }
        Ok(step)
    }

    pub fn decklists(&self) -> HashMap<String, Vec<ObjectView>> {
        self.decklists.iter().enumerate().map(|(i, d)| ((i + 1).to_string(), d.clone())).collect()
    }

    pub fn is_over(&self) -> bool {
        self.game.is_over()
    }

    /// The game being played, as it stands: what the AI recorder copies once it ends.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The question outstanding, unless the game is over: what an AI seat beside the table is asked (`cantrip-ai-host`).
    pub fn pending_request(&self) -> Option<Request> {
        self.pending()
    }

    /// `answer` to `request` as its seat's client would send it, in that seat's handles as they stand, which it leaves
    /// as they are; and whether it named an object the seat holds no handle for (which a table would refuse), since the
    /// handles are minted in step with the table's only by what the table does.
    pub fn payload(&self, answer: &Answer, request: &Request) -> (AnswerPayload, bool) {
        let space = &self.handles[request.player.index()];
        let mut scratch = space.clone();
        let payload = payload_of(answer, request, &mut scratch, &self.game);
        (payload, scratch.minted() != space.minted())
    }

    /// The prompt's simplest legal answer to the question outstanding (`PromptPayload::default_answer`), in the seat's
    /// handles, which it leaves as they are: what an AI seat falls back to.
    pub fn default_payload(&self) -> Option<AnswerPayload> {
        let request = self.pending()?;
        let mut scratch = self.handles[request.player.index()].clone();
        prompt(&self.game, &request, &mut scratch).ok().map(|(p, _)| p.default_answer())
    }

    /// The log up to where it can no longer be cut back.
    fn publishable(&self) -> usize {
        publishable(&self.game)
    }

    fn pending(&self) -> Option<Request> {
        if self.game.is_over() { None } else { self.game.pending().cloned() }
    }

    fn view_for(&mut self, p: PlayerId, mentioned: &HashSet<ObjectId>) -> PlayerView {
        // A run in progress is the answering player's alone (see the module doc).
        if deferred(&self.game, p)
            && let Some(sent) = &self.sent[p.index()]
        {
            return sent.clone();
        }
        let extra = self.extra[p.index()].clone();
        let faces = self.faces[p.index()].clone();
        let view = player_view(&self.game, p, &mut self.handles[p.index()], &self.names, &self.conceded, &extra, &faces, mentioned);
        self.sent[p.index()] = Some(view.clone());
        view
    }

    pub fn view(&mut self, seat_number: i32) -> Result<PlayerView, Rejection> {
        let p = player_at(seat_number).filter(|p| p.index() < self.names.len()).ok_or_else(|| Rejection::new("InvalidSetup", "no such seat"))?;
        Ok(self.view_for(p, &HashSet::new()))
    }

    /// Everything one advance of the game produced: the events since the last step as each seat hears them, each seat's
    /// view, and the question outstanding.
    pub fn step(&mut self) -> Value {
        let from = self.published.min(self.game.log.len());
        let bound = self.publishable().max(from);
        let records: Vec<Record> = self.game.log[from..bound].to_vec();
        self.published = bound;
        let mut protos = std::mem::take(&mut self.told);
        let next = self.sequence + 1 + protos.len() as u64;
        protos.extend(self.tracker.collect(&self.game, &records, next));

        let pending = self.pending();
        let revealed: HashSet<ObjectId> = protos
            .iter()
            .flat_map(|p| match p {
                Proto::Revealed { cards, .. } => cards.clone(),
                _ => Vec::new(),
            })
            .collect();
        let seats: Vec<PlayerId> = self.game.state.seats().collect();
        for &p in &seats {
            let mut faces = revealed.clone();
            // Cards a player looked at are shown to that player alone, and to the player controlling them (CR 723.4).
            for proto in &protos {
                if let Proto::Looked { player, cards } = proto
                    && self.game.state.sees_as(p, *player)
                {
                    faces.extend(cards.iter().copied());
                }
            }
            let mut extra = faces.clone();
            self.faces[p.index()] = faces;
            // What a request names is shown to the player it is asked of, and to the player answering it for them.
            if let Some(request) = pending.as_ref().filter(|r| self.game.state.sees_as(p, r.player)) {
                extra.extend(request_objects(request));
            }
            self.extra[p.index()] = extra;
        }

        let base = self.sequence;
        let mut views: HashMap<String, PlayerView> = HashMap::new();
        let mut seat_events: HashMap<String, Vec<Value>> = HashMap::new();
        for &p in &seats {
            let extra = self.extra[p.index()].clone();
            let mut teller = Teller { game: &self.game, viewer: p, handles: &mut self.handles[p.index()], extra: &extra, mentioned: HashSet::new() };
            let mut numbered = Vec::new();
            for (i, proto) in protos.iter().enumerate() {
                if let Some(event) = teller.tell(proto) {
                    let mut value = serde_json::to_value(&event).expect("an event serialises");
                    if let Value::Object(map) = &mut value {
                        map.insert("Sequence".into(), json!(base + 1 + i as u64));
                    }
                    numbered.push(value);
                }
            }
            let mentioned = teller.mentioned;
            seat_events.insert(seat(p).to_string(), numbered);
            views.insert(seat(p).to_string(), self.view_for(p, &mentioned));
        }
        let public = self.public_step(&protos, &revealed, base);
        self.sequence = base + protos.len() as u64;

        let mut requests = Vec::new();
        if let Some(request) = pending {
            // Asked of its player, answered by the seat controlling them (CR 723.5), in that seat's handles.
            let answering = answering(&self.game, &request);
            let index = answering.index();
            let undo = self.undoable.offered(request.player);
            match prompt(&self.game, &request, &mut self.handles[index]) {
                Ok((mut payload, may_cancel)) => {
                    if let Some(source) = undo
                        && let PromptPayload::PriorityPromptMessage(PriorityPrompt { options, .. })
                        | PromptPayload::PayCostsPromptMessage(PayCostsPrompt { options, .. }) = &mut payload
                    {
                        options.push(ActionOptionMessage::new(ActionKindView::UndoManaAbility, self.handles[index].handle(source)));
                    }
                    requests.push(json!({
                    "RequestId": request.id,
                    "Seat": seat(answering),
                    "ForSeat": seat(request.player),
                    "Prompt": payload,
                    "MayCancel": may_cancel,
                    }));
                }
                Err(why) => eprintln!("cantrip-native-host: cannot relay {:?}: {why}", request.kind),
            }
        }

        let (outcome, winning_team) = match self.game.state.outcome {
            Outcome::InProgress => (GameOutcomeView::InProgress, 0),
            Outcome::Draw => (GameOutcomeView::Draw, 0),
            Outcome::Won(p) => (GameOutcomeView::TeamWon, seat(p)),
        };
        let mut step = json!({
            "Requests": requests,
            "Views": views,
            "Events": seat_events,
            "Finished": self.game.is_over(),
            "Outcome": outcome,
            "WinningTeam": winning_team,
        });
        if let Some(scenario) = self.scenario_step() {
            step["Scenario"] = scenario;
        }
        if let Some(public) = public {
            step["Public"] = public;
        }
        step
    }

    /// The public's part of a step, when the game has a public view: what it is told of `protos` (numbered from `base` as
    /// every seat's copy is), its view - the one it was last sent while any seat's run is in progress, as no run is its to
    /// see - and, by seat, each of its handles for an object that seat names by a handle of its own now, with that handle.
    /// It is shown what was revealed (`revealed`) and nothing else: no look, no request names anything to it.
    fn public_step(&mut self, protos: &[Proto], revealed: &HashSet<ObjectId>, base: u64) -> Option<Value> {
        let side = self.public.as_mut()?;
        let mut teller = Teller { game: &self.game, viewer: PUBLIC, handles: &mut side.handles, extra: revealed, mentioned: HashSet::new() };
        let mut events = Vec::new();
        for (i, proto) in protos.iter().enumerate() {
            if let Some(event) = teller.tell(proto) {
                let mut value = serde_json::to_value(&event).expect("an event serialises");
                if let Value::Object(map) = &mut value {
                    map.insert("Sequence".into(), json!(base + 1 + i as u64));
                }
                events.push(value);
            }
        }
        let mentioned = teller.mentioned;
        let view = match &side.sent {
            Some(sent) if deferred(&self.game, PUBLIC) => sent.clone(),
            _ => {
                let view = player_view(&self.game, PUBLIC, &mut side.handles, &self.names, &self.conceded, revealed, revealed, &mentioned);
                side.sent = Some(view.clone());
                view
            }
        };
        let s = &self.game.state;
        let entries = stack_entries(s);
        let named = side.handles.named();
        let mut map = serde_json::Map::new();
        for p in s.seats() {
            let space = &self.handles[p.index()];
            let pairs: Vec<Value> = named
                .iter()
                .filter(|&&(_, id)| {
                    if is_ability_key(id) { entries.iter().any(|e| e.key() == id) } else { id.index() < s.objects.len() && live(s, id) }
                })
                .filter_map(|&(public, id)| {
                    let theirs = space.existing(id);
                    (!theirs.is_none() && space.object(theirs) == Some(id)).then(|| json!([public.0, theirs.0]))
                })
                .collect();
            map.insert(seat(p).to_string(), Value::Array(pairs));
        }
        Some(json!({ "View": view, "Events": events, "Handles": map }))
    }

    pub fn answer(&mut self, request_id: u32, seat_number: i32, payload: &AnswerPayload) -> Result<Value, Rejection> {
        let Some(request) = self.pending() else {
            return Err(Rejection::new("StaleReply", format!("request {request_id} is not outstanding")));
        };
        if request.id != request_id {
            return Err(Rejection::new("StaleReply", format!("request {request_id} is not outstanding")));
        }
        if seat(answering(&self.game, &request)) != seat_number {
            return Err(Rejection::new("NotYourTurnToAnswer", format!("request {request_id} belongs to seat {}", seat(answering(&self.game, &request)))));
        }
        if self.scenario.is_some() {
            return self.answer_scenario(&request, payload);
        }
        if let AnswerPayload::TakeActionAnswer(TakeActionAnswer { kind: ActionKindView::UndoManaAbility, .. }) = payload {
            return self.undo(request.player);
        }
        let answer = translate(payload, &request, &self.handles[answering(&self.game, &request).index()], &self.game).map_err(|e| match e {
            Untranslatable::UnknownHandle(d) => Rejection::new("UnknownHandle", d),
            Untranslatable::Illegal(d) => Rejection::new("IllegalAnswer", d),
        })?;

        let noted = Undo::<Told>::before(&self.game, request.player, &answer);
        let kept = noted.is_mana().then(|| self.kept());
        let before = self.game.clone();
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| self.game.answer(request.id, request.player, answer.clone())));
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(Rejected(why))) => return Err(Rejection::new("IllegalAnswer", why)),
            Err(_) => {
                self.game = before;
                self.undoable.clear();
                return Err(Rejection::new("IllegalAnswer", "the engine failed on that answer; the game is as it was"));
            }
        }
        if self.undoable.may_take_back(&self.game, &noted)
            && let Some(mut kept) = kept
        {
            let first = self.sequence + 1 + self.told.len() as u64;
            kept.told = first..first;
            self.undoable.push(noted, kept);
            let step = self.step();
            if let Some(told) = self.undoable.last_kept() {
                told.told.end = self.sequence + 1;
            }
            return Ok(step);
        }
        match (&request.kind, &answer) {
            (RequestKind::ChooseStartingPlayer { .. }, Answer::Player(first)) => {
                self.told.push(Proto::StartingPlayer { chooser: request.player, player: *first });
            }
            (RequestKind::Mulligan { mulligans }, Answer::Yes) => {
                self.told.push(Proto::Mulligan { player: request.player, count: i32::from(*mulligans) + 1 });
            }
            (RequestKind::Mulligan { mulligans }, _) => {
                self.told.push(Proto::Kept { player: request.player, cards: 7 - i32::from(*mulligans) });
            }
            _ => {}
        }
        Ok(self.step())
    }

    /// What the rule of the outstanding question, a choice of cards or a library search (or a cost's tapping for power,
    /// collecting evidence or crafting), says of the cards `selected` so far, as the seat's handles name them: whether they are
    /// an answer (`Complete`) and which further cards may be added (`Addable`). The client asks it as its player picks; the
    /// game does not move, and the answer itself is still checked when it is given.
    pub fn check(&mut self, request_id: u32, seat_number: i32, selected: &[ViewHandle]) -> Result<Value, Rejection> {
        let Some(request) = self.pending().filter(|r| r.id == request_id) else {
            return Err(Rejection::new("StaleReply", format!("request {request_id} is not outstanding")));
        };
        if seat(answering(&self.game, &request)) != seat_number {
            return Err(Rejection::new("NotYourTurnToAnswer", format!("request {request_id} belongs to seat {}", seat(answering(&self.game, &request)))));
        }
        let space = &mut self.handles[answering(&self.game, &request).index()];
        let mut ids = Vec::with_capacity(selected.len());
        for &h in selected {
            match space.object(h) {
                Some(id) if id.index() < self.game.state.objects.len() => ids.push(id),
                _ => return Err(Rejection::new("UnknownHandle", format!("handle {} names nothing this seat can see", h.0))),
            }
        }
        match crate::prompt::verdict_of(&self.game, &request, &ids, space) {
            Some(verdict) => Ok(json!({ "Complete": verdict.complete, "Addable": verdict.addable })),
            None => Err(Rejection::new("IllegalAnswer", format!("request {request_id} is not a choice of cards"))),
        }
    }

    /// What the rules make of the declaration of blockers `blocks`, as the seat's handles name them, for the outstanding
    /// declaration (CR 509.1a-d, `BlockRequirements::judge`): whether it may be declared as it is (`Complete`), why not
    /// (`Issue`), what it costs (`Cost`) and the blocks some legal declaration extending it has (`Addable`). The client asks
    /// it as its player declares; the game does not move, and the answer itself is still checked when it is given.
    pub fn check_blocks(&mut self, request_id: u32, seat_number: i32, blocks: &[crate::protocol::BlockMessage]) -> Result<Value, Rejection> {
        let request = self.outstanding_for(request_id, seat_number)?;
        let RequestKind::DeclareBlockers { allowed, .. } = &request.kind else {
            return Err(Rejection::new("IllegalAnswer", format!("request {request_id} is not a declaration of blockers")));
        };
        // More blocks than there are blocks to make is no declaration: refused before anything is judged.
        if blocks.len() > allowed.len() {
            return Err(Rejection::new("IllegalAnswer", format!("{} blocks, of {} that may be made", blocks.len(), allowed.len())));
        }
        let payload = AnswerPayload::BlockersDeclaredAnswer(crate::protocol::BlockersDeclaredAnswer { blocks: blocks.to_vec() });
        let Answer::Block(pairs) = self.translated(&request, &payload)? else {
            return Err(Rejection::new("IllegalAnswer", format!("request {request_id} is not a declaration of blockers")));
        };
        let space = &mut self.handles[answering(&self.game, &request).index()];
        let verdict = crate::prompt::blocks_verdict_of(&request, &pairs, space)
            .ok_or_else(|| Rejection::new("IllegalAnswer", format!("request {request_id} is not a declaration of blockers")))?;
        serde_json::to_value(verdict).map_err(|e| Rejection::new("IllegalAnswer", format!("the verdict does not serialize: {e}")))
    }

    /// What the rules make of the declaration of attackers `attacks`, as the seat's handles name them, for the outstanding
    /// declaration (CR 508.1a-d, `AttackRules::judge`): whether it may be declared as it is, why not, what it costs and the
    /// attacks some legal declaration extending it has. Nothing advances.
    pub fn check_attacks(&mut self, request_id: u32, seat_number: i32, attacks: &[crate::protocol::AttackMessage]) -> Result<Value, Rejection> {
        let request = self.outstanding_for(request_id, seat_number)?;
        let RequestKind::DeclareAttackers { eligible, players, planeswalkers, battles, .. } = &request.kind else {
            return Err(Rejection::new("IllegalAnswer", format!("request {request_id} is not a declaration of attackers")));
        };
        // More attacks than creatures that may attack is no declaration: refused before anything is judged.
        if attacks.len() > eligible.len() || (players.is_empty() && planeswalkers.is_empty() && battles.is_empty() && !attacks.is_empty()) {
            return Err(Rejection::new("IllegalAnswer", format!("{} attacks, of {} creatures that may attack", attacks.len(), eligible.len())));
        }
        let payload = AnswerPayload::AttackersDeclaredAnswer(crate::protocol::AttackersDeclaredAnswer { attacks: attacks.to_vec() });
        let Answer::Attack(pairs) = self.translated(&request, &payload)? else {
            return Err(Rejection::new("IllegalAnswer", format!("request {request_id} is not a declaration of attackers")));
        };
        let space = &mut self.handles[answering(&self.game, &request).index()];
        let verdict = crate::prompt::attacks_verdict_of(&request, &pairs, space)
            .ok_or_else(|| Rejection::new("IllegalAnswer", format!("request {request_id} is not a declaration of attackers")))?;
        serde_json::to_value(verdict).map_err(|e| Rejection::new("IllegalAnswer", format!("the verdict does not serialize: {e}")))
    }

    /// The outstanding request `request_id`, which must be the seat's.
    fn outstanding_for(&self, request_id: u32, seat_number: i32) -> Result<Request, Rejection> {
        let Some(request) = self.pending().filter(|r| r.id == request_id) else {
            return Err(Rejection::new("StaleReply", format!("request {request_id} is not outstanding")));
        };
        if seat(answering(&self.game, &request)) != seat_number {
            return Err(Rejection::new("NotYourTurnToAnswer", format!("request {request_id} belongs to seat {}", seat(answering(&self.game, &request)))));
        }
        Ok(request)
    }

    /// `payload` as the engine's answer to `request`, in the asked seat's handles.
    fn translated(&self, request: &Request, payload: &AnswerPayload) -> Result<Answer, Rejection> {
        translate(payload, request, &self.handles[answering(&self.game, request).index()], &self.game).map_err(|e| match e {
            Untranslatable::UnknownHandle(d) => Rejection::new("UnknownHandle", d),
            Untranslatable::Illegal(d) => Rejection::new("IllegalAnswer", d),
        })
    }

    /// The seat gives up: it loses, and the game ends if one side is left.
    /// Takes back the latest mana ability `player` activated: the game goes back to the node it was activated at, by
    /// the tree (`Game::goto`), and what was told of it is retracted.
    fn undo(&mut self, player: PlayerId) -> Result<Value, Rejection> {
        let Some(told) = self.undoable.take_back(&mut self.game, player) else {
            return Err(Rejection::new("IllegalAnswer", "there is no mana ability to take back"));
        };
        self.taken_back(told);
        Ok(self.step())
    }

    /// What a mana ability that can be taken back keeps of the table as it was before it (`Told`), its told entries still
    /// to be counted.
    fn kept(&self) -> Told {
        Told {
            tracker: self.tracker.clone(),
            handles: self.handles.clone(),
            public: self.public.as_ref().map(|p| p.handles.clone()),
            published: self.published,
            told: 0..0,
        }
    }

    /// The table goes back to what a mana ability taken back kept of it: the tracking, every seat's handles and the public's,
    /// and how far the log was published; what was told of it is retracted.
    fn taken_back(&mut self, told: Told) {
        self.tracker = told.tracker;
        for (space, earlier) in self.handles.iter_mut().zip(told.handles) {
            space.restore(earlier);
        }
        if let (Some(public), Some(earlier)) = (self.public.as_mut(), told.public) {
            public.handles.restore(earlier);
        }
        self.published = told.published;
        if !told.told.is_empty() {
            self.told.push(Proto::Retracted { sequences: told.told.collect() });
        }
    }

    /// The game ends from outside its rules (`Game::end`): won by the seat `winner`, or a draw for none - a draw the players
    /// agreed to, a match clock run out. What its end reveals (CR 708.9) is told with it.
    pub fn end(&mut self, winner: Option<i32>) -> Result<Value, Rejection> {
        if self.game.is_over() {
            return Err(Rejection::new("IllegalAnswer", "the game is over"));
        }
        let outcome = match winner {
            None => Outcome::Draw,
            Some(seat_number) => Outcome::Won(
                player_at(seat_number)
                    .filter(|p| p.index() < self.names.len() && !self.game.state.player(*p).lost)
                    .ok_or_else(|| Rejection::new("IllegalAnswer", "no such seat in the game"))?,
            ),
        };
        self.undoable.clear();
        self.game.end(outcome);
        Ok(self.step())
    }

    pub fn concede(&mut self, seat_number: i32) -> Result<Value, Rejection> {
        let p = player_at(seat_number).filter(|p| p.index() < self.names.len()).ok_or_else(|| Rejection::new("NotYourTurnToAnswer", "no such seat"))?;
        if self.game.is_over() {
            return Err(Rejection::new("IllegalAnswer", "the game is over"));
        }
        self.conceded[p.index()] = true;
        self.undoable.clear();
        // The engine's own leaving (CR 104.3a, 800.4a): the monarch and the initiative pass on, the seat's objects leave, and the
        // game ends with one player left.
        self.told.extend(self.tracker.conceded(p));
        self.game.concede(p);
        Ok(self.step())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays both seats from the prompts alone, as the bot of `tests/stdio_game.rs` does, until `done` has seen
    /// what it is looking for in the events some seat was told. Returns every event each seat heard.
    fn play_until(session: &mut Session, done: impl Fn(&[Value]) -> bool) -> HashMap<String, Vec<Value>> {
        let mut heard: HashMap<String, Vec<Value>> = HashMap::new();
        let mut step = session.step();
        // A spell the bot cannot pay for is given up on until the next step of the turn, not tried again at once.
        let mut allow_cast = true;
        for _ in 0..2_000 {
            for (seat, list) in step["Events"].as_object().unwrap() {
                heard.entry(seat.clone()).or_default().extend(list.as_array().unwrap().iter().cloned());
            }
            if heard.values().any(|events| done(events)) {
                return heard;
            }
            assert!(!step["Finished"].as_bool().unwrap(), "the game ended first: {heard:?}");
            let request = &step["Requests"][0];
            let prompt: PromptPayload = serde_json::from_value(request["Prompt"].clone()).unwrap();
            let seat = request["Seat"].as_i64().unwrap() as i32;
            let id = request["RequestId"].as_u64().unwrap() as u32;
            if step["Events"].as_object().unwrap().values().any(|list| !of_type(list.as_array().unwrap(), "StepBeganEvent").is_empty()) {
                allow_cast = true;
            }
            let answer = choose(&prompt, &mut allow_cast);
            step = match session.answer(id, seat, &answer) {
                Ok(step) => step,
                Err(_) => session.answer(id, seat, &prompt.default_answer()).ok().expect("the default answer is taken"),
            };
        }
        let kinds: Vec<&str> = heard["1"].iter().map(|e| e["$type"].as_str().unwrap()).collect();
        panic!("the awaited event never came; last events {:?}; pending {}", &kinds[kinds.len().saturating_sub(12)..], step["Requests"][0]["Prompt"]);
    }

    fn choose(prompt: &PromptPayload, allow_cast: &mut bool) -> AnswerPayload {
        match prompt {
            PromptPayload::PriorityPromptMessage(p) => {
                let pick = |kind: ActionKindView, quiet: bool| p.options.iter().find(|o| o.kind == kind && (!quiet || o.produces.is_empty()));
                match pick(ActionKindView::PlayLand, false).or(pick(ActionKindView::CastSpell, false).filter(|_| *allow_cast)).or(pick(ActionKindView::ActivateAbility, true)) {
                    Some(o) => AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o)),
                    None => AnswerPayload::PassAnswer(PassAnswer {}),
                }
            }
            PromptPayload::PayCostsPromptMessage(p) => {
                let pick = |kind: ActionKindView| p.options.iter().find(|o| o.kind == kind);
                if let Some(o) = pick(ActionKindView::ConfirmCasting) {
                    return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
                }
                if let Some(o) = p.options.iter().find(|o| o.kind == ActionKindView::ActivateAbility && !o.produces.is_empty()) {
                    let mut answer = TakeActionAnswer::take(o);
                    answer.color = o.color_choices.first().copied();
                    return AnswerPayload::TakeActionAnswer(answer);
                }
                if let Some(o) = pick(ActionKindView::AssignMana) {
                    *allow_cast = false;
                    return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
                }
                *allow_cast = false;
                prompt.default_answer()
            }
            other => other.default_answer(),
        }
    }

    fn deal(first: &[&str], filler: &str, count: usize) -> Deal {
        let deck = |top: &[&str]| top.iter().map(|c| c.to_string()).chain(std::iter::repeat_n(filler.to_string(), count)).collect::<Vec<_>>();
        Deal { seats: vec![("North".into(), deck(first), Vec::new()), ("South".into(), deck(&[]), Vec::new())], basic_lands_outside: Vec::new(), companions: Vec::new(), minimum_deck_size: 60, seed: 1, shuffle: false, first_player: Some(1), chooser: None }
    }

    /// A game whose South is controlled by North during South's next turn (CR 723.1): the effect put in place before the
    /// game begins, as no card here makes it.
    fn controlled_south() -> Session {
        let (north, south) = (PlayerId(0), PlayerId(1));
        Session::prepared(deal(&[], "Forest", 30), |game| {
            let source = game.state.zone(Zone::library(north))[0];
            let timestamp = game.state.timestamp();
            game.state.effect_push(ContinuousEffect {
                source,
                controller: north,
                timestamp,
                affected: source,
                kind: EffectKind::ControlNextTurn { player: south, then_extra_turn: false },
                duration: Duration::Indefinite,
            });
        })
        .ok()
        .unwrap()
    }

    /// CR 723.5, 723.4: every question of South's in its controlled turn goes to seat 1 with `ForSeat` 2, in seat 1's handles,
    /// and seat 2 answering it is refused; seat 1 sees South's hand then, and both views say who controls South. Before and
    /// after that turn, South answers for itself and its hand is its own.
    #[test]
    fn a_controlled_players_questions_go_to_the_seat_controlling_them() {
        let south = PlayerId(1);
        let mut session = controlled_south();
        let mut step = session.step();
        let mut allow_cast = true;
        let mut controlled = 0;
        let hand_of = |view: &Value, owner: i64| {
            view["Zones"].as_array().unwrap().iter().find(|z| z["Kind"] == "Hand" && z["Owner"] == owner).unwrap()["Contents"].as_array().unwrap().len()
        };
        for _ in 0..2_000 {
            let turn = session.game.state.turn.number;
            if turn == 3 {
                break;
            }
            let r = step["Requests"][0].clone();
            let (id, seat, for_seat) = (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32, r["ForSeat"].as_i64().unwrap());
            let prompt: PromptPayload = serde_json::from_value(r["Prompt"].clone()).unwrap();
            let south_controlled = turn == 2;
            if for_seat == 2 {
                assert_eq!(seat, if south_controlled { 1 } else { 2 }, "turn {turn}: South's question goes to {seat}");
            } else {
                assert_eq!(seat, for_seat as i32, "North answers its own");
            }
            let shown = |seat: &str| step["Views"][seat]["Players"][1]["ControlledBy"].as_i64().unwrap();
            if south_controlled {
                controlled += 1;
                assert_eq!((shown("1"), shown("2")), (1, 1));
                assert_eq!(hand_of(&step["Views"]["1"], 2), session.game.state.zone(Zone::hand(south)).len(), "North sees South's hand");
                if for_seat == 2 {
                    let refused = session.answer(id, 2, &prompt.default_answer());
                    assert!(refused.is_err_and(|r| r.code == "NotYourTurnToAnswer"));
                }
            } else {
                assert_eq!((shown("1"), shown("2")), (0, 0));
                assert_eq!(hand_of(&step["Views"]["1"], 2), 0, "turn {turn}: South's hand is South's");
            }
            let answer = choose(&prompt, &mut allow_cast);
            step = match session.answer(id, seat, &answer) {
                Ok(step) => step,
                Err(_) => session.answer(id, seat, &prompt.default_answer()).ok().expect("the default answer is taken"),
            };
        }
        assert!(controlled > 0, "South's turn was played");
        assert_eq!(session.game.state.player_controller(south), south);
    }

    /// CR 723.6: a controlled player may concede, and the controller can't concede for them: seat 2 concedes during its
    /// controlled turn and loses.
    #[test]
    fn a_controlled_player_concedes_for_themselves() {
        let (north, south) = (PlayerId(0), PlayerId(1));
        let mut session = controlled_south();
        let mut step = session.step();
        let mut allow_cast = true;
        while session.game.state.player_controller(south) != north {
            let r = step["Requests"][0].clone();
            let (id, seat) = (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32);
            let prompt: PromptPayload = serde_json::from_value(r["Prompt"].clone()).unwrap();
            step = session.answer(id, seat, &choose(&prompt, &mut allow_cast)).ok().unwrap();
        }
        let over = session.concede(2).ok().unwrap();
        assert_eq!(over["Outcome"], "TeamWon");
        assert_eq!(over["WinningTeam"], 1);
    }

    /// A game deals as many seats as a set of seats holds (`MAX_SEATS`, a bit each in `Obj::seen_by`): eight are dealt,
    /// nine refused.
    #[test]
    fn a_deal_of_more_seats_than_a_game_holds_is_refused() {
        let seats = |n: usize| Deal { seats: (0..n).map(|i| (format!("Seat {i}"), vec!["Forest".to_string(); 20], Vec::new())).collect(), ..deal(&[], "Forest", 20) };
        assert!(Session::new(seats(MAX_SEATS)).is_ok());
        assert!(Session::new(seats(MAX_SEATS + 1)).is_err_and(|r| r.code == "InvalidSetup"));
    }

    fn of_type<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
        events.iter().filter(|e| e["$type"] == kind).collect()
    }

    /// CR 708.9: the face-down permanents and spells revealed as their owner leaves the game (a game of three that goes on)
    /// and at the end of the game (a duel a concession ends) reach every seat as a `CardsRevealedEvent` by their owner, before
    /// the game's end is told, each naming an object the seat's view says is what it really is - a face-down card the seat
    /// may now look at, though one that left with its owner has ceased to exist.
    #[test]
    fn face_down_cards_revealed_as_their_owner_leaves_and_at_the_end_reach_every_seat() {
        for (name, text, seats, over) in [
            (
                "leaves",
                include_str!("../../../scenarios/face-down-cards-are-revealed-as-their-owner-concedes-a-game-of-three.cantrip"),
                ["1", "2", "3"].as_slice(),
                false,
            ),
            ("ends", include_str!("../../../scenarios/face-down-cards-are-revealed-at-the-end-of-the-game.cantrip"), ["1", "2"].as_slice(), true),
            // The game ends with the cast's frame still waiting on its payment: what ended it is published all the same.
            (
                "ends mid-cast",
                include_str!("../../../scenarios/a-face-down-spell-being-cast-is-revealed-when-a-concession-ends-the-game.cantrip"),
                ["1", "2"].as_slice(),
                true,
            ),
        ] {
            let mut session = Session::scenario(text, name, Vec::new(), &[]).ok().unwrap();
            let step = session.step();
            assert_eq!(step["Scenario"]["Verdict"]["Passed"], true, "{name}: {step}");
            assert_eq!(step["Finished"], over, "{name}");
            for &seat in seats {
                let events = step["Events"][seat].as_array().unwrap();
                let revealed = of_type(events, "CardsRevealedEvent");
                let objects = step["Views"][seat]["Objects"].as_array().unwrap();
                let mut names: Vec<&str> = revealed
                    .iter()
                    .flat_map(|e| e["Cards"].as_array().unwrap())
                    .map(|handle| {
                        let object = objects.iter().find(|o| &o["Handle"] == handle).unwrap_or_else(|| panic!("{name}: seat {seat}: {handle} is in the view"));
                        assert_eq!(object["Known"], true, "{name}: seat {seat}: {object}");
                        assert_eq!(object["FaceDown"], true, "{name}: seat {seat}: {object}");
                        object["FaceDownName"].as_str().unwrap()
                    })
                    .collect();
                names.sort();
                assert_eq!(names, ["Abzan Guide", "Bolrac-Clan Basher"], "{name}: seat {seat}");
                assert!(revealed.iter().all(|e| e["Player"] == 1), "{name}: seat {seat}");
                if over {
                    let ended = of_type(events, "GameEndedEvent");
                    assert_eq!(ended.len(), 1, "{name}: seat {seat}");
                    assert!(revealed.iter().all(|e| e["Sequence"].as_u64() < ended[0]["Sequence"].as_u64()), "{name}: seat {seat}");
                }
            }
        }
    }

    /// A game ended from outside its rules (`Session::end`: a draw agreed to, a match clock) ends as every game does: the
    /// face-down permanent on the battlefield is revealed by its owner (CR 708.9) to every seat, with the game's end.
    #[test]
    fn a_game_ended_from_outside_its_rules_reveals_its_face_down_permanents() {
        let text = "[Bot1 Deck]\nAbzan Guide\nMox Pearl\nMox Jet\nMox Emerald\nIsland x16\n\n[Bot2 Deck]\nIsland x20\n\n[Actions]\n\
                    Bot1.Hand.Mox Pearl.Cast\nBot1.Pass\nBot1.Hand.Mox Jet.Cast\nBot1.Pass\nBot1.Hand.Mox Emerald.Cast\nBot1.Pass\n\
                    Bot1.Battlefield.Mox Pearl.Activate\nBot1.Battlefield.Mox Jet.Activate\nBot1.Battlefield.Mox Emerald.Activate\n\
                    Bot1.Hand.Abzan Guide.Cast(Ability: Morph)\nBot1.Pass\nBot2.Pass\n\n[Expect]\nBot1.Battlefield..FaceDownName = Abzan Guide\n";
        for winner in [None, Some(2)] {
            let mut session = Session::scenario(text, "ended", Vec::new(), &[]).ok().unwrap();
            let step = session.step();
            assert_eq!(step["Scenario"]["Verdict"]["Passed"], true, "{step}");
            assert_eq!(step["Finished"], false);
            let step = session.end(winner).ok().unwrap();
            assert_eq!(step["Finished"], true, "{winner:?}");
            assert_eq!(step["Outcome"], if winner.is_some() { "TeamWon" } else { "Draw" }, "{winner:?}");
            for seat in ["1", "2"] {
                let events = step["Events"][seat].as_array().unwrap();
                let revealed = of_type(events, "CardsRevealedEvent");
                assert_eq!(revealed.len(), 1, "{winner:?}: seat {seat}: {events:?}");
                assert_eq!(revealed[0]["Player"], 1);
                let handle = &revealed[0]["Cards"][0];
                let objects = step["Views"][seat]["Objects"].as_array().unwrap();
                let object = objects.iter().find(|o| &o["Handle"] == handle).expect("the revealed card is in the view");
                assert_eq!((object["Known"].clone(), object["FaceDownName"].clone()), (Value::Bool(true), json!("Abzan Guide")), "{winner:?}: seat {seat}");
                assert_eq!(of_type(events, "GameEndedEvent").len(), 1, "{winner:?}: seat {seat}");
            }
            assert!(session.end(None).is_err(), "a game over is not ended again");
        }
    }

    /// "Turn target face-down permanent face up." No printed card the engine has turns a permanent face up as an effect (Break
    /// Open does), so a test card does here, beside every card the engine has.
    fn unveil(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
        if let Target::Object(id) = item.target(0) {
            g.turn_face_up(id, item.controller);
        }
        Ok(())
    }

    fn face_down_permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
        matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.obj(id).face_down)
    }

    static UNVEIL: CardDef =
        CardDef { name: "Probe Unveil", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(face_down_permanent)], unveil), ..CardDef::BLANK };

    /// North casts three Moxen and Cryptic Coat, which cloaks the Lightning Bolt on top of its library, then Probe Unveil at the
    /// cloaked Bolt, at a table once the Unveil is on the stack. The Bolt's permanent.
    fn unveiling_a_cloaked_bolt() -> (Session, ObjectId) {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new(cards().cards.iter().copied().chain([&UNVEIL]))));
        let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table.find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
        let mut north = names(&["Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Cryptic Coat", "Probe Unveil", "Island", "Island", "Lightning Bolt"]);
        north.extend(names(&["Island"; 12]));
        // South holds an Unveil of its own: from the reveal on, its questions name the cloaked Bolt as a target it may choose.
        let mut south = names(&["Probe Unveil"]);
        south.extend(names(&["Island"; 19]));
        let mut game = Game::new(table, &Setup { decks: vec![north, south], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
        let mut casts = ["Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Cryptic Coat"].into_iter().peekable();
        let cloaked = |game: &Game| game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).face_down);
        for _ in 0..200 {
            game.run();
            if game.state.stack.last().is_some_and(|item| game.name(item.object) == "Probe Unveil") {
                let bolt = cloaked(&game).expect("the cloaked Bolt");
                assert_eq!(game.cards.get(game.state.obj(bolt).card).name, "Lightning Bolt");
                return (Session::of_game(game), bolt);
            }
            let request = game.pending().cloned().expect("the game goes on");
            let answer = match &request.kind {
                RequestKind::Priority { options } if request.player == PlayerId(0) => {
                    let named = |name: &str| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name));
                    if let Some(option) = casts.peek().and_then(|&name| named(name)) {
                        casts.next();
                        Answer::Act(Action::of(option))
                    } else if let (Some(permanent), true) = (cloaked(&game), game.state.stack.is_empty()) {
                        let mut action = Action::of(named("Probe Unveil").expect("the Unveil"));
                        action.targets = vec![Target::Object(permanent)];
                        Answer::Act(action)
                    } else {
                        Answer::Pass
                    }
                }
                RequestKind::PayCosts { options, .. } => {
                    let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Mox Sapphire"));
                    let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting);
                    Answer::Act(Action::of(confirm.or(mox).expect("a Mox to pay with, or the payment to confirm")))
                }
                RequestKind::Priority { .. } => Answer::Pass,
                other => panic!("unexpected question {other:?}"),
            };
            game.answer(request.id, request.player, answer).unwrap();
        }
        panic!("Probe Unveil was never cast");
    }

    /// The seat's view of the object a handle names.
    fn object_in<'a>(step: &'a Value, seat: &str, handle: &Value) -> &'a Value {
        let objects = step["Views"][seat]["Objects"].as_array().unwrap();
        objects.iter().find(|o| &o["Handle"] == handle).unwrap_or_else(|| panic!("seat {seat}: {handle} is in the view"))
    }

    /// CR 701.58g, 701.20a, 708.5: a cloaked instant an effect would turn face up is revealed by its controller and left face
    /// down. Every seat is shown it: each seat's `CardsRevealedEvent` names an object its view says is the Bolt, face down.
    /// The reveal over, the opponent may no longer look at the face-down permanent its controller still may, though the
    /// question it is asked then names it (a target its Unveil may choose): a question shows what it names, not its face.
    #[test]
    fn a_cloaked_instant_an_effect_would_turn_up_is_shown_to_every_seat_and_stays_face_down() {
        let (mut session, bolt) = unveiling_a_cloaked_bolt();
        let mut step = session.step();
        let mut passes = 0;
        while of_type(step["Events"]["2"].as_array().unwrap(), "CardsRevealedEvent").is_empty() {
            passes += 1;
            assert!(passes < 5, "the Bolt was never revealed: {step}");
            let request = session.pending_request().expect("a question");
            let (payload, _) = session.payload(&Answer::Pass, &request);
            step = session.answer(request.id, seat(session.game.state.player_controller(request.player)), &payload).ok().unwrap();
        }
        assert!(session.game.state.obj(bolt).face_down, "the Bolt stays face down");
        for seat in ["1", "2"] {
            let revealed = of_type(step["Events"][seat].as_array().unwrap(), "CardsRevealedEvent");
            assert_eq!(revealed.len(), 1, "seat {seat}");
            assert_eq!(revealed[0]["Player"], 1, "seat {seat}");
            let object = object_in(&step, seat, &revealed[0]["Cards"][0]);
            assert_eq!(object["FaceDown"], true, "seat {seat}: {object}");
            assert_eq!(object["Known"], true, "seat {seat}: {object}");
            assert_eq!(object["FaceDownName"], "Lightning Bolt", "seat {seat}: {object}");
            assert_eq!(object["Zone"], "Battlefield", "seat {seat}: {object}");
        }

        // The next step, with the reveal over: the opponent's view no longer says what it is; its controller's does.
        let request = session.pending_request().expect("a question");
        let (payload, _) = session.payload(&Answer::Pass, &request);
        let after = session.answer(request.id, seat(request.player), &payload).ok().unwrap();
        assert_eq!(after["Requests"][0]["Seat"], 2, "South is asked: {after}");
        assert!(request_objects(&session.pending_request().unwrap()).contains(&bolt), "South's question names the Bolt");
        for (seat, known) in [("1", true), ("2", false)] {
            let handle = json!(session.handles[seat.parse::<usize>().unwrap() - 1].existing(bolt));
            let object = object_in(&after, seat, &handle);
            assert_eq!(object["FaceDown"], true, "seat {seat}: {object}");
            assert_eq!(object["Known"], known, "seat {seat}: {object}");
            assert_eq!(object["FaceDownName"], if known { "Lightning Bolt" } else { "" }, "seat {seat}: {object}");
        }
    }

    /// Created events and object views publish the actual characteristic name, not the token definition's name
    /// (CR 111.4, 612.2a, 707.2). The existing scenarios force changed creating text, copies, and an explicit name.
    #[test]
    fn token_names_in_events_and_views_match_the_created_characteristics() {
        for (name, text, token, count) in [
            ("compound-name", include_str!("../../../scenarios/compound-token-name-follows-text-changing-its-creating-spell.cantrip"), "Tentacle Soldier Token", 1),
            ("compound-copy", include_str!("../../../scenarios/compound-token-name-copy-keeps-creating-text-not-later-subtype.cantrip"), "Tentacle Soldier Token", 3),
            ("explicit-name", include_str!("../../../scenarios/explicit-token-name-stays-literal-when-creating-types-change.cantrip"), "Ajani's Pridemate", 1),
        ] {
            let mut session = Session::scenario(text, name, Vec::new(), &[]).ok().unwrap();
            let step = session.step();
            assert_eq!(step["Scenario"]["Verdict"]["Passed"], true, "{name}: {step}");
            for seat in ["1", "2"] {
                let events = of_type(step["Events"][seat].as_array().unwrap(), "TokenCreatedEvent");
                assert_eq!(events.len(), count, "{name}: seat {seat}");
                assert!(events.iter().all(|e| e["Name"] == token), "{name}: {events:?}");
                let objects = step["Views"][seat]["Objects"].as_array().unwrap();
                assert_eq!(objects.iter().filter(|o| o["Name"] == token && o["Kind"] == "Token").count(), count, "{name}: seat {seat}");
            }
        }
    }

    /// A mana ability is taken back: the game returns to before it, the land untapped and its mana gone, and nothing
    /// more is offered to take back. One that finishes paying for a spell is not offered: the spell is cast.
    #[test]
    fn a_mana_ability_is_taken_back() {
        let mut session = Session::new(deal(&["Llanowar Elves"], "Forest", 19)).ok().unwrap();
        let mut step = session.step();
        let mut allow_cast = true;
        let request = |step: &Value| {
            let r = &step["Requests"][0];
            let prompt: PromptPayload = serde_json::from_value(r["Prompt"].clone()).unwrap();
            (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32, prompt)
        };
        let options = |prompt: &PromptPayload| match prompt {
            PromptPayload::PriorityPromptMessage(p) => p.options.clone(),
            PromptPayload::PayCostsPromptMessage(p) => p.options.clone(),
            _ => Vec::new(),
        };
        let mana = |prompt: &PromptPayload| options(prompt).into_iter().find(|o| o.kind == ActionKindView::ActivateAbility && !o.produces.is_empty());
        let undo = |prompt: &PromptPayload| options(prompt).into_iter().find(|o| o.kind == ActionKindView::UndoManaAbility);
        let take = |o: &ActionOptionMessage| AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
        let mut taken_back = false;
        for _ in 0..2_000 {
            let (id, seat, prompt) = request(&step);
            if !taken_back
                && matches!(prompt, PromptPayload::PriorityPromptMessage(_))
                && let Some(forest) = mana(&prompt)
            {
                let tapped = session.answer(id, seat, &take(&forest)).ok().unwrap();
                let (id, seat, prompt) = request(&tapped);
                assert!(mana(&prompt).is_none(), "the Forest is tapped: {prompt:?}");
                let green = |step: &Value| step["Views"]["1"]["Players"].as_array().unwrap().iter().map(|p| p["Mana"][4].as_i64().unwrap()).sum::<i64>();
                assert_eq!(green(&tapped), 1, "the Forest's mana is in the pool");
                let back = undo(&prompt).expect("Undo is offered");
                assert_eq!(back.source, forest.source, "on the Forest");

                step = session.answer(id, seat, &take(&back)).ok().unwrap();
                let (_, _, prompt) = request(&step);
                assert_eq!(mana(&prompt).map(|o| o.source), Some(forest.source), "the Forest is untapped");
                assert!(undo(&prompt).is_none(), "nothing more to take back");
                let pools: Vec<&Value> = step["Views"]["1"]["Players"].as_array().unwrap().iter().map(|p| &p["Mana"]).collect();
                assert!(pools.iter().all(|pool| pool.as_array().unwrap().iter().all(|n| n == 0)), "its mana is gone: {pools:?}");
                taken_back = true;
                continue;
            }
            if let PromptPayload::PayCostsPromptMessage(_) = &prompt
                && let Some(forest) = mana(&prompt)
            {
                step = session.answer(id, seat, &take(&forest)).ok().unwrap();
                let (_, _, prompt) = request(&step);
                assert!(undo(&prompt).is_none(), "the spell is cast, and its mana not taken back: {prompt:?}");
                assert!(taken_back);
                return;
            }
            let answer = choose(&prompt, &mut allow_cast);
            step = match session.answer(id, seat, &answer) {
                Ok(step) => step,
                Err(_) => session.answer(id, seat, &prompt.default_answer()).ok().expect("the default answer is taken"),
            };
        }
        panic!("Llanowar Elves was never paid for");
    }

    /// Plays on until seat 1 may activate a mana ability of the permanent named `name` with priority, activates it for
    /// green, and returns that option and the step after it.
    fn activate_mana_of(session: &mut Session, name: &str) -> (ActionOptionMessage, Value) {
        let mut step = session.step();
        let mut allow_cast = true;
        for _ in 0..2_000 {
            let r = &step["Requests"][0];
            assert!(!r.is_null(), "{name}'s mana ability was never offered before the game ended");
            let prompt: PromptPayload = serde_json::from_value(r["Prompt"].clone()).unwrap();
            let (id, seat) = (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32);
            let named = |h: ViewHandle| step["Views"]["1"]["Objects"].as_array().unwrap().iter().any(|o| o["Handle"] == json!(h) && o["Name"] == name);
            if seat == 1
                && let PromptPayload::PriorityPromptMessage(p) = &prompt
                && let Some(mana) = p.options.iter().find(|o| AutoMana::is(o) && named(o.source))
            {
                let mut answer = TakeActionAnswer::take(mana);
                answer.color = mana.color_choices.contains(&ColorView::Green).then_some(ColorView::Green);
                let after = session.answer(id, seat, &AnswerPayload::TakeActionAnswer(answer)).ok().unwrap();
                return (mana.clone(), after);
            }
            let answer = match &prompt {
                PromptPayload::MayPromptMessage(_) if seat == 1 => AnswerPayload::MayChosenAnswer(MayChosenAnswer { accept: true }),
                _ => choose(&prompt, &mut allow_cast),
            };
            step = match session.answer(id, seat, &answer) {
                Ok(step) => step,
                Err(_) => session.answer(id, seat, &prompt.default_answer()).ok().expect("the default answer is taken"),
            };
        }
        panic!("{name}'s mana ability was never offered");
    }

    struct AutoMana;
    impl AutoMana {
        fn is(o: &ActionOptionMessage) -> bool {
            o.kind == ActionKindView::ActivateAbility && (!o.produces.is_empty() || o.produces_of_any_one_color > 0)
        }
    }

    /// Activates `name`'s mana ability and takes it back: offered, its green gone after, and the permanent on the
    /// battlefield under the handle it had, its ability offered again.
    fn takes_back(top: &[&str], filler: &str, name: &str) {
        let mut session = Session::new(deal(top, filler, 20 - top.len())).ok().unwrap();
        let (used, after) = activate_mana_of(&mut session, name);
        assert_eq!(green_in_pool(&after), 1, "{name}: its mana is in the pool");
        let undo = options_of(&after).into_iter().find(|o| o.kind == ActionKindView::UndoManaAbility).unwrap_or_else(|| panic!("{name}: Undo is offered"));
        assert_eq!(undo.source, used.source, "{name}: on it");

        let r = &after["Requests"][0];
        let (id, seat) = (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32);
        let undone = session.answer(id, seat, &AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(&undo))).ok().unwrap();
        assert_eq!(green_in_pool(&undone), 0, "{name}: its mana is gone");
        assert_eq!(zone_of(&undone, used.source), json!("Battlefield"), "{name}: back, under the handle it had");
        assert!(options_of(&undone).iter().any(|o| o.source == used.source && AutoMana::is(o)), "{name}: and can be used again");
        assert!(!options_of(&undone).iter().any(|o| o.kind == ActionKindView::UndoManaAbility), "{name}: nothing more to take back");
    }

    #[test]
    fn sacrificed_and_spent_mana_artifacts_are_taken_back() {
        takes_back(&["Lotus Petal"], "Forest", "Lotus Petal");
        takes_back(&["Pentad Prism"], "Forest", "Pentad Prism");
        // Generous Plunderer makes the Treasure in its controller's upkeep.
        takes_back(&["Generous Plunderer"], "Mountain", "Treasure Token");
    }

    /// Chromatic Star is not taken back: going to the graveyard, it draws a card.
    #[test]
    fn chromatic_star_is_not_taken_back() {
        let mut session = Session::new(deal(&["Chromatic Star"], "Forest", 19)).ok().unwrap();
        let (_, mut step) = activate_mana_of(&mut session, "Chromatic Star");
        // Its {1} is paid with a Forest; until then nothing has happened, and the activation may be taken back.
        let mut allow_cast = false;
        loop {
            let r = &step["Requests"][0];
            let prompt: PromptPayload = serde_json::from_value(r["Prompt"].clone()).unwrap();
            if !matches!(prompt, PromptPayload::PayCostsPromptMessage(_)) {
                break;
            }
            let (id, seat) = (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32);
            step = session.answer(id, seat, &choose(&prompt, &mut allow_cast)).ok().unwrap();
        }
        assert_eq!(green_in_pool(&step), 1, "the Star's mana");
        assert!(!options_of(&step).iter().any(|o| o.kind == ActionKindView::UndoManaAbility), "no Undo");
    }

    /// A seat's sideboard is the cards it owns outside the game (CR 400.11a): its owner sees what is in it, the other seat
    /// only how many, and neither sees the other's cards among the objects of the view.
    #[test]
    fn a_sideboard_is_shown_to_its_owner_alone() {
        let mut dealt = deal(&[], "Island", 20);
        dealt.seats[0].2 = vec!["Lightning Bolt".into(), "Counterspell".into()];
        let mut session = Session::new(dealt).ok().unwrap();
        let sideboard = |view: &PlayerView, owner: i32| {
            view.zones.iter().find(|z| z.kind == Some(ZoneKindView::Sideboard) && z.owner == owner).cloned().unwrap()
        };
        let names = |view: &PlayerView, zone: &ZoneView| -> Vec<String> {
            zone.contents.iter().map(|h| view.objects.iter().find(|o| o.handle == *h).unwrap().name.clone()).collect()
        };
        let mine = session.view(1).ok().unwrap();
        let shown = sideboard(&mine, 1);
        assert!(shown.contents_visible);
        assert_eq!(names(&mine, &shown), ["Lightning Bolt", "Counterspell"]);
        let theirs = session.view(2).ok().unwrap();
        let hidden = sideboard(&theirs, 1);
        assert_eq!(hidden.count, 2);
        assert!(!hidden.contents_visible && hidden.contents.is_empty());
        assert!(!theirs.objects.iter().any(|o| o.name == "Lightning Bolt" || o.name == "Counterspell"));
        assert_eq!(sideboard(&theirs, 2).count, 0);
    }

    /// Companion (CR 103.2b, 702.139a, 116.2g): North chose Lurrus of the Dream-Den as its companion at deck building, and
    /// its deck - a Black Lotus and Islands - fulfils its condition, so it is revealed before the game with nothing asked: South
    /// sees it in North's sideboard, alone of it. Once the Lotus is out North is offered `TakeCompanion` for {3}, and taking it
    /// puts Lurrus into North's hand; it is offered no more.
    #[test]
    fn a_revealed_companion_is_shown_to_both_seats_and_taken_for_three() {
        let mut dealt = deal(&["Black Lotus"], "Island", 30);
        dealt.seats[0].2 = vec!["Lightning Bolt".into()];
        dealt.companions = vec![Some("Lurrus of the Dream-Den".into())];
        let mut session = Session::new(dealt).ok().unwrap();
        let sideboard = |view: &PlayerView| view.zones.iter().find(|z| z.kind == Some(ZoneKindView::Sideboard) && z.owner == 1).cloned().unwrap();
        let name_of = |view: &PlayerView, h: ViewHandle| view.objects.iter().find(|o| o.handle == h).map(|o| o.name.clone());
        let mut step = session.step();
        let (mut taken, mut allow_cast) = (false, true);
        for _ in 0..200 {
            let r = &step["Requests"][0];
            let prompt: PromptPayload = serde_json::from_value(r["Prompt"].clone()).unwrap();
            let (id, seat) = (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32);
            assert!(!matches!(prompt, PromptPayload::ChooseCardsPromptMessage(_)), "nothing is asked of a companion");
            let answer = match &prompt {
                PromptPayload::PriorityPromptMessage(p) if seat == 1 && p.options.iter().any(|o| o.kind == ActionKindView::TakeCompanion) => {
                    assert!(!taken, "offered only once");
                    let take = p.options.iter().find(|o| o.kind == ActionKindView::TakeCompanion).unwrap();
                    assert_eq!(take.description, "{3}: Put Lurrus of the Dream-Den in hand.");
                    // South sees the revealed card in North's sideboard, and nothing else of it.
                    let south = session.view(2).ok().unwrap();
                    let theirs = sideboard(&south);
                    assert_eq!((theirs.count, theirs.contents_visible), (2, false));
                    assert_eq!(theirs.contents.iter().map(|&h| name_of(&south, h)).collect::<Vec<_>>(), [Some("Lurrus of the Dream-Den".to_string())]);
                    assert!(!south.objects.iter().any(|o| o.name == "Lightning Bolt"));
                    taken = true;
                    AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(take))
                }
                PromptPayload::PriorityPromptMessage(_) if taken && seat == 1 => break,
                // The {3} is paid with the Lotus's three mana.
                PromptPayload::PayCostsPromptMessage(p) if taken && let Some(lotus) = p.options.iter().find(|o| o.produces_of_any_one_color == 3) => {
                    let mut answer = TakeActionAnswer::take(lotus);
                    answer.color = Some(ColorView::Blue);
                    AnswerPayload::TakeActionAnswer(answer)
                }
                _ => choose(&prompt, &mut allow_cast),
            };
            step = match session.answer(id, seat, &answer) {
                Ok(step) => step,
                Err(_) => session.answer(id, seat, &prompt.default_answer()).ok().expect("the default answer is taken"),
            };
        }
        assert!(taken, "the companion was never offered");
        let north = session.view(1).ok().unwrap();
        let hand = north.zones.iter().find(|z| z.kind == Some(ZoneKindView::Hand) && z.owner == 1).unwrap();
        assert!(hand.contents.iter().any(|&h| name_of(&north, h).as_deref() == Some("Lurrus of the Dream-Den")), "in North's hand");
        assert_eq!(sideboard(&north).count, 1, "the Lightning Bolt is left outside the game");
        let south = session.view(2).ok().unwrap();
        assert!(sideboard(&south).contents.is_empty(), "nothing of North's sideboard is shown any more");
    }

    /// A companion whose condition the deck does not fulfil - a Lurrus of the deck's own is a permanent card with mana value
    /// 3 - is not revealed (CR 103.2b): South sees North's sideboard closed with nothing in it, and North is never offered it.
    #[test]
    fn an_unfulfilled_companion_is_a_hidden_sideboard_card() {
        let mut dealt = deal(&["Black Lotus", "Lurrus of the Dream-Den"], "Island", 30);
        dealt.companions = vec![Some("Lurrus of the Dream-Den".into())];
        let mut session = Session::new(dealt).ok().unwrap();
        let south = session.view(2).ok().unwrap();
        let theirs = south.zones.iter().find(|z| z.kind == Some(ZoneKindView::Sideboard) && z.owner == 1).unwrap();
        assert_eq!((theirs.count, theirs.contents.len()), (1, 0));
        assert!(!south.objects.iter().any(|o| o.name == "Lurrus of the Dream-Den" && o.zone == Some(ZoneKindView::Sideboard)));
        let step = session.step();
        assert!(!options_of(&step).iter().any(|o| o.kind == ActionKindView::TakeCompanion));
    }

    /// The server's question before it deals a deck with a companion (CR 702.139a): which cards with companion in the sideboard the deck fulfils the condition
    /// of, judged by the cards' own conditions. A Lightning Bolt has no companion and is not listed; a deck of Islands fulfils
    /// Lurrus of the Dream-Den's, one with a Lurrus of its own (a permanent card with mana value 3) does not.
    #[test]
    fn the_server_is_told_which_companions_a_deck_fulfils() {
        let islands = |extra: &[&str]| extra.iter().map(|c| c.to_string()).chain(std::iter::repeat_n("Island".to_string(), 60)).collect::<Vec<_>>();
        let sideboard = vec!["Lightning Bolt".to_string(), "Lurrus of the Dream-Den".to_string()];
        let lurrus = |fulfilled| vec![CompanionCheckView { card: "Lurrus of the Dream-Den".into(), fulfilled }];
        assert_eq!(companions(&islands(&[]), &sideboard, 60).ok().unwrap(), lurrus(true));
        assert_eq!(companions(&islands(&["Lurrus of the Dream-Den"]), &sideboard, 60).ok().unwrap(), lurrus(false));
        assert!(companions(&islands(&[]), &["No Such Card".to_string()], 60).is_err());
        // Yorion, Sky Nomad's condition names the minimum deck size: eighty cards are twenty more than Constructed's sixty, and
        // never twenty more than Commander's hundred.
        let yorion = |fulfilled| vec![CompanionCheckView { card: "Yorion, Sky Nomad".into(), fulfilled }];
        let eighty = islands(&[]).into_iter().chain(std::iter::repeat_n("Island".to_string(), 20)).collect::<Vec<_>>();
        assert_eq!(companions(&eighty, &["Yorion, Sky Nomad".to_string()], 60).ok().unwrap(), yorion(true));
        assert_eq!(companions(&eighty[1..], &["Yorion, Sky Nomad".to_string()], 60).ok().unwrap(), yorion(false));
        assert_eq!(companions(&eighty, &["Yorion, Sky Nomad".to_string()], 100).ok().unwrap(), yorion(false));
    }

    /// A sideboard naming a card the engine does not have refuses the deal, as a deck naming one does: a card that brings a
    /// card in from outside the game chooses among the whole sideboard (CR 400.11a).
    #[test]
    fn a_sideboard_card_the_engine_does_not_have_refuses_the_deal() {
        let mut dealt = deal(&[], "Island", 20);
        dealt.seats[1].2 = vec!["No Such Card".into(), "Shock".into()];
        let refused = Session::new(dealt).err().unwrap();
        assert!(refused.detail.contains("sideboard names No Such Card"), "{}", refused.detail);
    }

    fn options_of(step: &Value) -> Vec<ActionOptionMessage> {
        match serde_json::from_value::<PromptPayload>(step["Requests"][0]["Prompt"].clone()).unwrap() {
            PromptPayload::PriorityPromptMessage(p) => p.options,
            PromptPayload::PayCostsPromptMessage(p) => p.options,
            _ => Vec::new(),
        }
    }

    fn zone_of(step: &Value, handle: ViewHandle) -> Value {
        step["Views"]["1"]["Objects"].as_array().unwrap().iter().find(|o| o["Handle"] == json!(handle)).map_or(Value::Null, |o| o["Zone"].clone())
    }

    fn green_in_pool(step: &Value) -> i64 {
        step["Views"]["1"]["Players"].as_array().unwrap().iter().map(|p| p["Mana"][4].as_i64().unwrap()).sum()
    }

    /// Black Lotus is taken back: sacrificed, it is on the battlefield again under the handle it had, its mana gone.
    #[test]
    fn a_sacrificed_black_lotus_is_taken_back() {
        let mut session = Session::new(deal(&["Black Lotus"], "Forest", 19)).ok().unwrap();
        let (lotus, after) = activate_mana_of(&mut session, "Black Lotus");
        assert_eq!(green_in_pool(&after), 3);
        assert_ne!(zone_of(&after, lotus.source), json!("Battlefield"), "sacrificed");
        let undo = options_of(&after).into_iter().find(|o| o.kind == ActionKindView::UndoManaAbility).expect("Undo is offered");

        let r = &after["Requests"][0];
        let (id, seat) = (r["RequestId"].as_u64().unwrap() as u32, r["Seat"].as_i64().unwrap() as i32);
        let undone = session.answer(id, seat, &AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(&undo))).ok().unwrap();
        assert_eq!(green_in_pool(&undone), 0, "its mana is gone");
        assert_eq!(zone_of(&undone, lotus.source), json!("Battlefield"), "back, under the handle it had");
        assert!(options_of(&undone).iter().any(|o| o.source == lotus.source && o.produces_of_any_one_color == 3), "and can be used again");
    }

    /// Lion's Eye Diamond is never taken back: its cost discards the hand, and what was in it has been seen.
    #[test]
    fn lions_eye_diamond_is_not_taken_back() {
        let mut session = Session::new(deal(&["Lion's Eye Diamond"], "Forest", 19)).ok().unwrap();
        let (_, after) = activate_mana_of(&mut session, "Lion's Eye Diamond");
        assert_eq!(green_in_pool(&after), 3);
        assert!(!options_of(&after).iter().any(|o| o.kind == ActionKindView::UndoManaAbility), "no Undo");
    }

    /// A mana ability whose cost is its own {T} names no permanent to tap: `Tap` is for a tap cost paid by a permanent
    /// of the player's choosing, and the client asks for one on the board when it is set - a Forest or a Gleemox
    /// clicked to pay for a spell would ask which permanent to tap, offering only itself.
    #[test]
    fn a_mana_ability_tapping_itself_names_no_permanent_to_tap() {
        let mut session = Session::new(deal(&["Gleemox", "Llanowar Elves"], "Forest", 18)).ok().unwrap();
        let mut step = session.step();
        let mut allow_cast = true;
        let mut checked = 0;
        for _ in 0..2_000 {
            let request = &step["Requests"][0];
            let prompt: PromptPayload = serde_json::from_value(request["Prompt"].clone()).unwrap();
            if let PromptPayload::PayCostsPromptMessage(p) = &prompt {
                for o in p.options.iter().filter(|o| o.kind == ActionKindView::ActivateAbility && !o.produces.is_empty() || o.produces_of_any_one_color > 0) {
                    assert_eq!(o.tap, None, "{}", o.description);
                    checked += 1;
                }
                if checked >= 2 {
                    return;
                }
            }
            let (id, seat) = (request["RequestId"].as_u64().unwrap() as u32, request["Seat"].as_i64().unwrap() as i32);
            let answer = choose(&prompt, &mut allow_cast);
            step = match session.answer(id, seat, &answer) {
                Ok(step) => step,
                Err(_) => session.answer(id, seat, &prompt.default_answer()).ok().expect("the default answer is taken"),
            };
        }
        panic!("no cost was paid with both mana sources on offer");
    }

    #[test]
    fn a_forced_coin_flip_reaches_every_seat() {
        for (face, won) in [(1u8, true), (2, false)] {
            let mut session = Session::new(deal(&["Mana Crypt"], "Plains", 19)).ok().unwrap();
            session.game.state.force_rolls(&[face]);
            let heard = play_until(&mut session, |events| !of_type(events, "CoinFlippedEvent").is_empty());
            for seat in ["1", "2"] {
                let flips = of_type(&heard[seat], "CoinFlippedEvent");
                assert_eq!(flips.len(), 1, "seat {seat}");
                assert_eq!((flips[0]["Player"].as_i64(), flips[0]["Won"].as_bool()), (Some(1), Some(won)), "seat {seat}");
            }
        }
    }

    /// What an ability did is the entries from its `First` up to its resolution, and nothing it did not make: Mana
    /// Crypt's trigger flipped a coin and took life, and the step, the draw and the trigger itself came before it.
    #[test]
    fn a_resolved_ability_owns_only_the_entries_its_effect_made() {
        let mut session = Session::new(deal(&["Mana Crypt"], "Plains", 19)).ok().unwrap();
        session.game.state.force_rolls(&[2]);
        let heard = play_until(&mut session, |events| {
            let flipped = events.iter().position(|e| e["$type"] == "CoinFlippedEvent");
            flipped.is_some_and(|at| events[at..].iter().any(|e| e["$type"] == "AbilityResolvedEvent"))
        });
        for seat in ["1", "2"] {
            let events = &heard[seat];
            let resolved = events.iter().find(|e| e["$type"] == "AbilityResolvedEvent").unwrap();
            let (first, at) = (resolved["First"].as_u64().unwrap(), resolved["Sequence"].as_u64().unwrap());
            let made: Vec<&str> = events
                .iter()
                .filter(|e| (first..at).contains(&e["Sequence"].as_u64().unwrap()))
                .map(|e| e["$type"].as_str().unwrap())
                .collect();
            assert_eq!(made, ["CoinFlippedEvent", "LifeChangedEvent"], "seat {seat}");
        }
    }

    #[test]
    fn a_forced_die_roll_reaches_every_seat() {
        let top = ["Mox Ruby", "Mox Pearl", "Mox Jet", "Sol Ring", "Comet, Stellar Pup"];
        let mut session = Session::new(deal(&top, "Island", 20)).ok().unwrap();
        session.game.state.force_rolls(&[2]);
        let heard = play_until(&mut session, |events| !of_type(events, "DieRolledEvent").is_empty());
        for seat in ["1", "2"] {
            let rolls = of_type(&heard[seat], "DieRolledEvent");
            assert_eq!(rolls.len(), 1, "seat {seat}");
            assert_eq!((rolls[0]["Player"].as_i64(), rolls[0]["Sides"].as_i64(), rolls[0]["Result"].as_i64()), (Some(1), Some(6), Some(2)), "seat {seat}");
        }
    }
}
