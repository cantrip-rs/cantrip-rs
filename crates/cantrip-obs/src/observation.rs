//! The observation: one seat's view of a game at one decision, by canonical index.

use serde::{Deserialize, Serialize};

/// Bumped whenever the shape or the meaning of an observation changes: a model is trained on one version.
pub const SCHEMA_VERSION: u32 = 2;

/// How much of the hidden game the seat is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InfoMode {
    /// Everything, hidden zones and their order included: for a teacher that sees the whole game, never a player.
    Full,
    /// What a player knows, and each opponent's decklist less the cards of it the seat can see (`pools`): the
    /// tournament seat, which knows what its opponent registered.
    Decklist,
    /// What a player knows, and nothing of the opponents' decks it has not seen.
    Hidden,
}

/// One seat's view of a game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub schema: u32,
    pub mode: InfoMode,
    /// The seat it is for.
    pub seat: u8,
    pub turn: TurnObs,
    /// Each player, by seat.
    pub players: Vec<PlayerObs>,
    /// Every object the seat can tell is there, by canonical index: those it sees, the stack's abilities, and a token
    /// for each hidden card.
    pub objects: Vec<ObjectObs>,
    /// The latest public events, oldest first.
    pub events: Vec<EventObs>,
    /// The question outstanding: who is asked and what kind of question. Its candidates only for the seat asked.
    pub request: Option<RequestObs>,
    /// The seat's own decklist, as registered: its library at the deal.
    pub decklist: Vec<Count>,
    /// Decklist mode: each opponent's decklist less the cards of it the seat can see now; empty for the seat and in
    /// the other modes.
    pub pools: Vec<Vec<Count>>,
    /// The cards the seat has seen each opponent hold this game, wherever they are now; empty for the seat.
    pub seen: Vec<Vec<Count>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnObs {
    pub number: u32,
    pub active: u8,
    /// The step, as the engine numbers them (CR 500: untap is 0).
    pub step: u8,
    pub priority: u8,
    pub starting: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerObs {
    pub life: i32,
    pub poison: u8,
    pub energy: u8,
    pub lost: bool,
    pub lands_played: u32,
    pub mulligans: u8,
    /// Mana in the pool, W U B R G C.
    pub pool: [u32; 6],
    /// How many cards are in their hand and library.
    pub hand: u16,
    pub library: u16,
    pub monarch: bool,
    pub speed: Option<u8>,
    /// Cards the seat knows are in this library without knowing where (a card shuffled in that it saw), by name.
    pub library_known: Vec<String>,
}

/// A name and how many.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Count {
    pub name: String,
    pub count: u16,
}

/// The kinds of place, in canonical order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ZoneObs {
    Library,
    Hand,
    Graveyard,
    Exile,
    Sideboard,
    Battlefield,
    PhasedOut,
    Stack,
    Command,
}

/// A place: a zone, and whose for the zones a player has (0 for the shared ones).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Place {
    pub zone: ZoneObs,
    pub player: u8,
}

/// What something points at: nothing, a player by seat, or an object by canonical index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Ref {
    None,
    Player(u8),
    Object(u32),
}

/// One object, by canonical index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectObs {
    /// An object the seat sees: what it is now and its state.
    Visible(Box<VisibleObs>),
    /// A card the seat cannot see: a token in its zone. `position` where the zone has an order the seat can follow
    /// (a library); `identity` where the seat knows what it is all the same (a card it saw put there).
    Hidden { place: Place, owner: u8, position: Option<u16>, identity: Option<String>, face_down: bool },
    /// An activated or triggered ability on the stack (`kind` 1 activated, 2 triggered), at `position` from the bottom.
    Ability { position: u16, source: Ref, name: String, kind: u8, controller: u8, targets: Vec<Ref>, x: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleObs {
    pub place: Place,
    /// From the top of a library, the bottom of a graveyard and of the stack; none in an unordered zone.
    pub position: Option<u16>,
    pub owner: u8,
    pub controller: u8,
    /// None for a face-down object whose face the seat may not see.
    pub name: Option<String>,
    pub chars: CharsObs,
    pub face_down: bool,
    pub token: bool,
    pub tapped: bool,
    pub damage: u16,
    /// Counters by kind, as the engine numbers them, where there are any.
    pub counters: Vec<(u16, i16)>,
    /// What it attacks, while it does, and whether it was blocked.
    pub attacking: Option<Ref>,
    pub blocked: bool,
    /// The attackers it blocks.
    pub blocking: Vec<Ref>,
    /// Summoning sick (CR 302.6).
    pub sick: bool,
    /// The object or player it is attached to.
    pub attached: Ref,
    /// A spell on the stack: its targets and X.
    pub spell: Option<SpellObs>,
}

/// Characteristics as the layers make them (CR 613), as bitsets in the engine's numbering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharsObs {
    pub types: u16,
    /// Basic, legendary, snow.
    pub supertypes: u8,
    pub colors: u8,
    pub subtypes: Vec<u16>,
    pub power: Option<i16>,
    pub toughness: Option<i16>,
    pub keywords: u64,
    pub mana_value: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellObs {
    pub controller: u8,
    pub targets: Vec<Ref>,
    pub x: u16,
}

/// A public event: its kind (the bit of the engine's `EventMask`), the player and objects it is about, and for a
/// zone change where from and to. An object no longer to be told apart (gone into a hidden zone) is `Ref::None`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventObs {
    pub turn: u32,
    pub kind: u8,
    pub player: Option<u8>,
    pub objects: Vec<Ref>,
    pub from: Option<Place>,
    pub to: Option<Place>,
    pub amount: i32,
}

