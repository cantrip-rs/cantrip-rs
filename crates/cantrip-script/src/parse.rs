//! The `.cantrip` grammar, as `src/Cantrip.Scripting/GameScript.cs` read it: two ordered
//! decklists, an action list with one cursor, and expectations. The syntax is
//! `scenarios/README.md`.

use std::fmt;

/// A script that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameScript {
    pub name: String,
    /// Each player's deck by card name, top of the library first. Index 0 is Bot1.
    pub decks: Vec<Vec<DeckEntry>>,
    /// Each player's sideboard by card name (a `[Bot1 Sideboard]` section): the cards they own outside the game (CR 100.4,
    /// 400.11a). Index 0 is Bot1; a player with no section has none.
    pub sideboards: Vec<Vec<DeckEntry>>,
    /// A Limited game's basic lands (a `[Basic Lands Outside]` section, one name a line): every sideboard holds any number
    /// of each (`Setup::basic_lands_outside`).
    pub basic_lands_outside: Vec<String>,
    /// Each player's companion, chosen as the deck was built (a `[Bot1 Companion]` section, one card name): dealt into their
    /// sideboard, and revealed before the game if their deck fulfils its condition (CR 103.2b). Index 0 is Bot1.
    pub companions: Vec<Option<String>>,
    /// The format's minimum deck size (a `[Minimum Deck Size]` section: `60` in Constructed, CR 100.2a; `40` in Limited, CR
    /// 100.2b), when not Freeform's none.
    pub minimum_deck_size: Option<u32>,
    pub steps: Vec<ScriptStep>,
    pub expectations: Vec<Expectation>,
    /// Whether the game starts as a real one does (CR 103).
    pub opening: bool,
    /// With `opening`, the seat that chooses who goes first: 1 for Bot1.
    pub chooser: i32,
    /// The faces the dice show, in the order they are rolled (a `[Dice]` section: `6; 1; 4`).
    pub dice: Vec<u8>,
    /// Life totals the players begin with when not 20 (a `[Life]` section: `Bot1: 3`), as (seat index, life).
    pub life: Vec<(usize, i32)>,
    /// The most events the game records before it is a draw, when not the engine's (an `[Event Limit]` section: `5000`;
    /// `cantrip_rules::MAX_GAME_EVENTS`).
    pub event_limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckEntry {
    pub card: String,
    pub copies: usize,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectation {
    pub key: String,
    pub value: String,
    pub line: usize,
    /// `Key != Value`: the key is in the dump and its value is not this one (a library shuffled out of a known order).
    pub negated: bool,
}

impl Expectation {
    /// The prefix of a key about what a client shows rather than the game's state (`Client.Bot1.Stack`): judged by a
    /// browser driving that seat, and skipped by a headless run, which has no client to look at.
    pub const CLIENT: &'static str = "Client.";

    pub fn is_client(&self) -> bool {
        self.key.starts_with(Self::CLIENT)
    }
}

impl fmt::Display for Expectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.key, if self.negated { "!=" } else { "=" }, self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneKind {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Command,
    Sideboard,
}

impl ZoneKind {
    pub fn parse(text: &str) -> Option<ZoneKind> {
        match text.trim().to_ascii_lowercase().as_str() {
            "library" => Some(ZoneKind::Library),
            "hand" => Some(ZoneKind::Hand),
            "battlefield" => Some(ZoneKind::Battlefield),
            "graveyard" => Some(ZoneKind::Graveyard),
            "stack" => Some(ZoneKind::Stack),
            "exile" => Some(ZoneKind::Exile),
            "command" => Some(ZoneKind::Command),
            "sideboard" => Some(ZoneKind::Sideboard),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ZoneKind::Library => "Library",
            ZoneKind::Hand => "Hand",
            ZoneKind::Battlefield => "Battlefield",
            ZoneKind::Graveyard => "Graveyard",
            ZoneKind::Stack => "Stack",
            ZoneKind::Exile => "Exile",
            ZoneKind::Command => "Command",
            ZoneKind::Sideboard => "Sideboard",
        }
    }
}

/// Cantrip's twelve steps of a turn, with the short names scripts and dumps use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TurnStep {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
    PostcombatMain,
    End,
    Cleanup,
}

