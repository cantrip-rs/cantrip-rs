//! Plays a `GameScript` against the native engine with the C# runner's semantics
//! (`src/Cantrip.Scripting/ScriptRunner.cs`): one cursor over the whole list, each line tried against whatever the
//! engine asks, the simplest answer given whenever the line under the cursor does not apply.

use std::collections::VecDeque;

use cantrip_engine::undo::Undo;
use cantrip_engine::*;

use crate::dump::{StateDump, bot};
use crate::parse::{self, ActionStep, Expectation, GameScript, HandoverStep, ScriptStep, ScriptVerb, WaitStep, bot_id, split_copy};

/// How many turns a line may wait to become possible before the script is taken to be stuck.
pub const TURNS_TO_WAIT: u32 = 4;
const ASK_LIMIT: usize = 20_000;

pub struct ScriptResult {
    pub game: Game,
    pub dump: StateDump,
    pub trace: Vec<String>,
    pub handed_over: bool,
}

#[derive(Debug, Clone)]
pub struct ScriptFailure {
    pub message: String,
    pub report: String,
}

impl std::fmt::Display for ScriptFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.report)
    }
}

struct Fail(String);

type R<T> = Result<T, Fail>;

fn fail<T>(message: impl Into<String>) -> R<T> {
    Err(Fail(message.into()))
}

/// A cast or activation in progress, from the line that began it: what `Spend:` still has to say.
struct Casting {
    player: PlayerId,
    /// The X or kicker count the line asked for (`X:`, `Kicked:`).
    x: Option<u16>,
    spend: VecDeque<Spend>,
    /// The creatures a `Convoke:` taps, in the order the payment asks (CR 702.51a), and how many cards `Delve:` exiles
    /// from the graveyard, the first offered each time (CR 702.66a).
    convoke: VecDeque<String>,
    delve: u32,
}

impl Casting {
    fn new(player: PlayerId, x: Option<u16>, spend: VecDeque<Spend>) -> Casting {
        Casting { player, x, spend, convoke: VecDeque::new(), delve: 0 }
    }
}

/// What the script means a player to answer a question with.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Answer(Answer),
    /// Take back the latest mana ability (`Bot1.Undo`).
    Undo,
}

/// Where playing on stopped.
enum Stop<'s> {
    Done,
    Asked(Request),
    Check(&'s Expectation),
}

/// An intention, and the line it comes from (none for a default or a payment the line in progress implies).
struct Decision<'s> {
    intent: Intent,
    line: Option<&'s ActionStep>,
}

struct Run<'s> {
    script: &'s GameScript,
    game: Game,
    cursor: usize,
    since: u32,
    casting: Option<Casting>,
    trace: Vec<String>,
    handed_over: bool,
    /// The colors an activation line named (`Colors: Red, Red, Green`) for the "any combination" question its mana
    /// ability asks next, as the C# runner takes them.
    colors: Option<Vec<Color>>,
    /// The mana abilities that can be taken back (`Bot1.Undo`), as the host offers them.
    undo: Undo<()>,
    /// A driven player's question, and what the script means them to answer, until they do.
    asked: Option<(Request, Decision<'s>)>,
}

pub fn run(script: &GameScript) -> Result<ScriptResult, ScriptFailure> {
    run_on(script, cards())
}

/// `run` on a card table of the caller's (`cantrip_engine::deal_on`): every card and some test-only ones, for a Rust test
/// that plays a scenario with a card no printed card is.
pub fn run_on(script: &GameScript, table: &'static CardTable) -> Result<ScriptResult, ScriptFailure> {
    let mut run = Run::deal_on(script, table)?;
    match run.play() {
        Ok(()) => {
            let dump = run.judge()?;
            Ok(ScriptResult { game: run.game, dump, trace: run.trace, handed_over: run.handed_over })
        }
        Err(Fail(message)) => Err(run.failure(message)),
    }
}

/// A scenario played through a table rather than by the runner alone (`docs/scenarios-in-the-browser.md`): the runner
/// answers for the seats it plays, and stops on each question to a driven seat - a browser page, say - with what the
/// script means it to answer, which the seat's answer must then be.
pub struct Scripted {
    run: Run<'static>,
}

/// Where a scenario played through a table stopped.
pub enum Advance {
    /// A driven seat is asked something, and the script means this answer.
    Ask(Expected),
    /// A check of what a client shows (`? Client.Bot1.Stack = ...`), to be judged by the driven seats' pages now.
    Check(Expectation),
    /// The script is over: [`Scripted::verdict`] judges the game, and [`Scripted::client_expectations`] are for the pages.
    Done,
}

/// A driven seat's question, and what the script means it to answer.
pub struct Expected {
    pub request: Request,
    pub intent: Intent,
    /// The line it comes from, `12: Bot1.Hand.Island.Play`, or none for a default or a payment the line in progress implies.
    pub line: Option<String>,
}

impl Scripted {
    /// Deals the scenario's game. The script lives as long as the process: a table plays few.
    pub fn deal(script: GameScript) -> Result<Scripted, ScriptFailure> {
        let script: &'static GameScript = Box::leak(Box::new(script));
        Ok(Scripted { run: Run::deal(script)? })
    }

    pub fn script(&self) -> &GameScript {
        self.run.script
    }

    /// The game the scenario is played in, which a table may hold in its stead between steps (`std::mem::swap`).
    pub fn game(&mut self) -> &mut Game {
        &mut self.run.game
    }

    /// Plays on, answering for every player but the `driven` ones, until a driven player is asked (`Some`) or the
    /// script ends (`None`; [`Scripted::verdict`] then judges it).
    pub fn advance(&mut self, driven: &[PlayerId]) -> Result<Advance, ScriptFailure> {
        match self.run.play_until(driven) {
            Ok(Stop::Done) => Ok(Advance::Done),
            Ok(Stop::Check(check)) => Ok(Advance::Check(check.clone())),
            Ok(Stop::Asked(request)) => {
                let (_, decision) = self.run.asked.as_ref().expect("held");
                let line = decision.line.map(|step| format!("{}: {}", step.line, step.text));
                Ok(Advance::Ask(Expected { request, intent: decision.intent.clone(), line }))
            }
            Err(Fail(message)) => Err(self.run.failure(message)),
        }
    }

    /// The permanent whose mana ability `player` may take back now, as the script's game has it.
    pub fn undo_offered(&self, player: PlayerId) -> Option<ObjectId> {
        self.run.undo.offered(player)
    }

    /// The `[Expect]` lines about what a client shows, for the pages to judge once the script is over.
    pub fn client_expectations(&self) -> Vec<Expectation> {
        self.run.script.expectations.iter().filter(|e| e.is_client()).cloned().collect()
    }

    /// The driven player's answer to the question `advance` stopped on. A failure if it is not the one the script means.
    pub fn answered(&mut self, given: Intent) -> Result<(), ScriptFailure> {
        self.run.answered(given).map_err(|Fail(message)| self.run.failure(message))
    }

    /// Whether the game ended as the scenario expects, once `advance` has said it is over.
    pub fn verdict(&mut self) -> Result<StateDump, ScriptFailure> {
        self.run.judge()
    }

    /// A failure of the scenario found outside the runner, with the runner's report.
    pub fn failure(&self, message: String) -> ScriptFailure {
        self.run.failure(message)
    }
}

impl<'s> Run<'s> {
    fn deal(script: &'s GameScript) -> Result<Run<'s>, ScriptFailure> {
        Run::deal_on(script, cards())
    }

    fn deal_on(script: &'s GameScript, table: &'static CardTable) -> Result<Run<'s>, ScriptFailure> {
        let decks: Vec<Vec<String>> = (0..script.decks.len()).map(|i| script.decklist(i)).collect();
        let sideboards: Vec<Vec<String>> = (0..script.sideboards.len()).map(|i| script.sideboard(i)).collect();
        let chooser = if script.opening { Some(PlayerId((script.chooser.max(1) - 1) as u8)) } else { None };
        let mut game = deal_on(table, &decks, &sideboards, &script.basic_lands_outside, 1, false, chooser).map_err(|m| ScriptFailure { message: m.clone(), report: m })?;
        // CANTRIP_VERIFY=1: every rollback is checked to restore the state exactly.
        game.verify = std::env::var_os("CANTRIP_VERIFY").is_some();
        game.state.force_rolls(&script.dice);
        for &(seat, life) in &script.life {
            if let Some(player) = game.state.players.get_mut(seat) {
                player.life = life;
            }
        }
        if let Some(limit) = script.event_limit {
            game.max_events = limit;
        }
        Ok(Run {
            script,
            game,
            cursor: 0,
            since: 1,
            casting: None,
            trace: Vec::new(),
            handed_over: false,
            colors: None,
            undo: Undo::default(),
            asked: None,
        })
    }

    /// The game as it ended, judged against `[Expect]`.
    fn judge(&self) -> Result<StateDump, ScriptFailure> {
        let dump = StateDump::of(&self.game);
        let mismatches = dump.mismatches(&self.script.expectations);
        if !mismatches.is_empty() {
            return Err(self.failure(format!("the game did not end as expected:
  {}", mismatches.join("
  "))));
        }
        Ok(dump)
    }
}

fn mana_symbol(text: &str) -> Option<Mana> {
    match text.trim().to_ascii_uppercase().as_str() {
        "W" => Some(Mana::W),
        "U" => Some(Mana::U),
        "B" => Some(Mana::B),
        "R" => Some(Mana::R),
        "G" => Some(Mana::G),
        "C" => Some(Mana::C),
        _ => None,
    }
}

fn is_colorless(text: &str) -> bool {
    let t = text.trim();
    t.eq_ignore_ascii_case("C") || t.eq_ignore_ascii_case("Colorless")
}

fn color_named(text: &str, short: bool) -> Option<Color> {
    let t = text.trim();
    for c in Color::ALL {
        if t.eq_ignore_ascii_case(c.name()) || (short && t.len() == 1 && t.eq_ignore_ascii_case(&c.symbol().to_string())) {
            return Some(c);
        }
    }
    None
}

fn engine_zone(zone: parse::ZoneKind) -> Option<ZoneKind> {
    Some(match zone {
        parse::ZoneKind::Library => ZoneKind::Library,
        parse::ZoneKind::Hand => ZoneKind::Hand,
        parse::ZoneKind::Battlefield => ZoneKind::Battlefield,
        parse::ZoneKind::Graveyard => ZoneKind::Graveyard,
        parse::ZoneKind::Stack => ZoneKind::Stack,
        parse::ZoneKind::Exile => ZoneKind::Exile,
        parse::ZoneKind::Sideboard => ZoneKind::Sideboard,
        _ => return None,
    })
}

fn engine_step(step: parse::TurnStep) -> Step {
    use parse::TurnStep as T;
    match step {
        T::Untap => Step::Untap,
        T::Upkeep => Step::Upkeep,
        T::Draw => Step::Draw,
        T::PrecombatMain => Step::PrecombatMain,
        T::BeginCombat => Step::BeginCombat,
        T::DeclareAttackers => Step::DeclareAttackers,
        T::DeclareBlockers => Step::DeclareBlockers,
        T::CombatDamage => Step::CombatDamage,
        T::EndOfCombat => Step::EndOfCombat,
        T::PostcombatMain => Step::PostcombatMain,
        T::End => Step::End,
        T::Cleanup => Step::Cleanup,
    }
}

fn player_of(bot: i32) -> PlayerId {
    PlayerId((bot - 1) as u8)
}

/// Whether a verb answers a kind of question (`Answers` in the C# runner).
fn answers(verb: ScriptVerb, kind: &RequestKind) -> bool {
    use ScriptVerb as V;
    match verb {
        V::Play | V::Cast | V::Activate | V::Foretell | V::Suspend | V::Pass => matches!(kind, RequestKind::Priority { .. }),
        V::PayCost => matches!(kind, RequestKind::PayCosts { .. }),
        V::Undo => matches!(kind, RequestKind::Priority { .. } | RequestKind::PayCosts { .. }),
        V::Accept | V::Decline => matches!(kind, RequestKind::May { .. } | RequestKind::UnlessPays { .. }),
        V::Choose => matches!(kind, RequestKind::ChooseCards { .. } | RequestKind::TapForPower { .. } | RequestKind::CollectEvidence { .. } | RequestKind::Craft { .. }),
        V::ChoosePlayer => matches!(kind, RequestKind::ChoosePlayer { .. }),
        V::Discard => matches!(kind, RequestKind::DiscardToHandSize { .. }),
        V::Attack => matches!(kind, RequestKind::DeclareAttackers { .. }),
        V::Block => matches!(kind, RequestKind::DeclareBlockers { .. }),
        V::AssignDamage => matches!(kind, RequestKind::AssignCombatDamage { .. }),
        V::Modes => matches!(kind, RequestKind::ChooseModes { .. } | RequestKind::ChooseSubtype { .. }),
        V::Target => matches!(kind, RequestKind::ChooseTarget { .. }),
        V::Divide => matches!(kind, RequestKind::ChooseDividedTargets { .. }),
        V::RemoveCounters => matches!(kind, RequestKind::RemoveCounters { .. }),
        V::Colors => matches!(kind, RequestKind::ChooseColors { .. }),
        V::PlayFirst | V::DrawFirst => matches!(kind, RequestKind::ChooseStartingPlayer { .. }),
        V::Mulligan | V::Keep => matches!(kind, RequestKind::Mulligan { .. }),
        V::Triggers => matches!(kind, RequestKind::OrderTriggers { .. }),
        V::Arrange => matches!(kind, RequestKind::DistributeCards { .. }),
        V::Search => matches!(kind, RequestKind::SearchLibrary { .. }),
        V::Replace => matches!(kind, RequestKind::ChooseReplacement { .. }),
        V::Name => matches!(kind, RequestKind::ChooseName { .. } | RequestKind::ChooseModes { .. }),
        _ => false,
    }
}

fn kind_name(kind: &RequestKind) -> &'static str {
    match kind {
        RequestKind::Priority { .. } => "Priority",
        RequestKind::PayCosts { .. } => "PayCosts",
        RequestKind::UnlessPays { .. } => "UnlessPays",
        RequestKind::ChooseCards { .. } => "ChooseCards",
        RequestKind::ChooseTarget { .. } => "ChooseTarget",
        RequestKind::ChooseModes { .. } => "ChooseModes",
        RequestKind::ChooseColors { .. } => "ChooseColors",
        RequestKind::May { .. } => "May",
        RequestKind::ChoosePlayer { .. } => "ChoosePlayer",
        RequestKind::TapForPower { .. } => "TapForPower",
        RequestKind::CollectEvidence { .. } => "CollectEvidence",
        RequestKind::Craft { .. } => "Craft",
        RequestKind::RemoveCounters { .. } => "RemoveCounters",
        RequestKind::DeclareAttackers { .. } => "DeclareAttackers",
        RequestKind::DeclareBlockers { .. } => "DeclareBlockers",
        RequestKind::AssignCombatDamage { .. } => "AssignCombatDamage",
        RequestKind::DiscardToHandSize { .. } => "DiscardToHandSize",
        RequestKind::Mulligan { .. } => "Mulligan",
        RequestKind::ChooseStartingPlayer { .. } => "ChooseStartingPlayer",
        RequestKind::OrderTriggers { .. } => "OrderTriggers",
        RequestKind::DistributeCards { .. } => "DistributeCards",
        RequestKind::SearchLibrary { .. } => "SearchLibrary",
        RequestKind::ChooseReplacement { .. } => "ChooseReplacement",
        RequestKind::ChooseName { .. } => "ChooseName",
        RequestKind::ChooseSubtype { .. } => "ChooseSubtype",
        RequestKind::ChooseDividedTargets { .. } => "ChooseDividedTargets",
    }
}

fn pays(owed: &ManaCost, mana: Mana) -> bool {
    owed.generic > 0 || owed.symbols[mana as usize] > 0 || owed.hybrid.iter().flatten().any(|(a, b)| *a == mana || *b == mana)
}

impl<'s> Run<'s> {
    fn current(&self) -> Option<&'s ScriptStep> {
        self.script.steps.get(self.cursor)
    }

    fn turn(&self) -> u32 {
        self.game.state.turn.number
    }

    fn next(&mut self) {
        self.cursor += 1;
        self.since = self.turn().max(1);
    }

    fn failure(&self, message: String) -> ScriptFailure {
        let mut report = format!("{}: {message}\n", self.script.name);
        if let Some(r) = self.game.pending() {
            report.push_str(&format!("\nOutstanding: {} asked {}\n", bot(r.player), self.describe_request(r)));
        }
        report.push_str("\nTrace:\n");
        for line in self.trace.iter().rev().take(40).rev() {
            report.push_str(&format!("  {line}\n"));
        }
        report.push_str("\nState:\n");
        report.push_str(&StateDump::of(&self.game).render());
        ScriptFailure { message, report }
    }

    fn describe_request(&self, r: &Request) -> String {
        let g = &self.game;
        let name = |id: ObjectId| g.known_name(id).to_string();
        match &r.kind {
            RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } => {
                let list: Vec<String> = options
                    .iter()
                    .map(|o| {
                        let src = o.source.map(name).unwrap_or_default();
                        let desc = describe(g.cards, &g.state, &g.log, o);
                        format!("{:?} {src} #{} {desc}{}", o.kind, o.index, o.mana.map(|m| format!(" {m:?}")).unwrap_or_default())
                    })
                    .collect();
                format!("{} [{}]", kind_name(&r.kind), list.join(" | "))
            }
            RequestKind::ChooseCards { rule, .. } => {
                let words: String = rule.words(&|id| self.game.cards.get(CardId(id as u16)).name.to_string()).iter().map(|w| format!(" {w}")).collect();
                let eligible = first_choices(g, r);
                format!("ChooseCards {} of [{}]{words}", count_words(rule.count), eligible.iter().map(|&i| name(i)).collect::<Vec<_>>().join(", "))
            }
            RequestKind::Craft { cards, minimum, maximum, .. } => {
                format!("Craft {minimum} to {maximum} of [{}]", cards.iter().map(|&i| name(i)).collect::<Vec<_>>().join(", "))
            }
            RequestKind::RemoveCounters { pools, total, up_to, .. } => {
                let list: Vec<String> = pools.iter().map(|p| format!("{}: {} {}", name(p.object), p.kind.token(), p.count)).collect();
                let up_to = if *up_to { "up to " } else { "" };
                format!("RemoveCounters {up_to}{total} from among [{}]", list.join("; "))
            }
            other => format!("{} {other:?}", kind_name(other)),
        }
    }

