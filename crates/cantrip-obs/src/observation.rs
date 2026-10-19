//! The observation: one seat's view of a game at one decision, by canonical index.

use serde::{Deserialize, Serialize};

/// Bumped whenever the shape or the meaning of an observation changes: a model is trained on one version.
pub const SCHEMA_VERSION: u32 = 1;

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
    pub lands_played: u8,
    pub mulligans: u8,
    /// Mana in the pool, W U B R G C.
    pub pool: [u8; 6],
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
}