impl TurnStep {
    pub const ALL: [TurnStep; 12] = [
        TurnStep::Untap,
        TurnStep::Upkeep,
        TurnStep::Draw,
        TurnStep::PrecombatMain,
        TurnStep::BeginCombat,
        TurnStep::DeclareAttackers,
        TurnStep::DeclareBlockers,
        TurnStep::CombatDamage,
        TurnStep::EndOfCombat,
        TurnStep::PostcombatMain,
        TurnStep::End,
        TurnStep::Cleanup,
    ];

    pub fn short(self) -> &'static str {
        match self {
            TurnStep::Untap => "Untap",
            TurnStep::Upkeep => "Upkeep",
            TurnStep::Draw => "Draw",
            TurnStep::PrecombatMain => "Main1",
            TurnStep::BeginCombat => "BeginCombat",
            TurnStep::DeclareAttackers => "Attackers",
            TurnStep::DeclareBlockers => "Blockers",
            TurnStep::CombatDamage => "Damage",
            TurnStep::EndOfCombat => "EndCombat",
            TurnStep::PostcombatMain => "Main2",
            TurnStep::End => "End",
            TurnStep::Cleanup => "Cleanup",
        }
    }

    fn long(self) -> &'static str {
        match self {
            TurnStep::Untap => "Untap",
            TurnStep::Upkeep => "Upkeep",
            TurnStep::Draw => "Draw",
            TurnStep::PrecombatMain => "PrecombatMain",
            TurnStep::BeginCombat => "BeginCombat",
            TurnStep::DeclareAttackers => "DeclareAttackers",
            TurnStep::DeclareBlockers => "DeclareBlockers",
            TurnStep::CombatDamage => "CombatDamage",
            TurnStep::EndOfCombat => "EndOfCombat",
            TurnStep::PostcombatMain => "PostcombatMain",
            TurnStep::End => "End",
            TurnStep::Cleanup => "Cleanup",
        }
    }

    pub fn parse(text: &str) -> Result<TurnStep, ParseError> {
        TurnStep::ALL
            .iter()
            .copied()
            .find(|s| s.short().eq_ignore_ascii_case(text) || s.long().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                ParseError(format!(
                    "\"{text}\" is not a step; expected one of {}",
                    TurnStep::ALL.iter().map(|s| s.short()).collect::<Vec<_>>().join(", ")
                ))
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptVerb {
    Play,
    Cast,
    Activate,
    Foretell,
    Suspend,
    /// Pay {3} and put the revealed companion into the hand (CR 116.2g): `Bot1.Sideboard.Lurrus of the Dream-Den.TakeCompanion`.
    TakeCompanion,
    Pass,
    PayCost,
    Decline,
    Accept,
    Undo,
    Cancel,
    Choose,
    Discard,
    Search,
    Arrange,
    Attack,
    Block,
    AssignDamage,
    Divide,
    RemoveCounters,
    Target,
    Modes,
    Replace,
    Triggers,
    Name,
    Colors,
    Number,
    ChoosePlayer,
    PlayFirst,
    DrawFirst,
    Mulligan,
    Keep,
    Concede,
}

impl ScriptVerb {
    /// The verbs whose refusal, written bare (`! Bot1.Choose`), says that kind of question must not be asked of the player before
    /// their next priority: `Accept` is any "you may", the others the question they answer.
    pub fn is_question(self) -> bool {
        matches!(
            self,
            ScriptVerb::Accept
                | ScriptVerb::Decline
                | ScriptVerb::Discard
                | ScriptVerb::Choose
                | ScriptVerb::Colors
                | ScriptVerb::Number
                | ScriptVerb::Name
                | ScriptVerb::Arrange
                | ScriptVerb::ChoosePlayer
        )
    }

    pub const ALL: [ScriptVerb; 34] = [
        ScriptVerb::Play,
        ScriptVerb::Cast,
        ScriptVerb::Activate,
        ScriptVerb::Foretell,
        ScriptVerb::Suspend,
        ScriptVerb::TakeCompanion,
        ScriptVerb::Pass,
        ScriptVerb::PayCost,
        ScriptVerb::Decline,
        ScriptVerb::Accept,
        ScriptVerb::Undo,
        ScriptVerb::Cancel,
        ScriptVerb::Choose,
        ScriptVerb::Discard,
        ScriptVerb::Search,
        ScriptVerb::Arrange,
        ScriptVerb::Attack,
        ScriptVerb::Block,
        ScriptVerb::AssignDamage,
        ScriptVerb::Divide,
        ScriptVerb::RemoveCounters,
        ScriptVerb::Target,
        ScriptVerb::Modes,
        ScriptVerb::Replace,
        ScriptVerb::Triggers,
        ScriptVerb::Name,
        ScriptVerb::Colors,
        ScriptVerb::Number,
        ScriptVerb::ChoosePlayer,
        ScriptVerb::PlayFirst,
        ScriptVerb::DrawFirst,
        ScriptVerb::Mulligan,
        ScriptVerb::Keep,
        ScriptVerb::Concede,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ScriptVerb::Play => "Play",
            ScriptVerb::Cast => "Cast",
            ScriptVerb::Activate => "Activate",
            ScriptVerb::Foretell => "Foretell",
            ScriptVerb::Suspend => "Suspend",
            ScriptVerb::TakeCompanion => "TakeCompanion",
            ScriptVerb::Pass => "Pass",
            ScriptVerb::PayCost => "PayCost",
            ScriptVerb::Decline => "Decline",
            ScriptVerb::Accept => "Accept",
            ScriptVerb::Undo => "Undo",
            ScriptVerb::Cancel => "Cancel",
            ScriptVerb::Choose => "Choose",
            ScriptVerb::Discard => "Discard",
            ScriptVerb::Search => "Search",
            ScriptVerb::Arrange => "Arrange",
            ScriptVerb::Attack => "Attack",
            ScriptVerb::Block => "Block",
            ScriptVerb::AssignDamage => "AssignDamage",
            ScriptVerb::Divide => "Divide",
            ScriptVerb::RemoveCounters => "RemoveCounters",
            ScriptVerb::Target => "Target",
            ScriptVerb::Modes => "Modes",
            ScriptVerb::Replace => "Replace",
            ScriptVerb::Triggers => "Triggers",
            ScriptVerb::Name => "Name",
            ScriptVerb::Colors => "Colors",
            ScriptVerb::Number => "Number",
            ScriptVerb::ChoosePlayer => "ChoosePlayer",
            ScriptVerb::PlayFirst => "PlayFirst",
            ScriptVerb::DrawFirst => "DrawFirst",
            ScriptVerb::Mulligan => "Mulligan",
            ScriptVerb::Keep => "Keep",
            ScriptVerb::Concede => "Concede",
        }
    }

    pub fn parse(text: &str) -> Result<ScriptVerb, ParseError> {
        match text.to_ascii_uppercase().as_str() {
            "PAY" => return Ok(ScriptVerb::PayCost),
            "ASSIGN" => return Ok(ScriptVerb::AssignDamage),
            _ => {}
        }
        ScriptVerb::ALL.iter().copied().find(|v| v.name().eq_ignore_ascii_case(text)).ok_or_else(|| {
            ParseError(format!(
                "\"{text}\" is not an action; expected one of {}",
                ScriptVerb::ALL.iter().map(|v| v.name()).collect::<Vec<_>>().join(", ")
            ))
        })
    }

    pub fn needs_card(self) -> bool {
        matches!(self, ScriptVerb::Play | ScriptVerb::Cast | ScriptVerb::Activate | ScriptVerb::Foretell | ScriptVerb::Suspend | ScriptVerb::TakeCompanion)
    }
}

/// One argument: a target or card name, or a named value such as `X: 3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptArgument {
    pub key: Option<String>,
    pub value: String,
}