/// The question outstanding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestObs {
    pub player: u8,
    pub kind: String,
    /// The objects it names, by canonical index: for the seat asked; empty for anyone else.
    pub candidates: Vec<u32>,
    /// Everything the question allows, in canonical terms, for the seat asked; `None` for anyone else. What the
    /// factoring (`crate::factor`) reads.
    pub ask: Option<Ask>,
}

/// The question asked of the seat, with every constraint an answer must meet, by canonical index. Lists the engine
/// orders are sorted canonically (by `Ref`, or by the whole entry), so that nothing of the engine's numbering shows;
/// a list whose order the rules give (the cards looked at, top first) keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ask {
    /// The spell or ability being announced may be given up (CR 733.1): `Cancel` is an answer.
    pub may_cancel: bool,
    /// The object the question is about (the spell paid for, the ability resolving); `Ref::None` where it has none.
    pub source: Ref,
    pub kind: AskKind,
}

/// A mana cost: generic, W U B R G C symbols, hybrid pairs (as mana numbers), Phyrexian W U B R G, X symbols.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CostObs {
    pub generic: u8,
    pub symbols: [u8; 6],
    pub hybrid: Vec<(u8, u8)>,
    pub phyrexian: [u8; 5],
    pub x: u8,
}

/// The kinds of action an option is, as the engine numbers them (`ActionKind`).
pub mod action {
    pub const PLAY_LAND: u8 = 0;
    pub const CAST_SPELL: u8 = 1;
    pub const ACTIVATE_ABILITY: u8 = 2;
    pub const CANCEL_CASTING: u8 = 3;
    pub const PAY_COST: u8 = 4;
    pub const DECLINE_COST: u8 = 5;
    pub const ASSIGN_MANA: u8 = 6;
    pub const CONFIRM_CASTING: u8 = 7;
    pub const INCREASE_X: u8 = 8;
    pub const DECREASE_X: u8 = 9;
    pub const TAP_TO_PAY: u8 = 10;
    pub const EXILE_TO_PAY: u8 = 11;
}

/// One option of a priority, payment or "unless" question: an action prototype the answer fills in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OptionObs {
    /// `action::*`.
    pub kind: u8,
    pub source: Ref,
    /// Which way of casting, which ability of the source.
    pub index: u8,
    pub targets_required: u8,
    pub targets_optional: bool,
    /// Who may be chosen for its targets, every slot together, sorted.
    pub target_candidates: Vec<Ref>,
    /// The colors of "one mana of any color" to choose from (bit c for color c, W U B R G).
    pub color_choices: u8,
    pub names_nothing: bool,
    /// Mana W U B R G C as 0..5: the pool mana an `AssignMana` spends; colorless an ability may name.
    pub mana: Option<u8>,
    /// The mana's restriction (CR 106.6), by the engine's name for it.
    pub restriction: Option<String>,
    pub pays_nothing: bool,
    /// The permission a cast or land play is made by (`ActionOption::granted_by`) and the alternative cost another object
    /// gives (`paid_by`): the grant's object and the index of its ability. They tell apart options that are otherwise the
    /// same (a free equip beside the paid one), so an answer maps back to the engine's option; the encoder does not read
    /// them yet (docs/known-issues.md, KI-141).
    #[serde(default)]
    pub granted_by: Option<(Ref, u8)>,
    #[serde(default)]
    pub paid_by: Option<(Ref, u8)>,
}