    fn deliver(&mut self, request: &Request, answer: Answer, step: Option<&ActionStep>) -> R<()> {
        let who = bot(request.player);
        match step {
            Some(s) => self.trace.push(format!("T{} {}: line {} {}", self.turn(), who, s.line, s.text)),
            None => {
                if !matches!(answer, Answer::Pass) {
                    self.trace.push(format!("T{} {}: (default) {answer:?} to {}", self.turn(), who, kind_name(&request.kind)));
                }
            }
        }
        let noted = Undo::<()>::before(&self.game, request.player, &answer);
        match self.game.answer(request.id, request.player, answer.clone()) {
            Ok(()) => {
                if self.undo.may_take_back(&self.game, &noted) {
                    self.undo.push(noted, ());
                }
                Ok(())
            }
            Err(Rejected(why)) => match step {
                Some(s) => fail(format!("line {}: the engine refused {}: {why}", s.line, s.text)),
                None => fail(format!("the engine refused {who}'s unscripted answer ({answer:?}): {why}")),
            },
        }
    }

    fn play(&mut self) -> R<()> {
        match self.play_until(&[])? {
            Stop::Done => Ok(()),
            _ => unreachable!("no player is driven"),
        }
    }

    /// Plays on, answering for every player but the `driven` ones, until the script ends, a driven player is asked (its
    /// intention held in `asked` until [`Run::answered`]), or, with a player driven, a check of what a client shows comes up.
    fn play_until(&mut self, driven: &[PlayerId]) -> R<Stop<'s>> {
        let mut asks = 0;
        loop {
            if let Some(check) = self.run_checks(!driven.is_empty())? {
                return Ok(Stop::Check(check));
            }
            if self.game.is_over() {
                if let Some(step) = self.current() {
                    return fail(format!("the game ended before line {}: {}", step.line(), step.text()));
                }
                return Ok(Stop::Done);
            }
            asks += 1;
            if asks > ASK_LIMIT {
                return fail("the game went on too long: looping?");
            }
            let Some(request) = self.game.pending().cloned() else {
                return fail("the engine stopped with nothing to ask and the game not over");
            };

            let stack_empty = self.game.state.stack.is_empty();
            if self.current().is_none()
                && self.casting.is_none()
                && stack_empty
                && matches!(request.kind, RequestKind::Priority { .. })
            {
                return Ok(Stop::Done);
            }

            if let Some(ScriptStep::Handover(h)) = self.current()
                && self.waiting(h, &request)
            {
                self.handed_over = true;
                self.next();
                return Ok(Stop::Done);
            }

            // `! Bot1.Accept`, `! Bot1.Choose`, a bare `! Bot1.Target` or `! Bot2.Block`...: the player is not asked that kind of question
            // before their next priority (a bare Block: no block is possible, so the declaration is never asked for).
            if let Some(ScriptStep::Refused { action, line, text }) = self.current()
                && ((action.verb.is_question() && !(matches!(action.verb, ScriptVerb::Choose | ScriptVerb::Colors | ScriptVerb::Name) && action.positional().next().is_some())
                    && !(matches!(action.verb, ScriptVerb::Arrange) && !action.raw_arguments.trim().is_empty()))
                    || (matches!(action.verb, ScriptVerb::Target | ScriptVerb::Modes | ScriptVerb::Block)
                        && action.positional().next().is_none()
                        && action.named("Count").is_none()))
                && request.player == player_of(action.bot)
            {
                match request.kind {
                    ref k if answers(action.verb, k) => return fail(format!("line {line} says {text}, but the player was asked that question")),
                    RequestKind::Priority { .. } => {
                        self.next();
                        continue;
                    }
                    _ => {}
                }
            }

            // A player who has lost has left the game and is asked nothing, priority included (CR 800.4a): the refusal of a
            // question to them holds once the stack they might have been asked about has emptied, with no such question put.
            if let Some(ScriptStep::Refused { action, .. }) = self.current()
                && action.verb.is_question()
                && self.game.state.player(player_of(action.bot)).lost
                && stack_empty
                && matches!(request.kind, RequestKind::Priority { .. })
            {
                self.next();
                continue;
            }

            // `! Bot2.Block(Blocker > Attacker)` when the declare-blockers question never came: an effect (Headliner Scarlett's) left
            // the defender no legal block at all, so the engine asks nothing. The refusal holds if, with priority in the
            // declare-blockers step, the blocker is an untapped creature of that player, the attacker is attacking that player,
            // and no block was made.
            if let Some(ScriptStep::Refused { action, line, text }) = self.current()
                && matches!(action.verb, ScriptVerb::Block)
                && matches!(request.kind, RequestKind::Priority { .. })
                && self.game.state.turn.step() == Step::DeclareBlockers
            {
                self.check_block_never_asked(action, *line, text)?;
                self.next();
                continue;
            }

            // `! Bot1.Hand.Fire Covenant.Cast(X: 6)` after the cast that announces X: X cannot be announced that high. The refusal
            // holds if the most the question lets be announced is short of 6 (life to pay, creatures to sacrifice) and fails if 6 could
            // be. The card announcing must be the one named, or the refusal proves nothing.
            if let Some(ScriptStep::Refused { action, line, text }) = self.current()
                && matches!(action.verb, ScriptVerb::Cast | ScriptVerb::Activate)
                && let Some(wanted) = action.named("X")
                && request.player == player_of(action.bot)
                && let RequestKind::PayCosts { announcing: true, source, most, .. } = &request.kind
            {
                let Ok(wanted) = wanted.trim().parse::<u32>() else { return fail(format!("line {line}: {wanted} is not a number")) };
                if !action.card.as_deref().is_some_and(|card| self.game.known_name(*source).eq_ignore_ascii_case(card)) {
                    return fail(format!("line {line}: {text} proves nothing: {} is the one announcing X", self.game.known_name(*source)));
                }
                if *most >= wanted {
                    return fail(format!("line {line} says {text} must not be allowed, but X could be announced up to {most}"));
                }
                self.next();
                continue;
            }

            // `Bot1.PayCost(X: 3)`: "you may pay {X}" with X chosen as it is paid (CR 107.3f) raises or lowers X to what the line
            // says, one step at a time, and then pays it as `PayCost` does.
            if let Some(ScriptStep::Action(step)) = self.current()
                && step.verb == ScriptVerb::PayCost
                && request.player == player_of(step.bot)
                && let RequestKind::UnlessPays { options, x: Some(x), .. } = &request.kind
                && let Some(wanted) = x_of(step)?
                && wanted != *x
            {
                let (line, text) = (step.line, step.text.clone());
                let kind = if wanted > *x { ActionKind::IncreaseX } else { ActionKind::DecreaseX };
                match options.iter().find(|o| o.kind == kind) {
                    Some(o) => {
                        let answer = Answer::Act(Action::of(o));
                        self.deliver(&request, answer, None)?;
                    }
                    None => return fail(format!("line {line}: {text}: X cannot be chosen as {wanted} (it stops at {x})")),
                }
                continue;
            }

            // `! Bot1.PayCost`: at the player's next "unless pays" question, paying is not among the options.
            if let Some(ScriptStep::Refused { action, line, text }) = self.current()
                && matches!(action.verb, ScriptVerb::PayCost)
                && request.player == player_of(action.bot)
                && let RequestKind::UnlessPays { options, .. } = &request.kind
            {
                if options.iter().any(|o| o.kind == ActionKind::PayCost) {
                    return fail(format!("line {line} says {text} must not be offered, but it is"));
                }
                self.next();
                continue;
            }

            // `! Bot1.Battlefield.Llanowar Elves.Activate` right after an `Attack` line: while the declaration's cost to attack is
            // paid (CR 508.1i), that mana ability is not offered - the creature was tapped by attacking (CR 508.1f).
            if let Some(ScriptStep::Refused { action, line, text }) = self.current()
                && matches!(action.verb, ScriptVerb::Activate)
                && request.player == player_of(action.bot)
                && let RequestKind::PayCosts { options, attackers, .. } = &request.kind
                && !attackers.is_empty()
            {
                if !self.offered(action, options, true)?.is_empty() {
                    return fail(format!("line {line} says {text} must not be offered while paying to attack, but it is"));
                }
                self.next();
                continue;
            }

            // `! Bot1.Undo`: at the player's next priority or payment, there is no mana ability to take back.
            if let Some(ScriptStep::Refused { action, line, text }) = self.current()
                && matches!(action.verb, ScriptVerb::Undo)
                && request.player == player_of(action.bot)
                && answers(action.verb, &request.kind)
            {
                if let Some(source) = self.undo.offered(request.player) {
                    return fail(format!("line {line} says {text}, but {}'s mana ability may be taken back", self.game.known_name(source)));
                }
                self.next();
                continue;
            }

            if let Some(ScriptStep::Refused { action, line, text }) = self.current()
                && request.player == player_of(action.bot)
                && answers(action.verb, &request.kind)
            {
                if let RequestKind::Priority { options } = &request.kind {
                    let mut offered = self.offered(action, options, true)?;
                    // A refusal that names targets (`! Bot2.Hand.Lightning Bolt.Cast(Bot1.Battlefield.Emrakul)`) refuses only
                    // the option that could be announced with them, and proves nothing when no such object exists.
                    let specs: Vec<&str> = action.positional().collect();
                    if !specs.is_empty() {
                        let all: Vec<ObjectId> = (0..self.game.state.objects.len()).map(|i| ObjectId(i as u32)).collect();
                        for spec in &specs {
                            if bot_id(spec).is_none() && !spec.to_ascii_lowercase().starts_with("stack.") && self.objects(action, spec, &all)?.is_empty() {
                                return fail(format!("line {line}: {text} proves nothing: nothing is named {spec}"));
                            }
                        }
                        let mut kept = Vec::new();
                        for o in offered {
                            let mut every = true;
                            for spec in &specs {
                                every &= self.target(action, spec, &o.target_candidates)?.is_some();
                            }
                            if every {
                                kept.push(o);
                            }
                        }
                        offered = kept;
                    }
                    if !offered.is_empty() {
                        return fail(format!("line {line} says {text} must not be offered, but it is"));
                    }
                }
                self.check_refused_declaration(action, *line, text, &request)?;
                self.next();
                continue;
            }

            self.fail_if_stuck()?;

            if let Some(ScriptStep::Wait(w)) = self.current()
                && self.reached(w)
            {
                self.next();
                continue;
            }

            if driven.contains(&request.player) {
                let decision = self.intend(&request)?;
                self.asked = Some((request.clone(), decision));
                return Ok(Stop::Asked(request));
            }
            self.decide(&request)?;
        }
    }

    /// Runs the checks under the cursor. A check of what a client shows is handed out when a client is watching
    /// (`watched`) and skipped when none is.
    fn run_checks(&mut self, watched: bool) -> R<Option<&'s Expectation>> {
        while let Some(ScriptStep::Check { expectation, .. }) = self.current() {
            if expectation.is_client() {
                self.next();
                if watched {
                    return Ok(Some(expectation));
                }
                continue;
            }
            let dump = StateDump::of(&self.game);
            let mismatches = dump.mismatches(std::slice::from_ref(expectation));
            if let Some(m) = mismatches.first() {
                return fail(format!("a check did not hold: {m}"));
            }
            self.next();
        }
        Ok(None)
    }

    fn waiting(&self, h: &HandoverStep, request: &Request) -> bool {
        match h.verb {
            None => true,
            Some(verb) => (h.bot == 0 || request.player == player_of(h.bot)) && answers(verb, &request.kind),
        }
    }

    fn reached(&self, w: &WaitStep) -> bool {
        let t = &self.game.state.turn;
        t.active == player_of(w.bot) && t.step() == engine_step(w.step) && w.turn.is_none_or(|n| n == t.number)
    }

    fn fail_if_stuck(&self) -> R<()> {
        let turn = self.turn();
        match self.current() {
            Some(ScriptStep::Wait(w)) => {
                if let Some(t) = w.turn {
                    if turn > t {
                        return fail(format!("turn {t} went by without reaching line {}: {}", w.line, w.text));
                    }
                } else if turn > self.since + TURNS_TO_WAIT {
                    return fail(format!("line {} was not reached in {TURNS_TO_WAIT} turns: {}", w.line, w.text));
                }
            }
            Some(step @ (ScriptStep::Handover(_) | ScriptStep::Refused { .. } | ScriptStep::Action(_))) => {
                if turn > self.since + TURNS_TO_WAIT {
                    return fail(format!(
                        "line {} did not become possible in {TURNS_TO_WAIT} turns: {}",
                        step.line(),
                        step.text()
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// A driven player answered the question `play_until` stopped on: it must be the answer the script means, which is
    /// then given (`None` is a mana ability taken back).
    fn answered(&mut self, given: Intent) -> R<()> {
        let Some((request, decision)) = self.asked.take() else {
            return fail("an answer came with nothing asked");
        };
        if !self.same_but_for_interchangeable(&given, &decision.intent) {
            let what = match decision.line {
                Some(step) => format!("line {}: {}", step.line, step.text),
                None => "the default".to_string(),
            };
            return fail(format!("{} answered {given:?} to {}, but {what} means {:?}", bot(request.player), kind_name(&request.kind), decision.intent));
        }
        self.carry_out(&request, decision)
    }

    /// Whether two answers are the same once objects nobody could tell apart are taken for one another: two untapped
    /// Forests of one player, two Islands in a hand (scenarios/README.md: same-named cards in one place are
    /// interchangeable). A page clicking the top of a pile of Forests answers what the script means by "a Forest".
    fn same_but_for_interchangeable(&self, given: &Intent, meant: &Intent) -> bool {
        match (given, meant) {
            (Intent::Undo, Intent::Undo) => true,
            (Intent::Answer(a), Intent::Answer(b)) => a == b || self.canonical(a) == self.canonical(b),
            _ => false,
        }
    }

    /// The answer with each object it names replaced by the first object it is indistinguishable from.
    fn canonical(&self, answer: &Answer) -> Answer {
        let s = &self.game.state;
        let signature = |id: ObjectId| {
            let o = s.obj(id);
            // Summoning sickness tells creatures apart (CR 302.6), and nothing else.
            let sick = o.controlled_since == s.turn.number && self.game.chars(id).is(CardType::Creature);
            (self.game.known_name(id), o.owner, o.controller, o.zone, o.token, o.tapped, o.damage, o.counter_array(), sick)
        };
        let one = |id: ObjectId| -> ObjectId {
            if id.index() >= s.objects.len() {
                return id;
            }
            let wanted = signature(id);
            (0..s.objects.len() as u32).map(ObjectId).find(|&other| live(s, other) && signature(other) == wanted).unwrap_or(id)
        };
        let target = |t: Target| match t {
            Target::Object(id) => Target::Object(one(id)),
            other => other,
        };
        match answer {
            Answer::Act(a) => {
                let mut a = a.clone();
                a.source = a.source.map(one);
                a.targets = a.targets.iter().map(|&t| target(t)).collect();
                Answer::Act(a)
            }
            Answer::Cards(ids) => Answer::Cards(ids.iter().map(|&id| one(id)).collect()),
            Answer::Targets(ts) => Answer::Targets(ts.iter().map(|&t| target(t)).collect()),
            Answer::Attack(v) => Answer::Attack(v.iter().map(|&(a, t)| (one(a), target(t))).collect()),
            Answer::Block(v) => Answer::Block(v.iter().map(|&(b, a)| (one(b), one(a))).collect()),
            Answer::Shares(v) => Answer::Shares(v.iter().map(|&(a, c, n)| (one(a), c.map(one), n)).collect()),
            Answer::Placements { placements, shuffle } => {
                Answer::Placements { placements: placements.iter().map(|p| p.iter().map(|&id| one(id)).collect()).collect(), shuffle: *shuffle }
            }
            Answer::Divided(v) => Answer::Divided(v.iter().map(|&(t, n)| (target(t), n)).collect()),
            Answer::Counters(v) => Answer::Counters(v.iter().map(|&(o, k, n)| (one(o), k, n)).collect()),
            other => other.clone(),
        }
    }

    fn decide(&mut self, request: &Request) -> R<()> {
        let decision = self.intend(request)?;
        self.carry_out(request, decision)
    }

    /// What the script means the asked player to answer: the line under the cursor if it applies, else the payment the
    /// line in progress implies, else the default. Called once per question: it moves the payment in progress on.
    fn intend(&mut self, request: &Request) -> R<Decision<'s>> {
        // `Bot1.Undo`: the latest mana ability the player activated is taken back, and they are asked again.
        if let Some(ScriptStep::Action(step)) = self.current()
            && matches!(step.verb, ScriptVerb::Undo)
            && request.player == player_of(step.bot)
            && answers(step.verb, &request.kind)
        {
            if self.undo.offered(request.player).is_none() {
                return fail(format!("line {}: {} has no mana ability to take back", step.line, bot(request.player)));
            }
            return Ok(Decision { intent: Intent::Undo, line: Some(step) });
        }

        let paying = self.casting.as_ref().is_some_and(|c| c.player == request.player)
            && matches!(request.kind, RequestKind::PayCosts { .. });
        if !paying {
            if matches!(request.kind, RequestKind::Priority { .. }) {
                self.casting = None;
            }
        } else if let Some(auto) = self.auto_pay(request)? {
            return Ok(Decision { intent: Intent::Answer(auto), line: None });
        }

        if let Some(ScriptStep::Action(step)) = self.current()
            && request.player == player_of(step.bot)
            && let Some(answer) = self.matches(step, request)?
        {
            return Ok(Decision { intent: Intent::Answer(answer), line: Some(step) });
        }

        Ok(Decision { intent: Intent::Answer(self.default(request)?), line: None })
    }

    /// Gives the answer `intend` decided on, and moves the cursor past its line.
    fn carry_out(&mut self, request: &Request, decision: Decision<'s>) -> R<()> {
        let answer = match decision.intent {
            Intent::Undo => {
                let step = decision.line.expect("an Undo line");
                if self.undo.take_back(&mut self.game, request.player).is_none() {
                    return fail(format!("line {}: {} has no mana ability to take back", step.line, bot(request.player)));
                }
                self.trace.push(format!("T{} {}: line {} {}", self.turn(), bot(request.player), step.line, step.text));
                self.next();
                return Ok(());
            }
            Intent::Answer(answer) => answer,
        };
        let Some(step) = decision.line else {
            return self.deliver(request, answer, None);
        };
        self.next();
        self.deliver(request, answer.clone(), Some(step))?;
        let act = match &answer {
            Answer::Act(a) => Some(a.kind),
            _ => None,
        };
        match (&request.kind, act) {
            (RequestKind::Priority { .. }, Some(ActionKind::CastSpell | ActionKind::ActivateAbility)) => {
                self.colors = colors_of(step)?;
                let x = x_of(step)?;
                let mut casting = Casting::new(request.player, x, spend_of(step)?);
                casting.convoke = names_of(step, "Convoke");
                casting.delve = match step.named("Delve") {
                    None => 0,
                    Some(text) => match text.trim().parse::<u32>() {
                        Ok(n) => n,
                        Err(_) => return fail(format!("line {}: {text} is not a number", step.line)),
                    },
                };
                self.casting = Some(casting);
            }
            // `Bot1.Accept(X: 5)`: a cast the "may" makes as it resolves announces that X (CR 107.3a), as a miracle cost that
            // includes {X} does.
            (RequestKind::May { .. }, None) if answer == Answer::Yes && step.named("X").is_some() => {
                self.casting = Some(Casting::new(request.player, x_of(step)?, spend_of(step)?));
            }
            // `Bot1.Choose(Burst Lightning; Kicked: 1)`: the card chosen to be cast as an effect resolves ("you may cast a
            // spell from your hand without paying its mana cost") announces that X or those kicks (CR 107.3a, 118.9d, 601.2b).
            (RequestKind::ChooseCards { purpose: Purpose::CastFree, .. }, None) if step.named("X").or_else(|| step.named("Kicked")).is_some() => {
                self.casting = Some(Casting::new(request.player, x_of(step)?, spend_of(step)?));
            }
            (RequestKind::UnlessPays { .. }, Some(ActionKind::PayCost)) => {
                self.casting = Some(Casting::new(request.player, None, spend_of(step)?));
            }
            (_, Some(ActionKind::ActivateAbility)) => {
                if self.casting.is_some() {
                    let more = spend_of(step)?;
                    self.casting.as_mut().unwrap().spend.extend(more);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Paying from the pool without being told (`AutoPay`): `Spend:` first, then any pool mana that pays what is owed.
    fn auto_pay(&mut self, request: &Request) -> R<Option<Answer>> {
        let RequestKind::PayCosts { remaining, options, announcing, most, .. } = &request.kind else { return Ok(None) };
        if *announcing {
            // X, or how many times it is kicked, announced as the line said, as a page announces it.
            let want = u32::from(self.casting.as_ref().and_then(|c| c.x).unwrap_or(0));
            return Ok((want <= *most).then_some(Answer::Announce(want)));
        }
        // Convoke taps the creatures it names, one per question, and Delve exiles that many cards from the graveyard, the
        // first offered each time: what pays the cost instead of mana (CR 601.2h).
        if let Some(casting) = self.casting.as_mut() {
            if !casting.convoke.is_empty() && options.iter().any(|o| o.kind == ActionKind::TapToPay) {
                let wanted = casting.convoke.pop_front().unwrap();
                let tap = options.iter().find(|o| {
                    o.kind == ActionKind::TapToPay && o.source.is_some_and(|s| self.game.known_name(s).eq_ignore_ascii_case(&wanted))
                });
                return match tap {
                    Some(o) => Ok(Some(Answer::Act(Action::of(o)))),
                    None => fail(format!(
                        "Convoke: names {wanted}, but the creatures offered are {:?}",
                        options.iter().filter(|o| o.kind == ActionKind::TapToPay).filter_map(|o| o.source).map(|s| self.game.known_name(s)).collect::<Vec<_>>()
                    )),
                };
            }
            if casting.delve > 0
                && let Some(o) = options.iter().find(|o| o.kind == ActionKind::ExileToPay)
            {
                casting.delve -= 1;
                return Ok(Some(Answer::Act(Action::of(o))));
            }
        }
        let assign: Vec<&ActionOption> = options.iter().filter(|o| o.kind == ActionKind::AssignMana).collect();
        if let Some(casting) = self.casting.as_mut()
            && !assign.is_empty()
            && let Some(wanted) = casting.spend.pop_front()
        {
            let game = &self.game;
            let from = |o: &ActionOption| o.source.map(|s| game.known_name(s).to_string());
            return match assign.iter().find(|o| (o.mana, o.restriction) == (Some(wanted.0), wanted.1) && from(o).map(|n| n.to_ascii_lowercase()) == wanted.2.as_ref().map(|n| n.to_ascii_lowercase())) {
                Some(o) => Ok(Some(Answer::Act(Action::of(o)))),
                None => fail(format!(
                    "Spend: names {wanted:?}, but the pool mana offered is {:?}",
                    assign.iter().map(|o| (o.mana, from(o))).collect::<Vec<_>>()
                )),
            };
        }
        if !remaining.is_zero()
            && let Some(o) = assign.iter().find(|o| o.mana.is_some_and(|m| pays(remaining, m)))
        {
            return Ok(Some(Answer::Act(Action::of(o))));
        }
        if let Some(o) = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting) {
            return Ok(Some(Answer::Act(Action::of(o))));
        }
        Ok(None)
    }

    fn default(&mut self, request: &Request) -> R<Answer> {
        Ok(match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { source, remaining, .. } => {
                if self.casting.is_none() {
                    self.casting = Some(Casting::new(request.player, None, VecDeque::new()));
                    if let Some(auto) = self.auto_pay(request)? {
                        return Ok(auto);
                    }
                    self.casting = None;
                }
                return fail(format!(
                    "{} is paying for {} with {remaining} still owed, and the next line does not pay it",
                    bot(request.player),
                    self.game.known_name(*source)
                ));
            }
            RequestKind::UnlessPays { options, .. } => {
                if let Some(ScriptStep::Action(step)) = self.current()
                    && step.verb == ScriptVerb::PayCost
                    && player_of(step.bot) == request.player
                {
                    return fail(format!("line {}: cannot pay yet: {}", step.line, step.text));
                }
                let decline = options.iter().find(|o| o.kind == ActionKind::DeclineCost).expect("decline offered");
                Answer::Act(Action::of(decline))
            }
            RequestKind::May { .. } => Answer::No,
            RequestKind::ChoosePlayer { candidates, .. } => Answer::Player(candidates[0]),
            // The first creatures offered with positive power, until their power is enough.
            RequestKind::TapForPower { cards, powers, minimum, .. } => {
                let mut total = 0u32;
                let mut chosen = Vec::new();
                for (&card, &power) in cards.iter().zip(powers) {
                    if total >= *minimum as u32 {
                        break;
                    }
                    if power <= 0 {
                        continue;
                    }
                    total += power as u32;
                    chosen.push(card);
                }
                Answer::Cards(chosen)
            }
            // The first cards offered, until their mana value is enough.
            RequestKind::CollectEvidence { cards, values, minimum, .. } => {
                let mut total = 0u32;
                let mut chosen = Vec::new();
                for (&card, &value) in cards.iter().zip(values) {
                    if total >= *minimum as u32 {
                        break;
                    }
                    total += value as u32;
                    chosen.push(card);
                }
                Answer::Cards(chosen)
            }
            // Craft's materials (CR 702.167a): the fewest it takes, the first offered - the permanents before the graveyard.
            RequestKind::Craft { cards, minimum, .. } => Answer::Cards(cards.iter().take(*minimum as usize).copied().collect()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand[hand.len() - *count as usize..].to_vec()),
            // The first cards offered, as many as the rule lets be taken one after another.
            RequestKind::ChooseCards { source, cards, rule, .. } => {
                Answer::Cards(select::first_answer(&self.game, rule, request.player, *source, cards, true).unwrap_or_default())
            }
            // A creature that attacks each combat if able attacks whether or not the script says so (CR 508.1d): the first
            // player, planeswalker or battle it may attack without paying a cost to attack, which is never required.
            RequestKind::DeclareAttackers { requirements, players, planeswalkers, battles, costs, .. } if !players.is_empty() => {
                let targets: Vec<Target> =
                    players.iter().map(|&p| Target::Player(p)).chain(planeswalkers.iter().chain(battles).map(|&o| Target::Object(o))).collect();
                Answer::Attack(
                    requirements
                        .iter()
                        .filter_map(|&r| targets.iter().find(|&&t| AttackCost::free(costs, r, t)).map(|&t| (r, t)))
                        .collect(),
                )
            }
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::AssignCombatDamage { divisions } => Answer::Shares(DamageDivision::suggested_shares(divisions)),
            // No blocks, or where an effect requires blocks (CR 509.1c) the engine's own declaration, which obeys them.
            RequestKind::DeclareBlockers { requirements, .. } => Answer::Block(requirements.suggested.clone()),
            // The first eligible modes, the affordable ones first where the question advises (`FirstEligible`).
            // The first eligible mode as many times as are wanted, where a mode may be chosen more than once.
            RequestKind::ChooseModes { eligible, count, repeat: true, .. } => {
                Answer::Modes((0..*count as usize).map(|i| eligible[i.min(eligible.len() - 1)]).collect())
            }
            RequestKind::ChooseModes { eligible, count, affordable, .. } => {
                let mut modes = eligible.clone();
                if !affordable.is_empty() {
                    modes.sort_by_key(|m| !affordable.contains(m));
                }
                Answer::Modes(modes.into_iter().take(*count as usize).collect())
            }
            // The fewest it takes; "with total mana value X or less", the first of them that fit together.
            RequestKind::ChooseTarget { candidates, least, total: Some(t), .. } => Answer::Targets(t.first_fitting(candidates, *least as usize)),
            RequestKind::ChooseTarget { candidates, least, .. } => {
                Answer::Targets(candidates.iter().take(*least as usize).copied().collect())
            }
            // The whole total on the first candidate, whatever the slot allows: a legal division, not a good one.
            RequestKind::ChooseDividedTargets { candidates, total, .. } => {
                Answer::Divided(if *total > 0 && !candidates.is_empty() { vec![(candidates[0], *total)] } else { Vec::new() })
            }
            // Counters from among permanents (CR 118.3): as many as there are from each pool in turn, the first offered first.
            RequestKind::RemoveCounters { pools, total, .. } => {
                let mut left = *total as u16;
                let mut taken = Vec::new();
                for p in pools {
                    let n = p.count.min(left);
                    if n > 0 {
                        taken.push((p.object, p.kind, n));
                        left -= n;
                    }
                }
                Answer::Counters(taken)
            }
            RequestKind::ChooseColors { choices, count, .. } => match self.colors.take() {
                Some(named) if named.len() == *count as usize && named.iter().all(|&c| choices.has(c)) => Answer::Colors(named),
                _ => Answer::Colors(choices.iter().take(*count as usize).collect()),
            },
            RequestKind::ChooseStartingPlayer { .. } => Answer::Player(request.player),
            RequestKind::Mulligan { .. } => Answer::No,
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            RequestKind::ChooseReplacement { .. } => Answer::Index(0),
            RequestKind::ChooseName { suggestion, .. } => Answer::Name(*suggestion),
            RequestKind::ChooseSubtype { choices, .. } => Answer::Subtype(choices[0]),
            RequestKind::DistributeCards { cards, destinations, .. } => {
                Answer::Placements { placements: greedy_distribution(cards, destinations), shuffle: false }
            }
            // Nothing found where finding nothing is allowed (CR 701.23b); otherwise the first cards offered, as few as the rule
            // takes.
            RequestKind::SearchLibrary { source, library, rule, .. } => {
                Answer::Cards(select::first_answer(&self.game, rule, request.player, *source, library, false).unwrap_or_default())
            }
        })
    }

    fn matches(&self, step: &ActionStep, request: &Request) -> R<Option<Answer>> {
        use ScriptVerb as V;
        let options = match &request.kind {
            RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } => {
                Some(options)
            }
            _ => None,
        };
        Ok(match (step.verb, &request.kind) {
            // A cast an effect makes as it resolves (CR 608.2g) asks for its targets with the spell already on the
            // stack (CR 601.2a); the `Cast(...)` line that names it gives them.
            (V::Cast, RequestKind::ChooseTarget { source, purpose: Purpose::Targets, candidates, least, count, .. })
                if self.game.state.stack.iter().any(|i| i.kind == StackKind::Spell && i.object == *source) =>
            {
                self.choose_target(step, candidates, *least, *count)?
            }
            (V::Play | V::Cast | V::Activate | V::Foretell | V::Suspend, _) => match options {
                Some(options) => self.match_option(step, options)?,
                None => None,
            },
            (V::Pass, RequestKind::Priority { .. }) => Some(Answer::Pass),
            (V::PayCost, RequestKind::UnlessPays { options, .. }) => {
                options.iter().find(|o| o.kind == ActionKind::PayCost).map(|o| Answer::Act(Action::of(o)))
            }
            // A payment of life alone (a shockland's 2 life, ward's): accepting it is paying it, as a "you may" is answered.
            // One with mana in it is paid with `PayCost`.
            (V::Accept, RequestKind::UnlessPays { options, total, .. }) if *total == ManaCost::ZERO => {
                options.iter().find(|o| o.kind == ActionKind::PayCost).map(|o| Answer::Act(Action::of(o)))
            }
            (V::Decline, RequestKind::UnlessPays { options, .. }) => {
                options.iter().find(|o| o.kind == ActionKind::DeclineCost).map(|o| Answer::Act(Action::of(o)))
            }
            (V::Decline, RequestKind::May { .. }) => Some(Answer::No),
            (V::Accept, RequestKind::May { .. }) => Some(Answer::Yes),
            (V::Cancel, RequestKind::PayCosts { options, .. }) => {
                options.iter().find(|o| o.kind == ActionKind::CancelCasting).map(|o| Answer::Act(Action::of(o)))
            }
            (V::Cancel, _) if request.may_cancel => Some(Answer::Cancel),
            (V::Choose, RequestKind::ChooseCards { .. }) => Some(self.pick_by_rule(step, request)?),
            // Harmonize's creature is up to one (CR 702.180a): unless the script taps one, none is.
            (V::Choose, RequestKind::TapForPower { purpose: Purpose::TapToReduce, cards, .. }) => Some(self.pick(step, cards, 1, false)?),
            (V::Choose, RequestKind::TapForPower { cards, .. }) => Some(self.pick(step, cards, cards.len(), false)?),
            (V::Choose, RequestKind::CollectEvidence { cards, .. }) => Some(self.pick(step, cards, cards.len(), false)?),
            (V::Choose, RequestKind::Craft { cards, maximum, .. }) => Some(self.pick(step, cards, *maximum as usize, false)?),
            (V::ChoosePlayer, RequestKind::ChoosePlayer { candidates, .. }) => {
                let Some(spec) = step.positional().next() else {
                    return fail(format!("line {}: ChoosePlayer needs a player", step.line));
                };
                match bot_id(spec).map(player_of) {
                    Some(p) if candidates.contains(&p) => Some(Answer::Player(p)),
                    _ => return fail(format!("line {}: {spec} is not a player who may be chosen", step.line)),
                }
            }
            (V::Discard, RequestKind::DiscardToHandSize { hand, count }) => Some(self.pick(step, hand, *count as usize, true)?),
            (V::Attack, RequestKind::DeclareAttackers { eligible, players, planeswalkers, battles, .. }) => {
                let both: Vec<ObjectId> = planeswalkers.iter().chain(battles.iter()).copied().collect();
                self.attack(step, eligible, players, &both)?
            },
            (V::Block, RequestKind::DeclareBlockers { eligible, attackers, allowed, .. }) => self.block(step, eligible, attackers, allowed)?,
            (V::AssignDamage, RequestKind::AssignCombatDamage { divisions }) => self.assign_damage(step, divisions)?,
            // "Choose odd or even" is a name in the C# engine's scripts: `Name(Odd)`.
            (V::Modes | V::Name, RequestKind::ChooseModes { modes, eligible, count, at_most, .. }) => {
                Some(self.modes(step, modes, eligible, *count, (*at_most).max(*count))?)
            }
            (V::Target, RequestKind::ChooseTarget { candidates, least, count, .. }) => {
                self.choose_target(step, candidates, *least, *count)?
            }
            (V::Divide, RequestKind::ChooseDividedTargets { candidates, .. }) => Some(self.divide(step, candidates)?),
            (V::RemoveCounters, RequestKind::RemoveCounters { pools, total, up_to, .. }) => Some(self.remove_counters(step, pools, *total, *up_to)?),
            (V::Colors, RequestKind::ChooseColors { choices, count, .. }) => {
                let mut colors = Vec::new();
                for spec in step.positional() {
                    match color_named(spec, false) {
                        Some(c) if choices.has(c) => colors.push(c),
                        _ => return fail(format!("line {}: {spec} is not a color on offer", step.line)),
                    }
                }
                if colors.len() != *count as usize {
                    return fail(format!("line {}: {} colors wanted", step.line, count));
                }
                Some(Answer::Colors(colors))
            }
            (V::PlayFirst, RequestKind::ChooseStartingPlayer { .. }) => Some(Answer::Player(request.player)),
            (V::DrawFirst, RequestKind::ChooseStartingPlayer { candidates }) => {
                let at = candidates.iter().position(|&p| p == request.player).unwrap_or(0);
                Some(Answer::Player(candidates[(at + 1) % candidates.len()]))
            }
            (V::Triggers, RequestKind::OrderTriggers { sources, abilities }) => Some(self.order_triggers(step, sources, abilities)?),
            (V::Replace, RequestKind::ChooseReplacement { sources, hints, .. }) => Some(self.replacement(step, sources, hints)?),
            (V::Arrange, RequestKind::DistributeCards { cards, destinations, may_shuffle, .. }) => {
                Some(self.arrange(step, cards, destinations, *may_shuffle)?)
            }
            (V::Search, RequestKind::SearchLibrary { .. }) => Some(self.pick_by_rule(step, request)?),
            // "Choose a creature type" is a modes question in the C# engine: `Modes(Elf)`.
            (V::Modes, RequestKind::ChooseSubtype { choices, .. }) => {
                let spec = step.positional().next().unwrap_or("").trim();
                match choices.iter().copied().find(|s| s.name().eq_ignore_ascii_case(spec)) {
                    Some(s) => Some(Answer::Subtype(s)),
                    None => return fail(format!("line {}: {spec} is not a type the question offers", step.line)),
                }
            }
            (V::Name, RequestKind::ChooseName { .. }) => {
                let spec = step.positional().next().unwrap_or("").trim();
                match self.game.cards.find(spec) {
                    Some(card) => Some(Answer::Name(Some(card))),
                    None => return fail(format!("line {}: no card is named {spec}", step.line)),
                }
            }
            (V::Mulligan, RequestKind::Mulligan { .. }) => Some(Answer::Yes),
            (V::Keep, RequestKind::Mulligan { .. }) => Some(Answer::No),
            _ => None,
        })
    }

    // --- Names ------------------------------------------------------------------------------------------------

    fn holder(&self, id: ObjectId) -> PlayerId {
        let obj = self.game.state.obj(id);
        if obj.zone.is(ZoneKind::Battlefield) || obj.zone.is(ZoneKind::Stack) || obj.zone.is(ZoneKind::PhasedOut) { obj.controller } else { obj.owner }
    }

    /// A zone's objects as the C# engine lists them: libraries, graveyards and the stack top first.
    fn zone_objects(&self, kind: ZoneKind) -> Vec<ObjectId> {
        let s = &self.game.state;
        let mut ids: Vec<ObjectId> = match kind {
            ZoneKind::Battlefield => s.battlefield().to_vec(),
            ZoneKind::Stack => s.zone(Zone::STACK).iter().rev().copied().collect(),
            ZoneKind::Exile => {
                let mut all: Vec<ObjectId> = s.seats().flat_map(|p| s.zone(Zone::exile(p)).to_vec()).collect();
                all.sort();
                all
            }
            ZoneKind::Graveyard => s.seats().flat_map(|p| s.zone(Zone::graveyard(p)).iter().rev().copied().collect::<Vec<_>>()).collect(),
            ZoneKind::Library => s.seats().flat_map(|p| s.zone(Zone::library(p)).to_vec()).collect(),
            ZoneKind::Hand => s.seats().flat_map(|p| s.zone(Zone::hand(p)).to_vec()).collect(),
            ZoneKind::Command => s.zone(Zone::COMMAND).to_vec(),
            ZoneKind::Sideboard => s.seats().flat_map(|p| s.zone(Zone::sideboard(p)).to_vec()).collect(),
            ZoneKind::PhasedOut => s.zone(Zone::PHASED_OUT).to_vec(),
            ZoneKind::Gone => Vec::new(),
        };
        ids.retain(|&id| live(s, id));
        ids
    }

    fn same_named(&self, kind: ZoneKind, name: &str, holder: PlayerId) -> Vec<ObjectId> {
        self.zone_objects(kind)
            .into_iter()
            .filter(|&id| self.holder(id) == holder && self.game.known_name(id).eq_ignore_ascii_case(name))
            .collect()
    }

    /// A mana ability the player has of their own (CR 605.1a, 611.2a), named by the card that gave it and no zone,
    /// `Bot1.Channel.Activate`: it has no source, so it is offered wherever that card has gone.
    fn is_players_ability(&self, step: &ActionStep, option: &ActionOption) -> bool {
        let (Some(source), Some(card)) = (option.source, &step.card) else { return false };
        option.index >= PLAYER_MANA_ABILITY
            && step.zone.is_none()
            && self.game.cards.get(self.game.state.obj(source).card).name.eq_ignore_ascii_case(card)
    }

    fn is_source(&self, step: &ActionStep, source: Option<ObjectId>) -> bool {
        let Some(id) = source else { return false };
        let s = &self.game.state;
        if !live(s, id) {
            return false;
        }
        let Some(card) = &step.card else { return false };
        if !self.game.known_name(id).eq_ignore_ascii_case(card) {
            return false;
        }
        let zone = s.obj(id).zone.kind();
        if let Some(z) = step.zone.and_then(engine_zone)
            && z != zone
        {
            return false;
        }
        if let Some(n) = step.copy {
            let list = self.same_named(zone, card, self.holder(id));
            return list.get(n - 1) == Some(&id);
        }
        true
    }

    /// The objects of `pool` a spec names (`Objects` in the C# runner).
    fn objects(&self, step: &ActionStep, spec: &str, pool: &[ObjectId]) -> R<Vec<ObjectId>> {
        let mut parts: Vec<&str> = spec.split('.').collect();
        let mut holder = None;
        let mut zone = None;
        if parts.len() > 1
            && let Some(b) = bot_id(parts[0])
        {
            holder = Some(player_of(b));
            parts.remove(0);
        }
        if parts.len() > 1
            && let Some(z) = parse::ZoneKind::parse(parts[0])
        {
            zone = engine_zone(z);
            parts.remove(0);
        }
        let mut name = parts.join(".").trim().to_string();
        let mut copy = None;
        if let Some((base, n)) = split_copy(&name) {
            name = base;
            copy = Some(n);
        }
        let known = self.game.cards.find(&name).is_some() || self.game.state.objects.iter().enumerate().any(|(i, _)| self.game.known_name(ObjectId(i as u32)).eq_ignore_ascii_case(&name));
        if !known {
            return fail(format!("line {}: no card is named \"{name}\"", step.line));
        }
        let s = &self.game.state;
        Ok(pool
            .iter()
            .copied()
            .filter(|&id| {
                if !live(s, id) || !self.game.known_name(id).eq_ignore_ascii_case(&name) {
                    return false;
                }
                let kind = s.obj(id).zone.kind();
                if zone.is_some_and(|z| z != kind) || holder.is_some_and(|h| h != self.holder(id)) {
                    return false;
                }
                match copy {
                    Some(n) => self.same_named(kind, &name, self.holder(id)).get(n - 1) == Some(&id),
                    None => true,
                }
            })
            .collect())
    }

    fn target(&self, step: &ActionStep, spec: &str, candidates: &[Target]) -> R<Option<Target>> {
        if let Some(b) = bot_id(spec) {
            let p = player_of(b);
            return Ok(candidates.iter().copied().find(|&t| t == Target::Player(p)));
        }
        // An ability on the stack is named for its source, as `Stack.Name`: the topmost of that name, or with `#n` the nth
        // from the top (`Stack.Name#2`, the one under it).
        if let Some((zone, name)) = spec.split_once('.')
            && zone.eq_ignore_ascii_case("Stack")
        {
            let (name, nth) = match name.rsplit_once('#') {
                Some((n, k)) if k.trim().parse::<usize>().is_ok_and(|k| k > 0) => (n, k.trim().parse::<usize>().unwrap() - 1),
                _ => (name, 0),
            };
            let stack = &self.game.state.stack;
            let found = candidates.iter().copied().filter(|t| matches!(t, Target::Ability(_))).rev().filter(|t| {
                matches!(t, Target::Ability(n) if stack.iter().any(|i| i.serial == *n && self.game.known_name(i.object).eq_ignore_ascii_case(name.trim())))
            }).nth(nth);
            if found.is_some() {
                return Ok(found);
            }
        }
        let pool: Vec<ObjectId> = candidates.iter().filter_map(|t| t.object()).collect();
        let found = self.objects(step, spec, &pool)?;
        let Some(&first) = found.first() else { return Ok(None) };
        let place = |id: ObjectId| (self.game.state.obj(id).zone.kind(), self.holder(id));
        if found.iter().any(|&id| place(id) != place(first)) {
            return fail(format!("line {}: {spec} could be more than one card; write the one meant", step.line));
        }
        Ok(Some(Target::Object(first)))
    }

    /// The cards a `Choose(..)` or `Search(..)` line names for a choice of cards or a library search, judged by the question's
    /// own rule as a player picks (`selection_verdict`): each name is the first card of that name that may be added to the
    /// ones named before it, and together they must be an answer.
    fn pick_by_rule(&self, step: &ActionStep, request: &Request) -> R<Answer> {
        let mut chosen: Vec<ObjectId> = Vec::new();
        for spec in step.positional() {
            let Some((cards, verdict)) = selection_verdict(&self.game, request, &chosen) else {
                return fail(format!("line {}: not a choice of cards", step.line));
            };
            let addable: Vec<ObjectId> = verdict.addable.ones().map(|i| cards[i]).collect();
            match self.objects(step, spec, &addable)?.first() {
                Some(&id) => chosen.push(id),
                None => return fail(format!("line {}: {spec} is not among the cards that may be chosen", step.line)),
            }
        }
        let complete = selection_verdict(&self.game, request, &chosen).is_some_and(|(_, verdict)| verdict.complete);
        if !complete {
            let wanted = match &request.kind {
                RequestKind::ChooseCards { rule, .. } | RequestKind::SearchLibrary { rule, .. } => count_words(rule.count),
                _ => String::new(),
            };
            return fail(format!("line {}: {wanted} cards wanted, the line names {}, which the question does not take", step.line, chosen.len()));
        }
        Ok(Answer::Cards(chosen))
    }

    fn pick(&self, step: &ActionStep, pool: &[ObjectId], count: usize, exactly: bool) -> R<Answer> {
        let mut chosen: Vec<ObjectId> = Vec::new();
        for spec in step.positional() {
            let found = self.objects(step, spec, pool)?;
            match found.into_iter().find(|id| !chosen.contains(id)) {
                Some(id) => chosen.push(id),
                None => return fail(format!("line {}: {spec} is not among the cards that may be chosen", step.line)),
            }
        }
        if (exactly && chosen.len() != count as usize) || chosen.len() > count as usize {
            return fail(format!("line {}: {} cards wanted, the line names {}", step.line, count, chosen.len()));
        }
        Ok(Answer::Cards(chosen))
    }

    /// The refused block `Blocker > Attacker` (see the loop above) when no blockers question was asked in the step.
    fn check_block_never_asked(&self, action: &ActionStep, line: usize, text: &str) -> R<()> {
        let defender = player_of(action.bot);
        let battlefield: Vec<ObjectId> = self.game.state.battlefield().to_vec();
        for spec in action.positional() {
            let parts: Vec<&str> = spec.split('>').collect();
            if parts.len() != 2 {
                return fail(format!("line {line}: write a refused block as \"Blocker > Attacker\""));
            }
            let blockers: Vec<ObjectId> = self
                .objects(action, parts[0].trim(), &battlefield)?
                .into_iter()
                .filter(|&b| self.game.state.obj(b).controller == defender && !self.game.state.obj(b).tapped && self.game.chars(b).is(CardType::Creature))
                .collect();
            let attacking: Vec<ObjectId> = self
                .game
                .state
                .attackers
                .iter()
                .filter(|a| matches!(a.target, Target::Player(p) if p == defender))
                .map(|a| a.id)
                .collect();
            let attackers = self.objects(action, parts[1].trim(), &attacking)?;
            if blockers.is_empty() || attackers.is_empty() {
                return fail(format!("line {line}: {text} proves nothing: the blocker must be an untapped creature of the defender and the attacker must be attacking that player"));
            }
            if self.game.state.blocks.iter().any(|b| blockers.contains(&b.blocker) && attackers.contains(&b.attacker)) {
                return fail(format!("line {line} says {text} must not be allowed, but the block was made"));
            }
        }
        Ok(())
    }

    /// `Blocker > Attacker` per block; only a pair the request allows.
    /// A refused block or attack (`! Bot2.Block(Bear > Ox)`, `! Bot1.Attack(Bear > Bot2.Battlefield.Karn)`): checked at the
    /// player's next declaration, failing if that pair is allowed. Both sides must be in the question, or the refusal
    /// proves nothing.
    fn check_refused_declaration(&self, action: &ActionStep, line: usize, text: &str, request: &Request) -> R<()> {
        match &request.kind {
            RequestKind::DeclareBlockers { eligible, attackers, allowed, .. } => {
                // `! Bot2.Block()`: declaring no blockers at all is refused, because some creature must block (CR 509.1c).
                if action.positional().next().is_none() {
                    if check_answer(&self.game, request, &Answer::Block(Vec::new())).is_ok() {
                        return fail(format!("line {line} says {text} must not be allowed, but declaring no blockers is"));
                    }
                    return Ok(());
                }
                // The line names a whole declaration: an attacker that needs several blockers (menace, CR 702.110b) is not
                // legally blocked by fewer of them than the line gives it, however each pair stands alone.
                let mut given: Vec<(ObjectId, usize)> = Vec::new();
                for spec in action.positional() {
                    if let Some((_, atk)) = spec.split_once('>') {
                        for a in self.objects(action, atk.trim(), attackers)? {
                            match given.iter_mut().find(|g| g.0 == a) {
                                Some(g) => g.1 += 1,
                                None => given.push((a, 1)),
                            }
                        }
                    }
                }
                // The declaration the line names, the first candidate for each side of each pair.
                let mut pairs: Vec<(ObjectId, ObjectId)> = Vec::new();
                for spec in action.positional() {
                    if let Some((blk, atk)) = spec.split_once('>')
                        && let (Some(&b), Some(&a)) =
                            (self.objects(action, blk.trim(), eligible)?.first(), self.objects(action, atk.trim(), attackers)?.first())
                    {
                        pairs.push((b, a));
                    }
                }
                // A declaration of blocks each allowed alone that the engine's own check refuses as a whole holds: one that
                // obeys fewer of the blocks effects require than a declaration could (CR 509.1c: Lure).
                if pairs.len() == action.positional().count()
                    && pairs.iter().all(|p| allowed.contains(p))
                    && matches!(&request.kind, RequestKind::DeclareBlockers { requirements, .. } if requirements.most > 0)
                    && check_answer(&self.game, request, &Answer::Block(pairs.clone())).is_err()
                {
                    return Ok(());
                }
                for spec in action.positional() {
                    let parts: Vec<&str> = spec.split('>').collect();
                    if parts.len() != 2 {
                        return fail(format!("line {line}: write a refused block as \"Blocker > Attacker\""));
                    }
                    let blockers = self.objects(action, parts[0].trim(), eligible)?;
                    let targets = self.objects(action, parts[1].trim(), attackers)?;
                    if targets.is_empty() {
                        return fail(format!("line {line}: {text} proves nothing: the attacker must be in the question"));
                    }
                    // A blocker left out of the question is a refusal only if it is an untapped creature of the player
                    // asked, so that it is an effect ("can't block") that keeps it out and not a tapped or absent creature.
                    if blockers.is_empty() {
                        let mine: Vec<ObjectId> = self
                            .game
                            .state
                            .battlefield()
                            .iter()
                            .copied()
                            .filter(|&id| !self.game.state.obj(id).tapped && self.game.state.obj(id).controller == player_of(action.bot))
                            .collect();
                        if self.objects(action, parts[0].trim(), &mine)?.is_empty() {
                            return fail(format!("line {line}: {text} proves nothing: the blocker must be an untapped creature of the defender"));
                        }
                        continue;
                    }
                    // Fewer blockers than an attacker requires (menace): the declaration is refused (CR 509.1b). A line of
                    // several pairs is one declaration: more blockers on an attacker than "can't be blocked by more than one
                    // creature" allows (CR 509.1b), which the engine's own check of the answer must refuse.
                    let short = targets.iter().any(|&a| {
                        given.iter().find(|g| g.0 == a).is_some_and(|g| (g.1 as usize) < self.game.chars(a).minimum_blockers())
                    }) || (pairs.len() > 1
                        && pairs.iter().all(|p| allowed.contains(p))
                        && pairs.iter().any(|&(_, a)| pairs.iter().filter(|p| p.1 == a).count() > self.game.chars(a).maximum_blockers())
                        && check_answer(&self.game, request, &Answer::Block(pairs.clone())).is_err());
                    if !short && blockers.iter().any(|&b| targets.iter().any(|&a| allowed.contains(&(b, a)))) {
                        return fail(format!("line {line} says {text} must not be allowed, but it is"));
                    }
                }
            }
            RequestKind::DeclareAttackers { eligible, players, planeswalkers, battles, .. } => {
                // `! Bot1.Attack()`: declaring no attackers at all is refused, because some creature must attack (CR 508.1d).
                if action.positional().next().is_none() {
                    if check_answer(&self.game, request, &Answer::Attack(Vec::new())).is_ok() {
                        return fail(format!("line {line} says {text} must not be allowed, but declaring no attackers is"));
                    }
                    return Ok(());
                }
                for spec in action.positional() {
                    let parts: Vec<&str> = spec.split('>').collect();
                    if parts.len() != 2 {
                        return fail(format!("line {line}: write a refused attack as \"Attacker > Target\""));
                    }
                    if self.objects(action, parts[0].trim(), eligible)?.is_empty() {
                        // An attacker the question leaves out is a refusal worth proving when only a restriction (Defender,
                        // "can't attack", summoning sickness) can be the reason: it is the asked player's own and untapped,
                        // and another creature of theirs is offered, so the declaration is being asked at all.
                        let on_battlefield: Vec<ObjectId> = self.game.state.battlefield().to_vec();
                        let named = self.objects(action, parts[0].trim(), &on_battlefield)?;
                        let player = player_of(action.bot);
                        let restricted = !eligible.is_empty()
                            && named.iter().any(|&id| {
                                let obj = self.game.state.obj(id);
                                self.game.controller(id) == player && !obj.tapped
                            });
                        if !restricted {
                            return fail(format!("line {line}: {text} proves nothing: the attacker must be able to attack"));
                        }
                        continue;
                    }
                    let on_battlefield: Vec<ObjectId> = self.game.state.battlefield().to_vec();
                    let offered = match bot_id(parts[1]) {
                        Some(b) => players.contains(&player_of(b)),
                        None => {
                            let found = self.objects(action, parts[1].trim(), &on_battlefield)?;
                            if found.is_empty() {
                                return fail(format!("line {line}: {text} proves nothing: nothing of that name to attack"));
                            }
                            found.iter().any(|f| planeswalkers.contains(f) || battles.contains(f))
                        }
                    };
                    if offered {
                        // Offered as a pair, the attack may still be one no declaration can hold: a creature that must attack a
                        // certain player sent elsewhere while attacking that player is free (CR 508.1d). The declaration of this
                        // attack, with every other creature that must attack at the first thing it may attack for nothing, is
                        // asked of the rules; refused, the refusal holds.
                        let attacker = self.objects(action, parts[0].trim(), eligible)?[0];
                        let target = match bot_id(parts[1]) {
                            Some(b) => Target::Player(player_of(b)),
                            None => Target::Object(
                                self.objects(action, parts[1].trim(), &on_battlefield)?
                                    .into_iter()
                                    .find(|f| planeswalkers.contains(f) || battles.contains(f))
                                    .expect("offered"),
                            ),
                        };
                        let mut attacks = vec![(attacker, target)];
                        if let RequestKind::DeclareAttackers { requirements, costs, .. } = &request.kind {
                            let targets: Vec<Target> = players
                                .iter()
                                .map(|&p| Target::Player(p))
                                .chain(planeswalkers.iter().chain(battles).map(|&o| Target::Object(o)))
                                .collect();
                            for &r in requirements.iter().filter(|&&r| r != attacker) {
                                if let Some(&t) = targets.iter().find(|&&t| AttackCost::free(costs, r, t)) {
                                    attacks.push((r, t));
                                }
                            }
                        }
                        if check_answer(&self.game, request, &Answer::Attack(attacks)).is_ok() {
                            return fail(format!("line {line} says {text} must not be allowed, but it is"));
                        }
                    }
                }
            }
            RequestKind::ChooseModes { modes, source, .. } => {
                let mut chosen = Vec::new();
                // A mode named by its text that the question does not have at all is not offered, which is what the line
                // says (a fused cast over a mana-value limit, `! Bot1.Modes(Fuse)`) - but only a mode the source could
                // offer somewhere (`modes_the_source_could_offer`): a name it never could is a typo, and fails.
                let mut absent = false;
                let could = modes_the_source_could_offer(&self.game, *source);
                for spec in action.positional() {
                    let index = match spec.trim().parse::<usize>() {
                        Ok(n) if n >= 1 => n - 1,
                        _ => match modes.iter().position(|m| label_text(m).to_lowercase().contains(&spec.trim().to_lowercase())) {
                            Some(i) => i,
                            None if could.iter().any(|m| m.to_lowercase().contains(&spec.trim().to_lowercase())) => {
                                absent = true;
                                continue;
                            }
                            None => return fail(format!("line {line}: no mode reads \"{spec}\", and {} could offer none that does", self.game.known_name(*source))),
                        },
                    };
                    if index >= modes.len() {
                        return fail(format!("line {line}: there is no mode {spec}"));
                    }
                    chosen.push(index as u8);
                }
                // The rules' own check of the answer (a mode twice is refused unless the question lets a mode repeat).
                if !absent && check_answer(&self.game, request, &Answer::Modes(chosen)).is_ok() {
                    return fail(format!("line {line} says {text} must not be a choice the question allows, but it is"));
                }
            }
            RequestKind::ChooseTarget { candidates, least, count, .. } => {
                // `! Bot1.Target(Count: 2)`: the question does not take that many targets (CR 601.2c), though it has the
                // candidates for it; `Count: 0` is choosing none.
                if let Some(wanted) = action.named("Count") {
                    let Ok(wanted) = wanted.trim().parse::<usize>() else { return fail(format!("line {line}: {wanted} is not a number")) };
                    if wanted > candidates.len() {
                        return fail(format!("line {line}: {text} proves nothing: there are only {} candidates", candidates.len()));
                    }
                    if wanted >= *least as usize && wanted <= *count as usize {
                        return fail(format!("line {line} says {text} must not be a number of targets the question takes, but it is"));
                    }
                }
                let everything: Vec<ObjectId> = (0..self.game.state.objects.len() as u32).map(ObjectId).collect();
                // Several names that are all candidates are one answer, which the question must refuse: more targets than
                // the slot takes (`! Bot1.Target(A; B; C)` for "up to two"), or the same object named twice,
                // `! Bot1.Target(Grizzly Bears; Grizzly Bears)` (CR 115.3: one object is not chosen twice for one word
                // "target"), which must then be as many as the slot takes, so that the repetition is what is refused.
                let named: Vec<Option<Target>> = action.positional().map(|spec| self.target(action, spec.trim(), candidates)).collect::<R<_>>()?;
                if named.len() > 1 && named.iter().all(Option::is_some) {
                    let repeated = named.iter().enumerate().any(|(i, t)| named[..i].contains(t));
                    if repeated
                        && let RequestKind::ChooseTarget { least, count, .. } = &request.kind
                        && (named.len() > *count as usize || named.len() < *least as usize)
                    {
                        return fail(format!("line {line}: {text} proves nothing: the question takes {least} to {count} targets"));
                    }
                    let answer = Answer::Targets(named.into_iter().flatten().collect());
                    if check_answer(&self.game, request, &answer).is_ok() {
                        return fail(format!("line {line} says {text} must not be an answer the question accepts, but it is"));
                    }
                    return Ok(());
                }
                for spec in action.positional() {
                    if bot_id(spec).is_none() && self.objects(action, spec.trim(), &everything)?.is_empty() {
                        return fail(format!("line {line}: {text} proves nothing: no such card is in the game"));
                    }
                    if self.target(action, spec.trim(), candidates)?.is_some() {
                        return fail(format!("line {line} says {text} must not be a legal target, but it is"));
                    }
                }
            }
            // `! Bot1.Choose(Ornithopter)` for craft: no such material may be chosen. Naming only materials that may be, it is
            // one answer the question must refuse instead: too few for "two creatures" (CR 702.167a).
            RequestKind::Craft { cards, .. }
                if !action.positional().next().is_none()
                    && action.positional().all(|spec| self.objects(action, spec.trim(), cards).is_ok_and(|found| !found.is_empty())) =>
            {
                let mut chosen: Vec<ObjectId> = Vec::new();
                for spec in action.positional() {
                    if let Some(id) = self.objects(action, spec.trim(), cards)?.into_iter().find(|id| !chosen.contains(id)) {
                        chosen.push(id);
                    }
                }
                if check_answer(&self.game, request, &Answer::Cards(chosen)).is_ok() {
                    return fail(format!("line {line} says {text} must not be an answer the question accepts, but it is"));
                }
            }
            RequestKind::ChooseCards { .. } | RequestKind::TapForPower { .. } | RequestKind::Craft { .. } => {
                // What may be chosen at all: the cards its rule lets be picked first.
                let eligible = &first_choices(&self.game, request);
                let everything: Vec<ObjectId> = (0..self.game.state.objects.len() as u32).map(ObjectId).collect();
                // Several names that are all eligible are one answer, which the question must refuse: more cards than it takes
                // (`! Bot1.Choose(A; B; C)` for "up to two"), as for targets; or creatures whose total power falls short
                // (`! Bot1.Choose(A; B)` for crew, a negative power taking from it).
                if matches!(request.kind, RequestKind::ChooseCards { .. } | RequestKind::TapForPower { .. }) && action.positional().count() > 1 {
                    let mut named: Vec<ObjectId> = Vec::new();
                    for spec in action.positional() {
                        match self.objects(action, spec.trim(), eligible)?.into_iter().find(|id| !named.contains(id)) {
                            Some(id) => named.push(id),
                            None => break,
                        }
                    }
                    if named.len() == action.positional().count() {
                        if check_answer(&self.game, request, &Answer::Cards(named)).is_ok() {
                            return fail(format!("line {line} says {text} must not be an answer the question accepts, but it is"));
                        }
                        return Ok(());
                    }
                }
                for spec in action.positional() {
                    if self.objects(action, spec.trim(), &everything)?.is_empty() {
                        return fail(format!("line {line}: {text} proves nothing: no such card is in the game"));
                    }
                    if !self.objects(action, spec.trim(), eligible)?.is_empty() {
                        return fail(format!("line {line} says {text} must not be choosable, but it is"));
                    }
                }
            }
            // `! Bot1.RemoveCounters(Dyadrine, Synthesis Amalgam: 2)`: a removal the question must refuse - more counters from a
            // permanent than the question lets come off it (Dyadrine's one counter from each of two creatures), judged by the
            // rules' own check of the answer.
            RequestKind::RemoveCounters { pools, total, up_to, .. } => {
                // A permanent the question does not offer at all (a noncreature, an opponent's) is itself the refusal, as long as it
                // exists: it is no pool of the question, so nothing could be taken from it.
                let offered: Vec<Target> = pools.iter().map(|p| Target::Object(p.object)).collect();
                let everything: Vec<ObjectId> = (0..self.game.state.objects.len() as u32).map(ObjectId).collect();
                for piece in action.positional() {
                    let who = piece.rsplit_once(':').map_or(piece, |(who, _)| who).trim();
                    if self.target(action, who, &offered)?.is_none() {
                        if self.objects(action, who, &everything)?.is_empty() {
                            return fail(format!("line {line}: {text} proves nothing: no such card is in the game"));
                        }
                        return Ok(());
                    }
                }
                let answer = self.remove_counters(action, pools, *total, *up_to)?;
                if check_answer(&self.game, request, &answer).is_ok() {
                    return fail(format!("line {line} says {text} must not be a removal the question allows, but it is"));
                }
            }
            // `! Bot1.Name(Lightning Bolt)`: the name may not be chosen (CR 201.4a), checked as the engine checks an answer.
            RequestKind::ChooseName { kind, .. } => {
                for spec in action.positional() {
                    let Some(card) = self.game.cards.find(spec.trim()) else { return fail(format!("line {line}: no card is named {spec}")) };
                    if name_problem(self.game.cards, *kind, card).is_none() {
                        return fail(format!("line {line} says {text} must not be a name the question allows, but it is"));
                    }
                }
            }
            RequestKind::ChooseColors { choices, .. } => {
                for spec in action.positional() {
                    match color_named(spec.trim(), false) {
                        Some(c) if choices.has(c) => {
                            return fail(format!("line {line} says {text} must not be a color on offer, but it is"));
                        }
                        Some(_) => {}
                        None => return fail(format!("line {line}: {spec} is not a color")),
                    }
                }
            }
            RequestKind::SearchLibrary { library, .. } => {
                let eligible = first_choices(&self.game, request);
                // Several names that are all findable are one answer, which the search must refuse: more cards than it takes, or
                // cards it may not find together (`! Bot1.Search(Forest; Forest)` for "with different names").
                if action.positional().count() > 1 {
                    let mut named: Vec<ObjectId> = Vec::new();
                    for spec in action.positional() {
                        match self.objects(action, spec.trim(), &eligible)?.into_iter().find(|id| !named.contains(id)) {
                            Some(id) => named.push(id),
                            None => break,
                        }
                    }
                    if named.len() == action.positional().count() {
                        if check_answer(&self.game, request, &Answer::Cards(named)).is_ok() {
                            return fail(format!("line {line} says {text} must not be an answer the search accepts, but it is"));
                        }
                        return Ok(());
                    }
                }
                for spec in action.positional() {
                    let in_library = self.objects(action, spec.trim(), library)?;
                    if in_library.is_empty() {
                        return fail(format!("line {line}: {text} proves nothing: no such card is in the library searched"));
                    }
                    if in_library.iter().any(|c| eligible.contains(c)) {
                        return fail(format!("line {line} says {text} must not be findable, but it is"));
                    }
                }
            }
            // `! Bot1.Arrange(A; B | )`: the distribution is one the question refuses (too many to the hand, a card
            // unplaced), judged by the rules' own check of the answer.
            RequestKind::DistributeCards { cards, destinations, .. } if !action.raw_arguments.trim().is_empty() => {
                let placements = self.arrangement(action, cards, destinations)?;
                if check_answer(&self.game, request, &Answer::Placements { placements, shuffle: false }).is_ok() {
                    return fail(format!("line {line} says {text} must not be an answer the question accepts, but it is"));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn block(
        &self,
        step: &ActionStep,
        eligible: &[ObjectId],
        attackers: &[ObjectId],
        allowed: &[(ObjectId, ObjectId)],
    ) -> R<Option<Answer>> {
        let mut blocks: Vec<(ObjectId, ObjectId)> = Vec::new();
        for spec in step.positional() {
            let parts: Vec<&str> = spec.split('>').collect();
            if parts.len() != 2 {
                return fail(format!("line {}: write a block as \"Blocker > Attacker\", not \"{spec}\"", step.line));
            }
            let blockers: Vec<ObjectId> =
                self.objects(step, parts[0].trim(), eligible)?.into_iter().filter(|b| !blocks.iter().any(|x| x.0 == *b)).collect();
            let Some(attacker) = self.objects(step, parts[1].trim(), attackers)?.into_iter().next() else { return Ok(None) };
            if blockers.is_empty() {
                return Ok(None);
            }
            let Some(blocker) = blockers.into_iter().find(|&b| allowed.contains(&(b, attacker))) else {
                return fail(format!("line {}: no {} may block {}", step.line, parts[0].trim(), parts[1].trim()));
            };
            blocks.push((blocker, attacker));
        }
        Ok(Some(Answer::Block(blocks)))
    }

    /// `Source > Creature: n` per share, or `Source > BotN: n` for the player a trampler attacks; a creature the line
    /// leaves out keeps the engine's own division.
    fn assign_damage(&self, step: &ActionStep, divisions: &[DamageDivision]) -> R<Option<Answer>> {
        let mut shares: Vec<(ObjectId, Option<ObjectId>, u16)> = Vec::new();
        let mut written: Vec<ObjectId> = Vec::new();
        for spec in step.positional() {
            let parts: Vec<&str> = spec.split('>').collect();
            let parsed = (parts.len() == 2)
                .then(|| parts[1].rfind(':').map(|c| (parts[1][..c].trim(), parts[1][c + 1..].trim().parse::<u16>())))
                .flatten();
            let Some((to, Ok(amount))) = parsed else {
                return fail(format!(
                    "line {}: write a share as \"Source > Creature: amount\" or \"Source > Bot2: amount\", not \"{spec}\"",
                    step.line
                ));
            };
            let source_name = parts[0].trim();
            let mut division = None;
            for d in divisions {
                if !self.objects(step, source_name, &[d.source])?.is_empty() {
                    division = Some(d);
                    break;
                }
            }
            let Some(d) = division else { return Ok(None) };
            let creature = match bot_id(to) {
                Some(b) => {
                    if !d.may_assign_to_defender || d.defender != Target::Player(player_of(b)) {
                        return fail(format!("line {}: {source_name} cannot assign damage to {to}", step.line));
                    }
                    None
                }
                None => match self.objects(step, to, &d.creatures)?.into_iter().next() {
                    Some(c) => Some(c),
                    None => return Ok(None),
                },
            };
            written.push(d.source);
            shares.push((d.source, creature, amount));
        }
        for share in DamageDivision::suggested_shares(divisions) {
            if !written.contains(&share.0) {
                shares.push(share);
            }
        }
        Ok(Some(Answer::Shares(shares)))
    }

    fn attack(&self, step: &ActionStep, eligible: &[ObjectId], players: &[PlayerId], battles: &[ObjectId]) -> R<Option<Answer>> {
        let mut attacks = Vec::new();
        for spec in step.positional() {
            let parts: Vec<&str> = spec.split('>').collect();
            let found = self.objects(step, parts[0].trim(), eligible)?;
            let Some(attacker) = found.into_iter().find(|id| !attacks.iter().any(|(a, _)| a == id)) else {
                return Ok(None);
            };
            let defender = if parts.len() > 1 {
                match bot_id(parts[1]) {
                    Some(b) if players.contains(&player_of(b)) => Target::Player(player_of(b)),
                    // A battle a defending player protects: named as any permanent is.
                    _ => match self.objects(step, parts[1].trim(), battles)?.first() {
                        Some(&battle) => Target::Object(battle),
                        None => return fail(format!("line {}: {} cannot be attacked", step.line, parts[1].trim())),
                    },
                }
            } else if players.len() == 1 {
                Target::Player(players[0])
            } else {
                return fail(format!("line {}: say who {} attacks", step.line, parts[0].trim()));
            };
            attacks.push((attacker, defender));
        }
        Ok(Some(Answer::Attack(attacks)))
    }

    fn modes(&self, step: &ActionStep, modes: &[Label], eligible: &[u8], count: u8, most: u8) -> R<Answer> {
        let mut chosen = Vec::new();
        for spec in step.positional() {
            let index = match spec.trim().parse::<usize>() {
                Ok(n) if n >= 1 => n - 1,
                _ => match modes.iter().position(|m| label_text(m).to_lowercase().contains(&spec.trim().to_lowercase())) {
                    Some(i) => i,
                    None => return fail(format!("line {}: no mode reads \"{spec}\"", step.line)),
                },
            };
            if index >= modes.len() {
                return fail(format!("line {}: there is no mode {spec}", step.line));
            }
            chosen.push(index as u8);
        }
        if chosen.len() < count as usize || chosen.len() > most as usize || !chosen.iter().all(|m| eligible.contains(m)) {
            return fail(format!("line {}: not a choice the question allows", step.line));
        }
        Ok(Answer::Modes(chosen))
    }

    /// `RemoveCounters(Grizzly Bears: 2; Hill Giant: PlusOnePlusOne 1)`: each permanent named, with how many counters come off
    /// it, and the kind (as the dump spells it) where it has more than one. For "up to" (`up_to`), `RemoveCounters()` removes
    /// none, and a number of them past the most is left for the rules to refuse (`! Bot1.RemoveCounters(..)`).
    fn remove_counters(&self, step: &ActionStep, pools: &[CounterPool], total: u8, up_to: bool) -> R<Answer> {
        let mut objects: Vec<Target> = Vec::new();
        for p in pools {
            if !objects.contains(&Target::Object(p.object)) {
                objects.push(Target::Object(p.object));
            }
        }
        let mut taken = Vec::new();
        for piece in step.positional() {
            let Some((who, amount)) = piece.rsplit_once(':') else {
                return fail(format!("line {}: RemoveCounters wants \"permanent: [kind] amount\", not \"{piece}\"", step.line));
            };
            let (kind, amount) = match amount.trim().rsplit_once(' ') {
                Some((kind, amount)) => (Some(kind.trim()), amount),
                None => (None, amount.trim()),
            };
            let Ok(amount) = amount.parse::<u16>() else {
                return fail(format!("line {}: {amount} is not a number of counters", step.line));
            };
            let Some(Target::Object(object)) = self.target(step, who.trim(), &objects)? else {
                return fail(format!("line {}: {} has no counters the cost may take", step.line, who.trim()));
            };
            let on: Vec<&CounterPool> = pools.iter().filter(|p| p.object == object).collect();
            let pool = match kind {
                Some(kind) => on.iter().find(|p| p.kind.token().eq_ignore_ascii_case(kind) || p.kind.name().eq_ignore_ascii_case(kind)),
                None if on.len() == 1 => on.first(),
                None => return fail(format!("line {}: {} has more than one kind of counter; name the kind", step.line, who.trim())),
            };
            let Some(pool) = pool else {
                return fail(format!("line {}: {} has no such counters", step.line, who.trim()));
            };
            taken.push((object, pool.kind, amount));
        }
        // In the order the question offers the pools, as a page answering it sends them, so the two are the same answer.
        taken.sort_by_key(|&(o, k, _)| pools.iter().position(|p| p.object == o && p.kind == k));
        let sum: u32 = taken.iter().map(|t| t.2 as u32).sum();
        if !up_to && sum != total as u32 {
            return fail(format!("line {}: {sum} counters named, {total} to remove", step.line));
        }
        Ok(Answer::Counters(taken))
    }

    /// `Divide(Guerrilla Gorilla: 1; Llanowar Elves: 1)`: each target named, with its share of the total.
    fn divide(&self, step: &ActionStep, candidates: &[Target]) -> R<Answer> {
        let mut shares = Vec::new();
        for piece in step.positional() {
            let Some((who, amount)) = piece.rsplit_once(':') else {
                return fail(format!("line {}: Divide wants \"target: amount\", not \"{piece}\"", step.line));
            };
            let Ok(amount) = amount.trim().parse::<u32>() else {
                return fail(format!("line {}: {} is not a share", step.line, amount.trim()));
            };
            match self.target(step, who.trim(), candidates)? {
                Some(t) => shares.push((t, amount)),
                None => return fail(format!("line {}: {} is not a target on offer", step.line, who.trim())),
            }
        }
        Ok(Answer::Divided(shares))
    }

    fn choose_target(&self, step: &ActionStep, candidates: &[Target], least: u32, count: u32) -> R<Option<Answer>> {
        let specs: Vec<&str> = step.positional().collect();
        if specs.is_empty() {
            if least > 0 {
                return fail(format!("line {}: this needs a target", step.line));
            }
            return Ok(Some(Answer::Targets(Vec::new())));
        }
        if specs.len() > count as usize || specs.len() < least as usize {
            return fail(format!("line {}: takes {least} to {count} target(s) but the line gives {}", step.line, specs.len()));
        }
        let mut targets = Vec::new();
        for spec in specs {
            match self.target(step, spec, candidates)? {
                Some(t) => targets.push(t),
                None => return Ok(None),
            }
        }
        Ok(Some(Answer::Targets(targets)))
    }

    /// `Arrange(A; B | C)`: one `|`-separated group per destination, in the order the question lists them, each
    /// card named where it goes; every card looked at must be placed. `Arrange(Shuffle)` shuffles where allowed.
    fn arrange(&self, step: &ActionStep, cards: &[ObjectId], destinations: &[Destination], may_shuffle: bool) -> R<Answer> {
        let raw = step.raw_arguments.trim();
        if raw.eq_ignore_ascii_case("shuffle") {
            if !may_shuffle {
                return fail(format!("line {}: shuffling is not allowed here", step.line));
            }
            return Ok(Answer::Placements { placements: vec![Vec::new(); destinations.len()], shuffle: true });
        }
        let placements = self.arrangement(step, cards, destinations)?;
        if let Some(problem) = distribution_problem(cards, destinations, &placements) {
            return fail(format!("line {}: {problem}", step.line));
        }
        Ok(Answer::Placements { placements, shuffle: false })
    }

    /// The placements `Arrange(A; B | C)` names, as named: the cards must be among those looked at, but whether the
    /// question accepts the distribution is not judged (a refusal, `! Bot1.Arrange(..)`, asks the rules that).
    fn arrangement(&self, step: &ActionStep, cards: &[ObjectId], destinations: &[Destination]) -> R<Vec<Vec<ObjectId>>> {
        let groups: Vec<&str> = step.raw_arguments.split('|').collect();
        if groups.len() > destinations.len() {
            return fail(format!("line {}: {} groups for {} destinations", step.line, groups.len(), destinations.len()));
        }
        let mut placements: Vec<Vec<ObjectId>> = vec![Vec::new(); destinations.len()];
        for (i, group) in groups.iter().enumerate() {
            for piece in parse::ScriptArgument::split(group) {
                let found = self.objects(step, &piece.value, cards)?;
                match found.into_iter().find(|id| !placements.iter().any(|p| p.contains(id))) {
                    Some(id) => placements[i].push(id),
                    None => return fail(format!("line {}: {} is not among the cards looked at", step.line, piece.value)),
                }
            }
        }
        Ok(placements)
    }

    /// `Triggers(A; B)`: each trigger named by its source's name or part of its text, in the order they go on the
    /// stack; every one must be named.
    /// Which applicable replacement effect applies first (CR 616.1), named by its source card or by part of the hint
    /// that effect supplies.
    fn replacement(&self, step: &ActionStep, sources: &[ObjectId], hints: &[&'static str]) -> R<Answer> {
        let named: Vec<&str> = step.positional().collect();
        let offered = || {
            sources.iter().zip(hints).map(|(&s, h)| format!("{} ({h})", self.game.known_name(s))).collect::<Vec<_>>().join(" | ")
        };
        let [spec] = named[..] else {
            return fail(format!("line {}: Replace takes one effect, one of: {}", step.line, offered()));
        };
        let wanted = spec.trim().to_lowercase();
        // A source named with a copy number or a zone (`Ancient Adamantoise#2`, `Bot1.Battlefield.X`): two effects of
        // same-named permanents, told apart as everywhere else.
        if spec.contains('#') || spec.contains('.') {
            let ids = self.objects(step, spec.trim(), sources)?;
            return match ids.first().and_then(|id| sources.iter().position(|s| s == id)) {
                Some(i) => Ok(Answer::Index(i as u8)),
                None => fail(format!("line {}: \"{spec}\" is not one of: {}", step.line, offered())),
            };
        }
        let found = (0..sources.len())
            .find(|&i| self.game.known_name(sources[i]).to_string().to_lowercase() == wanted || hints[i].to_lowercase().contains(&wanted));
        match found {
            Some(i) => Ok(Answer::Index(i as u8)),
            None => fail(format!("line {}: \"{spec}\" is not one of: {}", step.line, offered())),
        }
    }

    fn order_triggers(&self, step: &ActionStep, sources: &[ObjectId], abilities: &[AbilityRef]) -> R<Answer> {
        let mut order: Vec<u32> = Vec::new();
        for spec in step.positional() {
            let wanted = spec.trim().to_lowercase();
            let found = (0..sources.len()).find(|&i| {
                !order.contains(&count_u32(i))
                    && (self.game.known_name(sources[i]).to_string().to_lowercase() == wanted
                        || self.game.cards.ability(abilities[i]).text().to_lowercase().contains(&wanted))
            });
            match found {
                Some(i) => order.push(count_u32(i)),
                None => return fail(format!("line {}: \"{spec}\" is not one of the triggers to order", step.line)),
            }
        }
        if order.len() != sources.len() {
            return fail(format!("line {}: Triggers takes every one of the {} triggers", step.line, sources.len()));
        }
        Ok(Answer::Order(order))
    }

    fn offered(&self, step: &ActionStep, options: &[ActionOption], every_cost: bool) -> R<Vec<ActionOption>> {
        let kind = match step.verb {
            ScriptVerb::Play => ActionKind::PlayLand,
            ScriptVerb::Cast => ActionKind::CastSpell,
            ScriptVerb::Activate => ActionKind::ActivateAbility,
            // A special action (CR 116.2) is an ability of the card that is done at once, named by its keyword.
            ScriptVerb::Foretell | ScriptVerb::Suspend => ActionKind::ActivateAbility,
            _ => return Ok(Vec::new()),
        };
        let g = &self.game;
        let mut offered: Vec<ActionOption> =
            options.iter().filter(|o| o.kind == kind && (self.is_source(step, o.source) || self.is_players_ability(step, o))).cloned().collect();
        if matches!(step.verb, ScriptVerb::Foretell | ScriptVerb::Suspend) {
            let word = step.verb.name();
            offered.retain(|o| o.source.is_some_and(|s| {
                g.chars(s).abilities().nth(o.index as usize).is_some_and(|a| {
                    matches!(g.cards.ability(a), AbilityDef::Activated(d) if d.special_action && d.text.starts_with(word))
                })
            }));
        }
        if let Some(ability) = step.named("Ability") {
            let needle = ability.to_lowercase();
            if let Some(n) = needle.strip_prefix('#').and_then(|n| n.parse::<usize>().ok()) {
                // The card's nth own option (`#1` its mana cost), not paid for by an alternative cost another object gives.
                offered.retain(|o| o.index as usize == n - 1 && (o.kind != ActionKind::CastSpell || o.paid_by.is_none()));
            } else {
                offered.retain(|o| {
                    // An Adventure or an Omen says which face of the card is cast, as the C# engine's description does.
                    let text = describe(g.cards, &g.state, &g.log, o);
                    let text = match o.index {
                        WAY_ADVENTURE => format!("as its Adventure, {text}"),
                        WAY_OMEN => format!("as its Omen, {text}"),
                        WAY_BACK_FACE => format!("as its back face, {text}"),
                        _ => text.to_string(),
                    };
                    text.to_lowercase().contains(&needle)
                });
            }
        } else if kind == ActionKind::CastSpell && !every_cost {
            // A cast named without its way is the card's for its mana cost: by the rules or a permission, not for an
            // alternative cost another object gives nor by a way a permanent gives (which say themselves).
            offered.retain(|o| o.index == 0 && o.paid_by.is_none() && !o.granted_by.is_some_and(|g| g.ability.as_granted_way().is_some()));
        }
        // A refused `Activate(Color: Green)` is offered only where green is among the colors the ability may add; a
        // refused `Activate(Color: Colorless)`, only where colorless is among the mana it may add.
        if every_cost && let Some(text) = step.named("Color") {
            let colorless = is_colorless(text);
            let color = if colorless { None } else { color_named(text, true) };
            if colorless || color.is_some() {
                offered.retain(|o| self.may_add(o, color));
            }
        }
        Ok(offered)
    }

    /// Whether an offered mana ability may add mana of `color` (colorless for `None`): one that adds a color of the
    /// player's choosing, from its choices; one that adds fixed mana, from what it adds. Anything else is not judged.
    fn may_add(&self, option: &ActionOption, color: Option<Color>) -> bool {
        if !option.color_choices.is_empty() {
            return color.is_some_and(|c| option.color_choices.has(c)) || (color.is_none() && option.mana == Some(Mana::C));
        }
        let g = &self.game;
        let Some(source) = option.source else { return true };
        let Some(ability) = g.chars(source).abilities().nth(option.index as usize) else { return true };
        match g.cards.ability(ability) {
            AbilityDef::Activated(d) if d.is_mana_ability() && d.adds.total() > 0 => {
                d.adds.get(color.map_or(Mana::C, |c| c.mana())) > 0
            }
            // An ability that adds no mana (a land's sacrifice-to-draw) is not one that adds this color.
            AbilityDef::Activated(d) if !d.is_mana_ability() => false,
            // Mana of a color to be chosen with no color to choose adds none (CR 106.5): a copy of Sunken Citadel that
            // never chose one.
            AbilityDef::Activated(d) if d.any_color => false,
            _ => true,
        }
    }

    fn is_mana_ability(&self, option: &ActionOption) -> bool {
        let Some(source) = option.source else { return false };
        let g = &self.game;
        let ability = g.chars(source).abilities().nth(option.index as usize);
        matches!(ability.map(|a| g.cards.ability(a)), Some(AbilityDef::Activated(d)) if d.is_mana_ability())
    }

    fn match_option(&self, step: &ActionStep, options: &[ActionOption]) -> R<Option<Answer>> {
        let offered = self.offered(step, options, false)?;
        let Some(first) = offered.first() else { return Ok(None) };
        let on_source: Vec<&ActionOption> = offered.iter().filter(|o| o.source == first.source).collect();
        let mut indices: Vec<u8> = on_source.iter().map(|o| o.index).collect();
        indices.dedup();
        // Two instances of one ability (CR 113.2c: Locus of Enlightenment crafted with two of a card) are interchangeable,
        // as same-named cards in one place are: the first offered is used.
        if let Some(source) = first.source {
            let chars = self.game.chars(source);
            let ability_at = |i: u8| chars.abilities().nth(i as usize);
            if indices.iter().all(|&i| ability_at(i).is_some() && ability_at(i) == ability_at(indices[0])) {
                indices.truncate(1);
            }
        }
        if indices.len() > 1 {
            return fail(format!(
                "line {}: {} has {} abilities to activate; say which with Ability: ...",
                step.line,
                step.card.as_deref().unwrap_or(""),
                indices.len()
            ));
        }
        let option = on_source[0];
        // A mana ability's sacrifice comes with the activation: `Sacrifice:` names it, a lone candidate is taken.
        if option.kind == ActionKind::ActivateAbility && option.targets_required > 0 && self.is_mana_ability(option) {
            let targets = match step.named("Sacrifice").or_else(|| step.named("Tap")) {
                Some(name) => match self.target(step, name, &option.target_candidates)? {
                    Some(t) => vec![t],
                    None => return fail(format!("line {}: {} cannot sacrifice {name}", step.line, step.text)),
                },
                None if option.target_candidates.len() == option.targets_required as usize => option.target_candidates.clone(),
                None => {
                    return fail(format!(
                        "line {}: {} sacrifices one of several permanents; add Sacrifice: (a card name)",
                        step.line, step.text
                    ));
                }
            };
            let mut action = Action::of(option);
            action.targets = targets;
            // The option's own `mana` says colorless may be named; only naming it puts it in the action (CR 106.7).
            action.mana = None;
            action.color = match step.named("Color") {
                Some(text) if is_colorless(text) => {
                    action.mana = Some(Mana::C);
                    None
                }
                Some(text) => match color_named(text, true) {
                    Some(c) => Some(c),
                    None => return fail(format!("line {}: {text} is not a color", step.line)),
                },
                None => None,
            };
            return Ok(Some(Answer::Act(action)));
        }
        let specs: Vec<&str> = step
            .positional()
            .chain(step.arguments.iter().filter(|a| a.key.as_deref() == Some("Target")).map(|a| a.value.as_str()))
            .collect();
        if option.targets_required > 0 && !option.targets_optional && specs.is_empty() {
            return fail(format!("line {}: {} needs a target", step.line, step.text));
        }
        if option.targets_required == 0 && !specs.is_empty() {
            return fail(format!("line {}: {} takes no target", step.line, step.text));
        }
        let mut targets = Vec::new();
        for spec in specs {
            match self.target(step, spec, &option.target_candidates)? {
                Some(t) => targets.push(t),
                None => return Ok(None),
            }
        }
        let color = match step.named("Color") {
            Some(text) => {
                if option.color_choices.is_empty() {
                    return fail(format!("line {}: {} adds no mana of a chosen color", step.line, step.text));
                }
                match color_named(text, true) {
                    Some(c) => Some(c),
                    None => return fail(format!("line {}: {text} is not a color", step.line)),
                }
            }
            None => {
                if !option.color_choices.is_empty() {
                    return fail(format!("line {}: {} adds mana of a color it has to be told; add Color:", step.line, step.text));
                }
                None
            }
        };
        let mut action = Action::of(option);
        action.mana = None;
        action.targets = targets;
        action.color = color;
        Ok(Some(Answer::Act(action)))
    }
}

/// The X or kicker count a line names (`X: 3`, `Kicked: 2`).
fn x_of(step: &ActionStep) -> R<Option<u16>> {
    match step.named("X").or_else(|| step.named("Kicked")) {
        None => Ok(None),
        Some(text) => match text.trim().parse::<u16>() {
            Ok(n) => Ok(Some(n)),
            Err(_) => fail(format!("line {}: {text} is not a number", step.line)),
        },
    }
}

/// The comma-separated names a line's key lists (`Convoke: Llanowar Elves, Birds of Paradise`).
fn names_of(step: &ActionStep, key: &str) -> VecDeque<String> {
    step.named(key).map(|t| t.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect()).unwrap_or_default()
}

/// The colors a line names with `Colors:` (`Colors: Red, Red, Green`).
fn colors_of(step: &ActionStep) -> R<Option<Vec<Color>>> {
    let Some(text) = step.named("Colors") else { return Ok(None) };
    let mut out = Vec::new();
    for piece in text.split(',') {
        match color_named(piece.trim(), true) {
            Some(c) => out.push(c),
            None => return fail(format!("line {}: Colors: {piece} is not a color", step.line)),
        }
    }
    Ok(Some(out))
}

/// Pool mana a `Spend:` line names: its kind, its restriction (CR 106.6), and the name of the source that produced it, for
/// mana a rule tells apart by its source (CR 106.3).
type Spend = (Mana, Option<ManaRestriction>, Option<String>);

/// `Spend:` names pool mana by its symbol, and for mana that carries a restriction its name after it: `R`, `R GainsHaste`;
/// mana told apart by its source names the source after `from`: `W from Barracks of the Thousand`. A restriction's name
/// has a comma in it (`LegendarySpells, CantBeCountered`), so pieces are split where a symbol begins.
fn spend_of(step: &ActionStep) -> R<VecDeque<Spend>> {
    let mut out = VecDeque::new();
    if let Some(text) = step.named("Spend") {
        let mut pieces: Vec<String> = Vec::new();
        for piece in text.split(',') {
            let piece = piece.trim();
            if piece.split_whitespace().next().is_some_and(|w| mana_symbol(w).is_some()) || pieces.is_empty() {
                pieces.push(piece.to_string());
            } else if let Some(last) = pieces.last_mut() {
                last.push_str(", ");
                last.push_str(piece);
            }
        }
        for piece in pieces {
            let mut words = piece.split_whitespace();
            let Some(mana) = words.next().and_then(mana_symbol) else {
                return fail(format!("line {}: Spend: {piece} is not a kind of mana", step.line));
            };
            let mut rest: Vec<&str> = words.collect();
            let from = match rest.iter().position(|w| *w == "from") {
                Some(at) => {
                    let name = rest[at + 1..].join(" ");
                    rest.truncate(at);
                    Some(name)
                }
                None => None,
            };
            let restriction = if rest.is_empty() {
                None
            } else {
                match ManaRestriction::parse(rest.into_iter()) {
                    Some(r) => Some(r),
                    None => return fail(format!("line {}: Spend: {piece} names something that is not a mana restriction", step.line)),
                }
            };
            out.push_back((mana, restriction, from));
        }
    }
    Ok(out)
}

/// The text of every mode a choice of modes asked by `source` could have, wherever it is offered: its card's faces and
/// halves by name, the fused cast of a fuse card, its modes, its own ways of being cast and alternative costs, and the
/// labels its card names. What a refusal of an absent mode (`! Bot1.Modes(Fuse)`) is checked against, so that a mode
/// the source never could offer is a typo, not a refusal that holds.
fn modes_the_source_could_offer(game: &Game, source: ObjectId) -> Vec<&'static str> {
    if source.0 as usize >= game.state.objects.len() {
        return Vec::new();
    }
    let def = game.cards.get(game.state.obj(source).card);
    let mut out = vec![def.name];
    if let Some((left, right)) = def.split {
        out.extend([left.name, right.name]);
    }
    if def.fuse {
        out.push(label_text(&Label::Fused(game.state.obj(source).card)));
    }
    out.extend(def.inset_faces().map(|(_, f)| f.name));
    out.extend(def.back.map(|b| b.name));
    let faces = std::iter::once(def).chain(def.split.into_iter().flat_map(|(l, r)| [l, r])).chain(def.inset_faces().map(|(_, f)| f)).chain(def.back);
    for face in faces {
        out.extend(face.modal.iter().flat_map(|m| m.modes.iter().map(|mode| mode.text)));
        out.extend(face.ways.iter().map(|w| w.text));
        out.extend(face.spell_alternatives.iter().map(|a| a.text));
        out.extend(face.labels.iter().copied());
    }
    out
}

/// A count as a failure report words it: "2", "up to 2", "any number".
fn count_words(count: Count) -> String {
    match count {
        Count::Exactly(n) | Count::AsManyAsCan(n) => n.to_string(),
        Count::UpTo(n) => format!("up to {n}"),
        Count::AnyNumber => "any number of".to_string(),
    }
}