impl ScriptArgument {
    /// The keys recognised before a colon. Anything else is a name, so "Circle of Protection:
    /// Red" stays whole.
    const KEYS: [&'static str; 15] = ["X", "Kicked", "Times", "Color", "Colors", "Ability", "Target", "Attacker", "Sacrifice", "Tap", "Spend", "Convoke", "Delve", "Count", "Mana"];

    pub fn split(raw: &str) -> Vec<ScriptArgument> {
        let mut arguments = Vec::new();
        for piece in raw.split(';') {
            let text = piece.trim();
            if text.is_empty() {
                continue;
            }
            let colon = text.find([':', '=']);
            let key = colon.map(|c| text[..c].trim()).unwrap_or("");
            if let Some(known) = Self::KEYS.iter().find(|k| k.eq_ignore_ascii_case(key)) {
                arguments.push(ScriptArgument { key: Some((*known).to_string()), value: text[colon.unwrap() + 1..].trim().to_string() });
            } else {
                arguments.push(ScriptArgument { key: None, value: text.to_string() });
            }
        }
        arguments
    }
}

#[derive(Debug, Clone)]
pub struct ActionStep {
    pub line: usize,
    pub text: String,
    pub bot: i32,
    pub zone: Option<ZoneKind>,
    pub card: Option<String>,
    /// Which of several same-named cards, counting from 1.
    pub copy: Option<usize>,
    pub verb: ScriptVerb,
    pub raw_arguments: String,
    pub arguments: Vec<ScriptArgument>,
}

impl ActionStep {
    pub fn positional(&self) -> impl Iterator<Item = &str> {
        self.arguments.iter().filter(|a| a.key.is_none()).map(|a| a.value.as_str())
    }