/// "Total mana value N or less" over a list: each entry's value, index for index, and N.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotalObs {
    pub values: Vec<u16>,
    pub most: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterPoolObs {
    pub object: Ref,
    pub kind: u16,
    pub count: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivisionObs {
    /// The creature whose combat damage it is.
    pub source: Ref,
    pub banding: bool,
    pub damage: u16,
    /// The creatures it may be dealt to, sorted, and what is lethal to each, index for index.
    pub creatures: Vec<Ref>,
    pub lethal: Vec<u16>,
    /// Trample (or as though not blocked): what is left once each creature has its lethal may go to `defender`.
    pub may_assign_to_defender: bool,
    pub defender: Ref,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestinationObs {
    /// 0 library top, 1 library bottom, 2 graveyard, 3 hand, 4 exile, 5 battlefield face down.
    pub kind: u8,
    pub min: u8,
    /// `u8::MAX` for "the rest".
    pub max: u8,
    /// Whether it may take each card, index for index with the cards.
    pub eligible: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AskKind {
    /// Act or pass; options sorted canonically.
    Priority { options: Vec<OptionObs> },
    /// Paying what is owed, or announcing X (`announcing`): `x` so far, `x_most` the most X the seat could pay as far as
    /// it can tell (the loyalty a "−X" may remove; 1 for a kicker without "any number of times").
    PayCosts { total: CostObs, remaining: CostObs, options: Vec<OptionObs>, announcing: bool, x: u16, x_most: u16, loyalty: bool, attackers: Vec<Ref>, life: u32 },
    /// "Unless pays" / "may pay" as it resolves: pay the whole cost or decline. `x` the X chosen so far where the payer
    /// chooses it.
    UnlessPays { subject: Ref, total: CostObs, remaining: CostObs, life: u8, life_payable: bool, verb: String, options: Vec<OptionObs>, x: Option<u16>, x_most: u16 },
    /// `count` (or up to) of `eligible` among `cards`. `purpose` the engine's name for why; `total` index for index with
    /// `eligible`.
    ChooseCards { purpose: String, cards: Vec<Ref>, eligible: Vec<Ref>, count: u8, up_to: bool, total: Option<TotalObs> },
    /// `least` to `count` of `candidates` (sorted), `total` index for index with them.
    ChooseTarget { purpose: String, candidates: Vec<Ref>, optional: bool, least: u8, count: u8, any_number: bool, slot_player: Option<u8>, total: Option<TotalObs> },
    /// Modes by index: `labels` says what each is.
    ChooseModes { labels: Vec<String>, eligible: Vec<u8>, count: u8, at_most: u8, affordable: Vec<u8>, additional_cost: bool, repeat: bool },
    /// `count` colors of `choices` (bits W U B R G); `purpose` the engine's `ColorUse` number.
    ChooseColors { choices: u8, count: u8, purpose: u8 },
    /// A card name: from names supplied by the caller. `looked_at` the cards just seen; `kind` 0 any, 1 nonland, 2 land.
    ChooseName { looked_at: Vec<Ref>, suggestion: Option<String>, kind: u8 },
    /// One of these subtypes (the engine's numbers), sorted.
    ChooseSubtype { choices: Vec<u16>, purpose: u8 },
    May { purpose: String, verb: String },
    ChoosePlayer { candidates: Vec<u8>, verb: String, amount: u16 },
    /// Any of `cards` whose powers (index for index) add up to `minimum`.
    TapForPower { purpose: String, cards: Vec<Ref>, powers: Vec<i16>, minimum: u8 },
    CollectEvidence { cards: Vec<Ref>, values: Vec<u8>, minimum: u8 },
    Craft { cards: Vec<Ref>, minimum: u8, maximum: u8 },
    /// `total` counters from among the pools (sorted).
    RemoveCounters { pools: Vec<CounterPoolObs>, total: u8 },
    /// A total divided among up to `max_targets` (0: any number) of `candidates`, each at least 1.
    ChooseDividedTargets { candidates: Vec<Ref>, total: u16, max_targets: u8, optional: bool },
    /// Each of `eligible` attacks one of `defenders` or stays home; each of `requirements` must attack if able.
    /// `costs` what an attack costs (attacker, defender, mana value of the cost); `can_pay` the most the seat can pay.
    DeclareAttackers { eligible: Vec<Ref>, defenders: Vec<Ref>, requirements: Vec<Ref>, costs: Vec<(Ref, Ref, u16)>, can_pay: u16 },
    /// One division per creature with a choice, sorted by source.
    AssignCombatDamage { divisions: Vec<DivisionObs> },
    /// Each of `eligible` blocks one attacker of `allowed` or none. `required` the blocks effects require if able, of
    /// which a declaration obeys `most`; `min_blockers`/`max_blockers` each attacker's, index for index with `attackers`.
    DeclareBlockers { eligible: Vec<Ref>, attackers: Vec<Ref>, allowed: Vec<(Ref, Ref)>, required: Vec<(Ref, Ref)>, most: u8, min_blockers: Vec<u8>, max_blockers: Vec<u8> },
    DiscardToHandSize { count: u8, hand: Vec<Ref> },
    Mulligan { mulligans: u8 },
    ChooseStartingPlayer { candidates: Vec<u8> },
    /// The triggers to order, sorted canonically (by source, then which of its card's abilities, `abilities` index for
    /// index): the answer lists them first on the stack first.
    OrderTriggers { sources: Vec<Ref>, abilities: Vec<u8> },
    /// The cards looked at, top first, each to one destination; or a shuffle where allowed.
    DistributeCards { cards: Vec<Ref>, destinations: Vec<DestinationObs>, may_shuffle: bool },
    /// `library` (sorted), `eligible` (sorted); `destination` the engine's `SearchDestination` number.
    SearchLibrary { library: Vec<Ref>, eligible: Vec<Ref>, count: u8, optional: bool, destination: u8 },
    /// Which replacement effect applies first: `sources` sorted (with their hints), the event's facts.
    ChooseReplacement { subject: Ref, life: Option<i32>, sources: Vec<Ref>, hints: Vec<String>, damage: Option<(Ref, Ref, u16)>, turn: bool, counters: Vec<(u16, i16)>, draw: bool },
}