    pub fn named(&self, key: &str) -> Option<&str> {
        self.arguments.iter().find(|a| a.key.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(key))).map(|a| a.value.as_str())
    }

    pub fn parse(line: &str, number: usize) -> Result<ActionStep, ParseError> {
        let mut head = line;
        let mut raw = "";
        if let Some(open) = line.find('(') {
            if !line.ends_with(')') {
                return Err(ParseError("arguments must close with ')' at the end of the line".into()));
            }
            head = line[..open].trim();
            raw = line[open + 1..line.len() - 1].trim();
        }
        let parts: Vec<&str> = head.split('.').collect();
        if parts.len() < 2 {
            return Err(ParseError("expected Bot1.Verb or Bot1.Zone.Card.Verb".into()));
        }
        let bot = bot_index(parts[0].trim())?;
        let verb = ScriptVerb::parse(parts[parts.len() - 1].trim())?;
        let mut zone = None;
        let mut card = None;
        let mut copy = None;
        if parts.len() > 2 {
            let mut first = 1;
            if let Some(parsed) = ZoneKind::parse(parts[1]) {
                if parts.len() > 3 {
                    zone = Some(parsed);
                    first = 2;
                }
            }
            // Rejoined, so a name with a full stop in it survives.
            let mut name = parts[first..parts.len() - 1].join(".").trim().to_string();
            if let Some((base, number)) = split_copy(&name) {
                name = base;
                copy = Some(number);
            }
            card = Some(name);
        }
        if verb.needs_card() != card.is_some() {
            return Err(ParseError(if verb.needs_card() {
                format!("{} needs the card doing it, as Bot1.Hand.Island.{}", verb.name(), verb.name())
            } else {
                format!("{} is not done by a card; write Bot1.{}(...)", verb.name(), verb.name())
            }));
        }
        Ok(ActionStep {
            line: number,
            text: line.to_string(),
            bot,
            zone,
            card,
            copy,
            verb,
            raw_arguments: raw.to_string(),
            arguments: ScriptArgument::split(raw),
        })
    }
}

/// `Island#2` → (`Island`, 2); `#1` alone is the first nameless object, a face-down permanent (CR 708.2a).
pub fn split_copy(name: &str) -> Option<(String, usize)> {
    let hash = name.rfind('#')?;
    let number: usize = name[hash + 1..].parse().ok()?;
    Some((name[..hash].trim().to_string(), number))
}

pub fn bot_index(name: &str) -> Result<i32, ParseError> {
    bot_id(name).ok_or_else(|| ParseError(format!("\"{name}\" is not a player; use Bot1, Bot2, Bot3 or Bot4")))
}

/// Bot1 → 1, Bot2 → 2, anything else → None.
pub fn bot_id(text: &str) -> Option<i32> {
    let upper = text.trim().to_ascii_uppercase();
    match upper.strip_prefix("BOT")?.parse::<i32>() {
        Ok(n @ 1..=4) => Some(n),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct WaitStep {
    pub line: usize,
    pub text: String,
    pub bot: i32,
    pub step: TurnStep,
    pub turn: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HandoverStep {
    pub line: usize,
    pub text: String,
    /// The player whose question to stop at, or 0 for whoever is asked next.
    pub bot: i32,
    pub verb: Option<ScriptVerb>,
}

#[derive(Debug, Clone)]
pub enum ScriptStep {
    Check { line: usize, text: String, expectation: Expectation },
    Refused { line: usize, text: String, action: ActionStep },
    Handover(HandoverStep),
    Wait(WaitStep),
    Action(ActionStep),
}

impl ScriptStep {
    pub fn line(&self) -> usize {
        match self {
            ScriptStep::Check { line, .. } | ScriptStep::Refused { line, .. } => *line,
            ScriptStep::Handover(h) => h.line,
            ScriptStep::Wait(w) => w.line,
            ScriptStep::Action(a) => a.line,
        }
    }

    pub fn text(&self) -> &str {
        match self {
            ScriptStep::Check { text, .. } | ScriptStep::Refused { text, .. } => text,
            ScriptStep::Handover(h) => &h.text,
            ScriptStep::Wait(w) => &w.text,
            ScriptStep::Action(a) => &a.text,
        }
    }

    fn parse(line: &str, number: usize) -> Result<ScriptStep, ParseError> {
        if let Some(rest) = line.strip_prefix('?') {
            let expectation = parse_expectation(rest.trim(), number)?;
            return Ok(ScriptStep::Check { line: number, text: line.to_string(), expectation });
        }
        if let Some(rest) = line.strip_prefix('!') {
            let action = ActionStep::parse(rest.trim(), number)?;
            if !matches!(action.verb, ScriptVerb::Play | ScriptVerb::Cast | ScriptVerb::Activate | ScriptVerb::Foretell | ScriptVerb::Suspend | ScriptVerb::TakeCompanion | ScriptVerb::Block | ScriptVerb::Attack | ScriptVerb::Search | ScriptVerb::Accept | ScriptVerb::Modes | ScriptVerb::Target | ScriptVerb::Undo | ScriptVerb::PayCost | ScriptVerb::RemoveCounters) && !action.verb.is_question() {
                return Err(ParseError("a refusal (!) names a Play, Cast, Activate, Foretell, Suspend, TakeCompanion, Block, Attack, Search, Accept, Modes, Number, Target, Undo or PayCost that must not be offered".into()));
            }
            return Ok(ScriptStep::Refused { line: number, text: line.to_string(), action });
        }
        if line.starts_with('[') && line.ends_with(']') {
            let inner = line[1..line.len() - 1].trim();
            if inner.to_ascii_lowercase().starts_with("handover") {
                return parse_handover(inner, line, number);
            }
            return parse_wait(inner, line, number);
        }
        Ok(ScriptStep::Action(ActionStep::parse(line, number)?))
    }
}

fn parse_handover(inner: &str, line: &str, number: usize) -> Result<ScriptStep, ParseError> {
    let rest = inner["handover".len()..].trim();
    if rest.is_empty() {
        return Ok(ScriptStep::Handover(HandoverStep { line: number, text: line.to_string(), bot: 0, verb: None }));
    }
    let (bot, verb) = rest
        .split_once('.')
        .ok_or_else(|| ParseError("expected [Handover] or a question to stop at, such as [Handover Bot1.AssignDamage]".into()))?;
    let bot = bot_index(bot.trim()).map_err(|_| ParseError("expected [Handover] or a question to stop at, such as [Handover Bot1.AssignDamage]".into()))?;
    let verb = ScriptVerb::parse(verb.trim())?;
    Ok(ScriptStep::Handover(HandoverStep { line: number, text: line.to_string(), bot, verb: Some(verb) }))
}

fn parse_wait(inner: &str, line: &str, number: usize) -> Result<ScriptStep, ParseError> {
    let error = || ParseError("expected a wait such as [Bot1.Main2] or [T3 Bot1.Main2]".into());
    let mut words = inner.split_whitespace();
    let first = words.next().ok_or_else(error)?;
    let (turn, target) = if first.to_ascii_uppercase().starts_with('T') && first[1..].chars().all(|c| c.is_ascii_digit()) && first.len() > 1 {
        (Some(first[1..].parse::<u32>().map_err(|_| error())?), words.next().ok_or_else(error)?)
    } else {
        (None, first)
    };
    if words.next().is_some() {
        return Err(error());
    }
    let (bot, step) = target.split_once('.').ok_or_else(error)?;
    let bot = bot_index(bot.trim()).map_err(|_| error())?;
    let step = TurnStep::parse(step.trim())?;
    Ok(ScriptStep::Wait(WaitStep { line: number, text: line.to_string(), bot, step, turn }))
}

pub fn parse_expectation(line: &str, number: usize) -> Result<Expectation, ParseError> {
    let equals = line.find('=').filter(|i| *i > 0).ok_or_else(|| ParseError("expected \"Key = Value\"".into()))?;
    let (key, negated) = match line[..equals].strip_suffix('!') {
        Some(key) => (key, true),
        None => (&line[..equals], false),
    };
    let expectation = Expectation { key: key.trim().to_string(), value: line[equals + 1..].trim().to_string(), line: number, negated };
    if negated && (expectation.is_client() || expectation.value == "(absent)") {
        return Err(ParseError("\"Key != Value\" is for a key of the game's state with a value: not a Client. key or (absent)".into()));
    }
    Ok(expectation)
}

fn parse_deck_entry(line: &str, number: usize) -> Result<DeckEntry, ParseError> {
    let count = |digits: &str| -> Result<usize, ParseError> {
        let n: usize = digits.parse().map_err(|_| ParseError("a card needs at least one copy".into()))?;
        if n == 0 {
            return Err(ParseError("a card needs at least one copy".into()));
        }
        Ok(n)
    };
    // "4 Island"
    if let Some((first, rest)) = line.split_once(char::is_whitespace) {
        if !first.is_empty() && first.chars().all(|c| c.is_ascii_digit()) && !rest.trim().is_empty() {
            return Ok(DeckEntry { card: rest.trim().to_string(), copies: count(first)?, line: number });
        }
    }
    // "Island x4"
    if let Some((name, last)) = line.rsplit_once(char::is_whitespace) {
        let last = last.trim();
        if (last.starts_with('x') || last.starts_with('X')) && last.len() > 1 && last[1..].chars().all(|c| c.is_ascii_digit()) && !name.trim().is_empty() {
            return Ok(DeckEntry { card: name.trim().to_string(), copies: count(&last[1..])?, line: number });
        }
    }
    Ok(DeckEntry { card: line.to_string(), copies: 1, line: number })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Deck(usize),
    Sideboard(usize),
    Companion(usize),
    MinimumDeckSize,
    BasicLandsOutside,
    Actions,
    Expect,
    Opening,
    Dice,
    Life,
    EventLimit,
}

impl GameScript {
    pub fn load(path: &std::path::Path) -> Result<GameScript, ParseError> {
        let text = std::fs::read_to_string(path).map_err(|e| ParseError(format!("{}: {e}", path.display())))?;
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        GameScript::parse(&text, &name)
    }

    pub fn parse(text: &str, name: &str) -> Result<GameScript, ParseError> {
        let mut script = GameScript { name: name.to_string(), chooser: 1, decks: vec![Vec::new(); 2], ..Default::default() };
        let mut section = Section::None;
        for (index, raw) in text.split('\n').enumerate() {
            let number = index + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            if let Some(header) = section_header(line) {
                section = header;
                script.opening |= header == Section::Opening;
                continue;
            }
            let result: Result<(), ParseError> = (|| {
                match section {
                    Section::Deck(seat) => {
                        if script.decks.len() <= seat {
                            script.decks.resize_with(seat + 1, Vec::new);
                        }
                        script.decks[seat].push(parse_deck_entry(line, number)?)
                    }
                    Section::Sideboard(seat) => {
                        if script.sideboards.len() <= seat {
                            script.sideboards.resize_with(seat + 1, Vec::new);
                        }
                        script.sideboards[seat].push(parse_deck_entry(line, number)?)
                    }
                    Section::Companion(seat) => {
                        if script.companions.len() <= seat {
                            script.companions.resize(seat + 1, None);
                        }
                        let entry = parse_deck_entry(line, number)?;
                        if entry.copies != 1 || script.companions[seat].is_some() {
                            return Err(ParseError("a [Bot1 Companion] section names one card: the companion chosen as the deck was built".into()));
                        }
                        script.companions[seat] = Some(entry.card);
                    }
                    Section::MinimumDeckSize => match line.parse::<u32>() {
                        Ok(n) if script.minimum_deck_size.is_none() => script.minimum_deck_size = Some(n),
                        _ => return Err(ParseError("a [Minimum Deck Size] section gives one number, as 40".into())),
                    },
                    Section::BasicLandsOutside => {
                        let entry = parse_deck_entry(line, number)?;
                        if entry.copies != 1 {
                            return Err(ParseError("a [Basic Lands Outside] section names each basic land once: a sideboard holds any number of it".into()));
                        }
                        script.basic_lands_outside.push(entry.card)
                    }
                    Section::Actions => script.steps.push(ScriptStep::parse(line, number)?),
                    Section::Expect => script.expectations.push(parse_expectation(line.trim_start_matches('?').trim(), number)?),
                    Section::Opening => script.chooser = opening_line(line)?,
                    Section::Life => {
                        let (key, value) = line.split_once(':').unwrap_or(("", ""));
                        let seat = key.trim().to_ascii_uppercase().strip_prefix("BOT").and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=4).contains(n));
                        match (seat, value.trim().parse::<i32>()) {
                            (Some(seat), Ok(life)) => script.life.push((seat - 1, life)),
                            _ => return Err(ParseError("a [Life] section gives a starting life total per player, as Bot1: 3".into())),
                        }
                    }
                    Section::EventLimit => match line.parse::<usize>() {
                        Ok(limit) if script.event_limit.is_none() => script.event_limit = Some(limit),
                        _ => return Err(ParseError("an [Event Limit] section gives one number of events, as 5000".into())),
                    },
                    Section::Dice => {
                        for face in line.split(';') {
                            match face.trim().parse::<u8>() {
                                Ok(f) if (1..=20).contains(&f) => script.dice.push(f),
                                _ => return Err(ParseError("a [Dice] section lists the faces the dice show, as 6; 1; 4".into())),
                            }
                        }
                    }
                    Section::None => return Err(ParseError("expected a section header such as [Bot1 Deck] first".into())),
                }
                Ok(())
            })();
            if let Err(e) = result {
                return Err(ParseError(format!("{name} line {number}: {e}.\n  {line}")));
            }
        }
        Ok(script)
    }

    /// Each deck's card names in order, one entry per copy, top first.
    pub fn decklist(&self, index: usize) -> Vec<String> {
        self.decks.get(index).into_iter().flatten().flat_map(|e| std::iter::repeat_n(e.card.clone(), e.copies)).collect()
    }

    /// Every card the script names for a player, deck and sideboard, one entry per copy.
    pub fn cards(&self) -> Vec<String> {
        (0..self.decks.len())
            .flat_map(|i| self.decklist(i))
            .chain((0..self.sideboards.len()).flat_map(|i| self.sideboard(i)))
            .chain(self.basic_lands_outside.iter().cloned())
            .chain(self.companions.iter().flatten().cloned())
            .collect()
    }

    /// Each sideboard's card names in order, one entry per copy.
    pub fn sideboard(&self, index: usize) -> Vec<String> {
        self.sideboards.get(index).into_iter().flatten().flat_map(|e| std::iter::repeat_n(e.card.clone(), e.copies)).collect()
    }
}

fn section_header(line: &str) -> Option<Section> {
    match line.to_ascii_uppercase().as_str() {
        "[ACTIONS]" => Some(Section::Actions),
        "[EXPECT]" => Some(Section::Expect),
        "[OPENING]" => Some(Section::Opening),
        "[DICE]" => Some(Section::Dice),
        "[LIFE]" => Some(Section::Life),
        "[EVENT LIMIT]" => Some(Section::EventLimit),
        "[BASIC LANDS OUTSIDE]" => Some(Section::BasicLandsOutside),
        "[MINIMUM DECK SIZE]" => Some(Section::MinimumDeckSize),
        other => {
            let rest = other.strip_prefix("[BOT")?;
            if let Some(seat) = rest.strip_suffix(" SIDEBOARD]") {
                let seat = seat.parse::<usize>().ok()?;
                return (1..=4).contains(&seat).then_some(Section::Sideboard(seat - 1));
            }
            if let Some(seat) = rest.strip_suffix(" COMPANION]") {
                let seat = seat.parse::<usize>().ok()?;
                return (1..=4).contains(&seat).then_some(Section::Companion(seat - 1));
            }
            let seat = rest.strip_suffix(" DECK]")?.parse::<usize>().ok()?;
            (1..=4).contains(&seat).then_some(Section::Deck(seat - 1))
        }
    }
}

fn opening_line(line: &str) -> Result<i32, ParseError> {
    let (key, value) = line.split_once(':').unwrap_or(("", ""));
    if key.trim().eq_ignore_ascii_case("Chooser") {
        if value.trim().eq_ignore_ascii_case("Bot2") {
            return Ok(2);
        }
        if value.trim().eq_ignore_ascii_case("Bot1") {
            return Ok(1);
        }
    }
    Err(ParseError("an [Opening] section says only who chooses who goes first: \"Chooser: Bot1\" or \"Chooser: Bot2\"".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_readme_example() {
        let script = GameScript::parse(
            "[Bot1 Deck]\nIsland\n4 Island\nMountain x2\n[Bot2 Deck]\nSwamp\n[Actions]\nBot1.Hand.Island.Play\nBot2.Hand.Duress.Cast(Bot1)\n[T5 Bot1.Main2]\n? Stack.Count = 1\n! Bot1.Hand.Island.Play\n[Handover Bot1.AssignDamage]\n[Expect]\nBot1.Graveyard = Counterspell\n",
            "x",
        )
        .unwrap();
        assert_eq!(script.decklist(0).len(), 7);
        assert_eq!(script.steps.len(), 6);
        match &script.steps[1] {
            ScriptStep::Action(a) => {
                assert_eq!(a.verb, ScriptVerb::Cast);
                assert_eq!(a.zone, Some(ZoneKind::Hand));
                assert_eq!(a.card.as_deref(), Some("Duress"));
                assert_eq!(a.positional().collect::<Vec<_>>(), vec!["Bot1"]);
            }
            other => panic!("{other:?}"),
        }
        match &script.steps[2] {
            ScriptStep::Wait(w) => assert_eq!((w.turn, w.bot, w.step), (Some(5), 1, TurnStep::PostcombatMain)),
            other => panic!("{other:?}"),
        }
        match &script.steps[5] {
            ScriptStep::Handover(h) => assert_eq!((h.bot, h.verb), (1, Some(ScriptVerb::AssignDamage))),
            other => panic!("{other:?}"),
        }
        assert_eq!(script.expectations[0].key, "Bot1.Graveyard");
    }

    #[test]
    fn an_expectation_may_be_negated() {
        let e = parse_expectation("Bot1.Library != Island; Forest", 3).unwrap();
        assert_eq!((e.key.as_str(), e.value.as_str(), e.negated), ("Bot1.Library", "Island; Forest", true));
        assert!(!parse_expectation("Bot1.Library = Island", 3).unwrap().negated);
        assert!(parse_expectation("Client.Bot1.Stack != Island", 3).is_err());
        assert!(parse_expectation("Monarch != (absent)", 3).is_err());
    }

    #[test]
    fn named_arguments_keep_card_names_whole() {
        let step = ActionStep::parse("Bot1.Hand.Circle of Protection: Red.Cast(X: 3; Ability: Adventure; Bot2.Battlefield.Grizzly Bears#2)", 1).unwrap();
        assert_eq!(step.card.as_deref(), Some("Circle of Protection: Red"));
        assert_eq!(step.named("x"), Some("3"));
        assert_eq!(step.named("Ability"), Some("Adventure"));
        assert_eq!(step.positional().collect::<Vec<_>>(), vec!["Bot2.Battlefield.Grizzly Bears#2"]);
        let copy = ActionStep::parse("Bot1.Battlefield.Island#2.Activate", 1).unwrap();
        assert_eq!((copy.card.as_deref(), copy.copy), (Some("Island"), Some(2)));
    }

    #[test]
    fn a_concession_is_a_line_of_its_player() {
        let step = ActionStep::parse("Bot3.Concede", 1).unwrap();
        assert_eq!((step.verb, step.bot), (ScriptVerb::Concede, 3));
    }

    #[test]
    fn parses_a_number_choice_and_its_refusal() {
        let script = GameScript::parse("[Bot1 Deck]\nIsland\n[Bot2 Deck]\nIsland\n[Actions]\nBot1.Number(10)\n! Bot1.Number\n", "x").unwrap();
        assert!(matches!(&script.steps[0], ScriptStep::Action(a) if a.verb == ScriptVerb::Number && a.positional().next() == Some("10")));
        assert!(matches!(&script.steps[1], ScriptStep::Refused { action, .. } if action.verb == ScriptVerb::Number));
    }

    #[test]
    fn a_refusal_may_be_a_bare_block() {
        let script = GameScript::parse("[Bot1 Deck]
Island
[Bot2 Deck]
Island
[Actions]
! Bot2.Block
", "x").unwrap();
        match &script.steps[0] {
            ScriptStep::Refused { action, .. } => assert_eq!((action.verb, action.positional().count()), (ScriptVerb::Block, 0)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_refusal_may_name_accept() {
        let script = GameScript::parse("[Bot1 Deck]
Island
[Bot2 Deck]
Island
[Actions]
! Bot1.Accept
", "x").unwrap();
        match &script.steps[0] {
            ScriptStep::Refused { action, .. } => assert_eq!(action.verb, ScriptVerb::Accept),
            other => panic!("{other:?}"),
        }
        assert!(GameScript::parse("[Bot1 Deck]
Island
[Bot2 Deck]
Island
[Actions]
! Bot1.Discard
", "x").is_ok());
        assert!(GameScript::parse("[Bot1 Deck]
Island
[Bot2 Deck]
Island
[Actions]
! Bot1.Pass
", "x").is_err());
    }
}
