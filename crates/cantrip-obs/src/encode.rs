//! The encoding a network reads (docs/ai-player-plan.md, "Network" > "Inputs"): an observation and the current
//! sub-step's option tokens (`crate::factor`) as at most `MAX_TOKENS` tokens, padded to the smallest of `BUCKETS` that
//! holds them, so that compilers see a few static shapes.
//!
//! Every token has a kind (`kind::*`), a row of the card index (`UNKNOWN_CARD` where there is none), `CATS`
//! categorical ids (`cat::*`, 0 where a slot does not apply), `PTRS` pointers to other tokens (`NO_POINTER` where
//! none) and `NUMS` numbers, scaled by powers of two so that what is an integer stays exact in f16 (each number is
//! rounded to f16 as it is made: the compact form loses nothing). Which number is where is the kind's feature list
//! (`object`, `player`, `game`, `event`, `option`, `pool_card`); `layout_json` writes it all out for the Python side,
//! and `encoder_hash` is the hash of that description.
//!
//! The order is canonical: the game, the players by seat, the objects in the observation's canonical order (a zone's
//! pooled token where its first pooled object is), the pool cards, the latest events oldest first, the answer's choices
//! so far, then the current step's options in the step's order. Equal observations encode equal bytes.
//!
//! What does not fit is pooled. By default a library's cards past its top `LIBRARY_TOP` and a sideboard are each one
//! pooled token (count plus averaged features); that is not a cap hit. Past `MAX_TOKENS`, events are dropped first
//! (oldest first), then pool cards, then whole zones are pooled, in `POOL_ORDER`, keeping the objects the options name;
//! then those too; and last, options are cut. Anything but dropped events sets `cap_hit`.
//!
//! In Full mode every object is visible: a card the seat could not see is a card object with its real card and the
//! `would_be_hidden` bit, which comes from where the card is (a library, an opponent's hand or sideboard, an opponent's
//! face-down permanent), not from what was revealed. Decklist mode adds pool-card tokens for the opponents' unseen
//! decklists; Hidden mode has hidden-object tokens. The mode is a game-token feature.

use std::collections::BTreeMap;

use crate::factor::{Machine, Step, Tok, Token};
use crate::*;

/// Bumped whenever the layout or the meaning of a number changes (and with it `encoder_hash`).
pub const SCHEMA_VERSION: u32 = 1;
pub const BUCKETS: [usize; 3] = [32, 64, 128];
pub const MAX_TOKENS: usize = 128;
pub const NUMS: usize = 96;
pub const CATS: usize = 9;
pub const PTRS: usize = 4;
pub const NO_POINTER: u8 = u8::MAX;
/// The card index's row for a card the encoding does not know.
pub const UNKNOWN_CARD: u32 = 0;
/// A library's cards from the top kept as tokens of their own; the rest are pooled.
pub const LIBRARY_TOP: u16 = 3;
pub const MAX_EVENTS: usize = 8;
/// The answer's choices so far kept, the latest.
pub const MAX_CHOSEN: usize = 16;
/// Seats relative to the observer (`cat::OWNER`, `cat::CONTROLLER`): 1 the seat, 2 the next, ...
pub const MAX_PLAYERS: usize = 8;

/// Token kinds.
pub mod kind {
    pub const PAD: u8 = 0;
    pub const GAME: u8 = 1;
    pub const PLAYER: u8 = 2;
    /// A card the seat sees (in Full mode every card), or an ability on the stack (`object::ABILITY`).
    pub const CARD: u8 = 3;
    pub const HIDDEN: u8 = 4;
    pub const POOL_CARD: u8 = 5;
    pub const EVENT: u8 = 6;
    /// An option of the current step, or a choice already made in this answer (`option::CHOSEN`).
    pub const OPTION: u8 = 7;
    /// Several objects of one zone as one token.
    pub const POOLED: u8 = 8;
    pub const NAMES: [&str; 9] = [
        "pad",
        "game",
        "player",
        "card",
        "hidden",
        "pool_card",
        "event",
        "option",
        "pooled",
    ];
}

/// Categorical slots, and each one's vocabulary size (0 always means "does not apply").
pub mod cat {
    /// 1 + `ZoneObs`: an object's zone, an event's destination.
    pub const ZONE: usize = 0;
    /// 1 + the owner's seat relative to the observer.
    pub const OWNER: usize = 1;
    pub const CONTROLLER: usize = 2;
    /// 1 + the step (CR 500), on the game token.
    pub const STEP: usize = 3;
    /// 1 + `ASK_KINDS` index, on the game token while the seat is asked.
    pub const ASK: usize = 4;
    /// 1 + the event's `EventMask` bit (64: none).
    pub const EVENT: usize = 5;
    /// 1 + `ZoneObs`: where an event's object came from.
    pub const FROM: usize = 6;
    /// 1 + `Tok`.
    pub const TOK: usize = 7;
    /// 1 + `action::*` of a priority/payment option.
    pub const ACTION: usize = 8;
    pub const NAMES: [(&str, usize); super::CATS] = [
        ("zone", 10),
        ("owner", 1 + super::MAX_PLAYERS),
        ("controller", 1 + super::MAX_PLAYERS),
        ("step", 13),
        ("ask", 1 + super::ASK_KINDS.len()),
        ("event", 66),
        ("from", 10),
        ("tok", 1 + super::TOKS.len()),
        ("action", 13),
    ];
}

macro_rules! features {
    ($(#[$doc:meta])* $m:ident { $($(#[$fdoc:meta])* $name:ident = $offset:literal, $width:literal, $scale:literal;)* }) => {
        $(#[$doc])*
        pub mod $m {
            $($(#[$fdoc])* pub const $name: usize = $offset;)*
            /// Name, offset, width, scale (the number is the value divided by it).
            pub const NAMES: &[(&str, usize, usize, f32)] = &[$((stringify!($name), $offset, $width, $scale as f32)),*];
        }
    };
}

features! {
    /// Card objects, hidden objects and pooled tokens (averaged over the pool, `COUNT` the pool's size).
    object {
        POWER = 0, 1, 8;
        TOUGHNESS = 1, 1, 8;
        HAS_PT = 2, 1, 1;
        MANA_VALUE = 3, 1, 8;
        /// The engine's `CardType` bits.
        TYPES = 4, 9, 1;
        /// Basic, legendary, snow.
        SUPERTYPES = 13, 3, 1;
        /// W U B R G.
        COLORS = 16, 5, 1;
        /// The engine's `Keyword` bits 0 to 47.
        KEYWORDS = 21, 48, 1;
        DAMAGE = 69, 1, 8;
        PLUS_ONE = 70, 1, 8;
        MINUS_ONE = 71, 1, 8;
        LOYALTY = 72, 1, 8;
        DEFENSE = 73, 1, 8;
        STUN = 74, 1, 8;
        OTHER_COUNTERS = 75, 1, 8;
        TAPPED = 76, 1, 1;
        SICK = 77, 1, 1;
        FACE_DOWN = 78, 1, 1;
        TOKEN = 79, 1, 1;
        ATTACKING = 80, 1, 1;
        BLOCKED = 81, 1, 1;
        /// How many attackers it blocks.
        BLOCKING = 82, 1, 1;
        ATTACHED = 83, 1, 1;
        SPELL = 84, 1, 1;
        ABILITY = 85, 1, 1;
        TRIGGERED = 86, 1, 1;
        X = 87, 1, 8;
        TARGETS = 88, 1, 1;
        HAS_POSITION = 89, 1, 1;
        POSITION = 90, 1, 16;
        /// A card the seat could not see were this not Full mode.
        WOULD_BE_HIDDEN = 91, 1, 1;
        /// The seat's own card that the other players cannot see (its hand, its library, its face-down permanents).
        HIDDEN_FROM_OTHERS = 92, 1, 1;
        IDENTITY_KNOWN = 93, 1, 1;
        COUNT = 94, 1, 16;
        /// An option of the step or a choice made names it.
        NAMED = 95, 1, 1;
    }
}

features! {
    player {
        LIFE = 0, 1, 16;
        POISON = 1, 1, 8;
        ENERGY = 2, 1, 8;
        LOST = 3, 1, 1;
        LANDS_PLAYED = 4, 1, 1;
        MULLIGANS = 5, 1, 8;
        /// W U B R G C.
        POOL = 6, 6, 8;
        HAND = 12, 1, 8;
        LIBRARY = 13, 1, 64;
        MONARCH = 14, 1, 1;
        HAS_SPEED = 15, 1, 1;
        SPEED = 16, 1, 4;
        LIBRARY_KNOWN = 17, 1, 8;
        SEAT = 18, 1, 1;
        ACTIVE = 19, 1, 1;
        PRIORITY = 20, 1, 1;
        ASKED = 21, 1, 1;
        STARTING = 22, 1, 1;
        /// Cards the seat has seen this player hold.
        SEEN = 23, 1, 16;
        /// Decklist mode: the cards of their decklist the seat has not seen.
        POOL_LEFT = 24, 1, 64;
    }
}

features! {
    game {
        TURN = 0, 1, 16;
        /// Full, Decklist, Hidden.
        MODE = 1, 3, 1;
        /// The seat is asked (the options are its).
        ASKED_SEAT = 4, 1, 1;
        MAY_CANCEL = 5, 1, 1;
        CHOSEN = 6, 1, 16;
        OPTIONS = 7, 1, 64;
        LEGAL = 8, 1, 64;
        /// The ask's own numbers (`ask_numbers`).
        ASK = 9, 6, 8;
        /// What remains to pay: generic, W U B R G C.
        COST = 15, 7, 8;
        HYBRID = 22, 1, 8;
        PHYREXIAN = 23, 1, 8;
        X = 24, 1, 8;
        X_MOST = 25, 1, 8;
        ANNOUNCING = 26, 1, 1;
        LOYALTY = 27, 1, 1;
        OBJECTS = 28, 1, 64;
        CAP_HIT = 29, 1, 1;
        EVENTS = 30, 1, 32;
    }
}

features! {
    event {
        /// Turns ago.
        AGO = 0, 1, 4;
        AMOUNT = 1, 1, 8;
        OBJECTS = 2, 1, 1;
        /// 0 the latest.
        RECENCY = 3, 1, 8;
    }
}

features! {
    option {
        /// The token's number (an option, mode, name, seat index...).
        N = 0, 1, 64;
        /// An X token's value.
        X = 1, 1, 8;
        /// Already chosen in this answer.
        CHOSEN = 2, 1, 1;
        /// Its place among the choices made, 0 the first kept.
        ORDER = 3, 1, 16;
        /// A color token: W U B R G, colorless, none.
        COLOR = 4, 7, 1;
        /// A priority/payment option: which way of casting or which ability.
        INDEX = 11, 1, 8;
        TARGETS_REQUIRED = 12, 1, 4;
        TARGETS_OPTIONAL = 13, 1, 1;
        CANDIDATES = 14, 1, 16;
        COLOR_CHOICES = 15, 5, 1;
        NAMES_NOTHING = 20, 1, 1;
        /// W U B R G C.
        MANA = 21, 6, 1;
        RESTRICTED = 27, 1, 1;
        PAYS_NOTHING = 28, 1, 1;
    }
}

features! {
    pool_card {
        COUNT = 0, 1, 4;
        /// From an opponent's decklist less what the seat has seen (Decklist mode).
        FROM_POOL = 1, 1, 1;
        /// Known to be in the library, where unknown.
        LIBRARY_KNOWN = 2, 1, 1;
    }
}

/// What each kind's pointer slots point at.
pub const POINTERS: [(&str, [&str; PTRS]); 6] = [
    ("game", ["active", "priority", "asked", "source"]),
    ("card", ["attached", "attacking", "blocking", "target"]),
    ("ability", ["source", "", "", "target"]),
    ("event", ["object", "object2", "player", ""]),
    ("option", ["a", "b", "", ""]),
    ("pool_card", ["", "", "", ""]),
];

/// The zones pooled past the cap, in order.
pub const POOL_ORDER: [ZoneObs; 9] = [
    ZoneObs::Sideboard,
    ZoneObs::Library,
    ZoneObs::Exile,
    ZoneObs::Graveyard,
    ZoneObs::Command,
    ZoneObs::PhasedOut,
    ZoneObs::Hand,
    ZoneObs::Battlefield,
    ZoneObs::Stack,
];

pub const ZONES: [&str; 9] = [
    "library",
    "hand",
    "graveyard",
    "exile",
    "sideboard",
    "battlefield",
    "phased_out",
    "stack",
    "command",
];

pub const ASK_KINDS: [&str; 26] = [
    "Priority",
    "PayCosts",
    "UnlessPays",
    "ChooseCards",
    "ChooseTarget",
    "ChooseModes",
    "ChooseColors",
    "ChooseName",
    "ChooseSubtype",
    "May",
    "ChoosePlayer",
    "TapForPower",
    "CollectEvidence",
    "Craft",
    "RemoveCounters",
    "ChooseDividedTargets",
    "DeclareAttackers",
    "AssignCombatDamage",
    "DeclareBlockers",
    "DiscardToHandSize",
    "Mulligan",
    "ChooseStartingPlayer",
    "OrderTriggers",
    "DistributeCards",
    "SearchLibrary",
    "ChooseReplacement",
];

/// Every `Tok`, by its number.
pub const TOKS: [Tok; 28] = [
    Tok::Cancel,
    Tok::Done,
    Tok::Pass,
    Tok::Option,
    Tok::Target,
    Tok::Color,
    Tok::AutoPay,
    Tok::X,
    Tok::Pay,
    Tok::Decline,
    Tok::Pick,
    Tok::Mode,
    Tok::Yes,
    Tok::No,
    Tok::Player,
    Tok::Name,
    Tok::NoName,
    Tok::Subtype,
    Tok::Stay,
    Tok::Attack,
    Tok::NoBlock,
    Tok::Block,
    Tok::Damage,
    Tok::Unit,
    Tok::Place,
    Tok::Shuffle,
    Tok::Next,
    Tok::Index,
];

pub const ACTIONS: [&str; 12] = [
    "PlayLand",
    "CastSpell",
    "ActivateAbility",
    "CancelCasting",
    "PayCost",
    "DeclineCost",
    "AssignMana",
    "ConfirmCasting",
    "IncreaseX",
    "DecreaseX",
    "TapToPay",
    "ExileToPay",
];

/// A card name's row in the card index (`UNKNOWN_CARD` for none). The engine side (`cantrip-ai`) makes the
/// index; a closure will do.
pub trait Cards {
    fn row(&self, name: &str) -> u32;
}

impl<F: Fn(&str) -> u32> Cards for F {
    fn row(&self, name: &str) -> u32 {
        self(name)
    }
}

/// Where a token came from: not part of the tensors, for decoding and for mapping a policy's pick back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Game,
    Player(u8),
    Object(u32),
    /// The pooled objects of a zone, by the zone and the seat whose they are (the controller on the battlefield and the
    /// stack).
    Pooled(ZoneObs, u8),
    PoolCard {
        player: u8,
        name: String,
    },
    Event(usize),
    /// The answer's choice so far, by its index in the choices.
    Chosen(usize),
    /// The current step's option, by its index in the step.
    Option(usize),
}

/// An encoded position: tensors of `bucket` tokens, of which the first `len` are real.
#[derive(Clone, Debug, PartialEq)]
pub struct Encoded {
    pub bucket: usize,
    pub len: usize,
    pub kind: Vec<u8>,
    pub card: Vec<u32>,
    /// `bucket` × `CATS`.
    pub cats: Vec<u8>,
    /// `bucket` × `PTRS`.
    pub ptrs: Vec<u8>,
    /// `bucket` × `NUMS`, each f16-exact.
    pub nums: Vec<f32>,
    /// True at the legal options' tokens only.
    pub legal: Vec<bool>,
    /// The step's options are tokens `first_option..first_option + options`, in the step's order.
    pub first_option: usize,
    pub options: usize,
    pub cap_hit: bool,
    /// `len` entries; empty for a position read back from its compact form.
    pub origin: Vec<Origin>,
}

impl Encoded {
    /// The attention mask: the real tokens.
    pub fn attention(&self) -> Vec<bool> {
        (0..self.bucket).map(|i| i < self.len).collect()
    }

    /// The token of the step's option `i`, if it was kept.
    pub fn option_token(&self, i: usize) -> Option<usize> {
        (i < self.options).then_some(self.first_option + i)
    }

    pub fn nums_of(&self, token: usize) -> &[f32] {
        &self.nums[token * NUMS..(token + 1) * NUMS]
    }

    pub fn cats_of(&self, token: usize) -> &[u8] {
        &self.cats[token * CATS..(token + 1) * CATS]
    }

    pub fn ptrs_of(&self, token: usize) -> &[u8] {
        &self.ptrs[token * PTRS..(token + 1) * PTRS]
    }

    /// The dense tensors' size: kind u8, card u32, cats u8, pointers u8, numbers f16, legal and attention u8.
    pub fn dense_bytes(&self) -> usize {
        self.bucket * (1 + 4 + CATS + PTRS + 2 * NUMS + 2)
    }
}

/// One token being made.
struct Row {
    kind: u8,
    card: u32,
    cats: [u8; CATS],
    ptrs: [Ref; PTRS],
    nums: [f32; NUMS],
    origin: Origin,
}

impl Row {
    fn new(kind: u8, origin: Origin) -> Row {
        Row {
            kind,
            card: UNKNOWN_CARD,
            cats: [0; CATS],
            ptrs: [Ref::None; PTRS],
            nums: [0.0; NUMS],
            origin,
        }
    }

    fn set(&mut self, at: usize, value: f32, scale: f32) {
        self.nums[at] = value / scale;
    }

    fn bits(&mut self, at: usize, width: usize, bits: u64) {
        for i in 0..width {
            self.nums[at + i] = ((bits >> i) & 1) as f32;
        }
    }
}

/// The seat `p` relative to `seat`, as a categorical id: 1 the seat itself.
fn rel(p: u8, seat: u8, players: usize) -> u8 {
    let n = players.max(1);
    (1 + (p as usize + n - seat as usize % n) % n).min(MAX_PLAYERS) as u8
}

/// The place an object is in, and whose it is for pooling: the zone's player, or on the battlefield, phased out and
/// on the stack its controller.
fn group_of(o: &ObjectObs) -> (ZoneObs, u8) {
    match o {
        ObjectObs::Visible(v) => match v.place.zone {
            ZoneObs::Battlefield | ZoneObs::PhasedOut | ZoneObs::Stack => {
                (v.place.zone, v.controller)
            }
            z => (z, v.place.player),
        },
        ObjectObs::Hidden { place, owner, .. } => match place.zone {
            ZoneObs::Battlefield | ZoneObs::PhasedOut | ZoneObs::Stack => (place.zone, *owner),
            z => (z, place.player),
        },
        ObjectObs::Ability { controller, .. } => (ZoneObs::Stack, *controller),
    }
}

fn position_of(o: &ObjectObs) -> Option<u16> {
    match o {
        ObjectObs::Visible(v) => v.position,
        ObjectObs::Hidden { position, .. } => *position,
        ObjectObs::Ability { position, .. } => Some(*position),
    }
}

/// The ask's kind as `ASK_KINDS` index, and its own numbers (`game::ASK`, before scaling).
pub fn ask_numbers(kind: &AskKind) -> (usize, [u32; 6]) {
    use AskKind as K;
    let l = |n: usize| n as u32;
    let b = |x: bool| if x { 8 } else { 0 };
    match kind {
        K::Priority { options } => (0, [0, 0, 0, l(options.len()), 0, 0]),
        K::PayCosts {
            options,
            life,
            attackers,
            ..
        } => (1, [0, 0, 0, l(options.len()), *life, l(attackers.len())]),
        K::UnlessPays {
            life,
            life_payable,
            options,
            x,
            ..
        } => (
            2,
            [
                *life as u32,
                b(*life_payable),
                0,
                l(options.len()),
                x.unwrap_or(0) as u32,
                0,
            ],
        ),
        K::ChooseCards {
            cards,
            eligible,
            count,
            up_to,
            total,
            ..
        } => (
            3,
            [
                *count as u32,
                0,
                b(*up_to),
                l(cards.len()),
                l(eligible.len()),
                total.as_ref().map_or(0, |t| t.most as u32),
            ],
        ),
        K::ChooseTarget {
            candidates,
            optional,
            least,
            count,
            any_number,
            total,
            ..
        } => (
            4,
            [
                *count as u32,
                *least as u32,
                b(*optional),
                l(candidates.len()),
                b(*any_number),
                total.as_ref().map_or(0, |t| t.most as u32),
            ],
        ),
        K::ChooseModes {
            labels,
            eligible,
            count,
            at_most,
            repeat,
            ..
        } => (
            5,
            [
                *count as u32,
                *at_most as u32,
                0,
                l(labels.len()),
                l(eligible.len()),
                b(*repeat),
            ],
        ),
        K::ChooseColors {
            choices,
            count,
            purpose,
        } => (
            6,
            [
                *count as u32,
                *purpose as u32,
                0,
                choices.count_ones(),
                0,
                0,
            ],
        ),
        K::ChooseName {
            looked_at, kind, ..
        } => (7, [0, *kind as u32, 0, l(looked_at.len()), 0, 0]),
        K::ChooseSubtype { choices, purpose } => {
            (8, [0, *purpose as u32, 0, l(choices.len()), 0, 0])
        }
        K::May { .. } => (9, [0; 6]),
        K::ChoosePlayer {
            candidates, amount, ..
        } => (10, [0, 0, 0, l(candidates.len()), *amount as u32, 0]),
        K::TapForPower { cards, minimum, .. } => {
            (11, [0, *minimum as u32, 0, l(cards.len()), 0, 0])
        }
        K::CollectEvidence { cards, minimum, .. } => {
            (12, [0, *minimum as u32, 0, l(cards.len()), 0, 0])
        }
        K::Craft {
            cards,
            minimum,
            maximum,
        } => (
            13,
            [*maximum as u32, *minimum as u32, 0, l(cards.len()), 0, 0],
        ),
        K::RemoveCounters {
            pools,
            total,
            up_to,
        } => (14, [*total as u32, 0, b(*up_to), l(pools.len()), 0, 0]),
        K::ChooseDividedTargets {
            candidates,
            total,
            max_targets,
            optional,
        } => (
            15,
            [
                *max_targets as u32,
                0,
                b(*optional),
                l(candidates.len()),
                *total as u32,
                0,
            ],
        ),
        K::DeclareAttackers {
            eligible,
            defenders,
            requirements,
            can_pay,
            ..
        } => (
            16,
            [
                0,
                // The creatures a requirement is on.
                l(eligible
                    .iter()
                    .filter(|&&c| requirements.iter().any(|r| r.0 == c))
                    .count()),
                0,
                l(eligible.len()),
                l(defenders.len()),
                *can_pay as u32,
            ],
        ),
        K::AssignCombatDamage { divisions } => (
            17,
            [
                0,
                0,
                0,
                l(divisions.len()),
                divisions.iter().map(|d| d.damage as u32).sum(),
                0,
            ],
        ),
        K::DeclareBlockers {
            eligible,
            attackers,
            required,
            most,
            ..
        } => (
            18,
            [
                *most as u32,
                l(required.len()),
                0,
                l(eligible.len()),
                l(attackers.len()),
                0,
            ],
        ),
        K::DiscardToHandSize { count, hand } => (19, [*count as u32, 0, 0, l(hand.len()), 0, 0]),
        K::Mulligan { mulligans } => (20, [*mulligans as u32, 0, 0, 0, 0, 0]),
        K::ChooseStartingPlayer { candidates } => (21, [0, 0, 0, l(candidates.len()), 0, 0]),
        K::OrderTriggers { sources, .. } => (22, [0, 0, 0, l(sources.len()), 0, 0]),
        K::DistributeCards {
            cards,
            destinations,
            may_shuffle,
        } => (
            23,
            [
                0,
                0,
                b(*may_shuffle),
                l(cards.len()),
                l(destinations.len()),
                0,
            ],
        ),
        K::SearchLibrary {
            library,
            eligible,
            count,
            optional,
            destination,
        } => (
            24,
            [
                *count as u32,
                *destination as u32,
                b(*optional),
                l(library.len()),
                l(eligible.len()),
                0,
            ],
        ),
        K::ChooseReplacement { sources, life, .. } => (
            25,
            [
                0,
                0,
                0,
                l(sources.len()),
                life.unwrap_or(0).unsigned_abs(),
                0,
            ],
        ),
    }
}

fn ask_options(kind: &AskKind) -> &[OptionObs] {
    match kind {
        AskKind::Priority { options }
        | AskKind::PayCosts { options, .. }
        | AskKind::UnlessPays { options, .. } => options,
        _ => &[],
    }
}

/// Encodes the machine's current step (and its choices so far) for the seat's observation.
pub fn encode_machine(observation: &Observation, machine: &Machine, cards: &impl Cards) -> Encoded {
    let empty = Step::default();
    encode(
        observation,
        machine.step().unwrap_or(&empty),
        machine.chosen(),
        &machine.names(),
        cards,
    )
}

/// Encodes a position: the observation, the step's options, the answer's choices so far and the names the step's
/// `Name` tokens index (`Machine::names`).
pub fn encode(
    observation: &Observation,
    step: &Step,
    chosen: &[Token],
    names: &[String],
    cards: &impl Cards,
) -> Encoded {
    let obs = observation;
    let n = obs.players.len();
    let seat = obs.seat;
    let r = |p: u8| rel(p, seat, n);
    let ask = obs.request.as_ref().and_then(|q| q.ask.as_ref());

    // The budget: the game, the players, a token for each place's objects at the least, and the options always; the
    // latest choices made.
    let groups: Vec<(ZoneObs, u8)> = obs.objects.iter().map(group_of).collect();
    let mut places: Vec<(ZoneObs, u8)> = groups.clone();
    places.sort_by_key(|&(z, p)| (POOL_ORDER.iter().position(|&o| o == z), p));
    places.dedup();
    let mut cap_hit = false;
    let room = MAX_TOKENS - 1 - n.min(MAX_TOKENS - 1);
    let options = step.tokens.len().min(room - places.len().min(room));
    cap_hit |= options < step.tokens.len();
    let chosen_from = chosen.len().saturating_sub(MAX_CHOSEN.min(room - options));
    cap_hit |= chosen.len() - chosen_from < chosen.len().min(MAX_CHOSEN);
    let chosen = &chosen[chosen_from..];
    let mut room = room - options - chosen.len();

    // The objects the options name stay tokens of their own as long as anything does.
    let mut pinned = vec![false; obs.objects.len()];
    let mut pin = |r: Ref| {
        if let Ref::Object(i) = r
            && let Some(p) = pinned.get_mut(i as usize)
        {
            *p = true;
        }
    };
    for t in step.tokens.iter().chain(chosen) {
        pin(t.a);
        pin(t.b);
    }
    if let Some(a) = ask {
        pin(a.source);
    }
    let mut pooled: Vec<bool> = obs
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            !pinned[i]
                && match groups[i].0 {
                    ZoneObs::Sideboard => true,
                    ZoneObs::Library => position_of(o).is_none_or(|p| p >= LIBRARY_TOP),
                    _ => false,
                }
        })
        .collect();
    let count = |pooled: &[bool]| {
        let mut pools: Vec<(ZoneObs, u8)> = (0..pooled.len())
            .filter(|&i| pooled[i])
            .map(|i| groups[i])
            .collect();
        pools.sort();
        pools.dedup();
        pooled.iter().filter(|&&p| !p).count() + pools.len()
    };
    // Pool place after place until the objects fit; then give back, latest first, what a later place made needless.
    let default = pooled.clone();
    let mut done: Vec<(ZoneObs, u8)> = Vec::new();
    for keep_pinned in [true, false] {
        for &place in &places {
            if count(&pooled) <= room {
                break;
            }
            let before = pooled.clone();
            for i in 0..pooled.len() {
                if groups[i] == place && (!keep_pinned || !pinned[i]) {
                    pooled[i] = true;
                }
            }
            if before != pooled {
                cap_hit = true;
                done.push(place);
            }
        }
    }
    for &place in done.iter().rev().skip(1) {
        let mut undone = pooled.clone();
        for i in 0..undone.len() {
            if groups[i] == place {
                undone[i] = default[i];
            }
        }
        if count(&undone) <= room {
            pooled = undone;
        }
    }
    let objects = count(&pooled);
    assert!(objects <= room, "pooling every zone fits the cap");
    room -= objects;

    // Pool cards: the opponents' unseen decklists (Decklist mode), and cards known to be in a library.
    let mut pool_cards: Vec<(u8, String, u16, bool)> = Vec::new();
    for (p, pool) in obs.pools.iter().enumerate() {
        for c in pool {
            pool_cards.push((p as u8, c.name.clone(), c.count, true));
        }
    }
    for (p, player) in obs.players.iter().enumerate() {
        let mut known: BTreeMap<&str, u16> = BTreeMap::new();
        for name in &player.library_known {
            *known.entry(name).or_default() += 1;
        }
        for (name, count) in known {
            pool_cards.push((p as u8, name.to_string(), count, false));
        }
    }
    if pool_cards.len() > room {
        cap_hit = true;
        pool_cards.truncate(room);
    }
    room -= pool_cards.len();
    let events = obs.events.len().min(MAX_EVENTS).min(room);
    let events = &obs.events[obs.events.len() - events..];

    // Token indices: the game, the players, then the objects and their pools.
    let first_object = 1 + n;
    let mut object_token = vec![0usize; obs.objects.len()];
    let mut pool_token: BTreeMap<(ZoneObs, u8), usize> = BTreeMap::new();
    let mut next = first_object;
    let mut order: Vec<Result<usize, (ZoneObs, u8)>> = Vec::new();
    for i in 0..obs.objects.len() {
        if pooled[i] {
            let token = *pool_token.entry(groups[i]).or_insert_with(|| {
                order.push(Err(groups[i]));
                next += 1;
                next - 1
            });
            object_token[i] = token;
        } else {
            order.push(Ok(i));
            object_token[i] = next;
            next += 1;
        }
    }
    let pointer = |r: Ref| -> u8 {
        match r {
            Ref::None => NO_POINTER,
            Ref::Player(p) if (p as usize) < n => (1 + p as usize) as u8,
            Ref::Object(i) if (i as usize) < object_token.len() => object_token[i as usize] as u8,
            _ => NO_POINTER,
        }
    };

    let mut rows: Vec<Row> = Vec::with_capacity(MAX_TOKENS);

    // The game.
    let mut g = Row::new(kind::GAME, Origin::Game);
    g.cats[cat::STEP] = 1 + obs.turn.step.min(11);
    g.set(game::TURN, obs.turn.number as f32, 16.0);
    g.nums[game::MODE + obs.mode as usize] = 1.0;
    g.set(game::CHOSEN, (chosen_from + chosen.len()) as f32, 16.0);
    g.set(game::OPTIONS, step.tokens.len() as f32, 64.0);
    g.set(
        game::LEGAL,
        step.mask.iter().filter(|&&m| m).count() as f32,
        64.0,
    );
    g.set(game::OBJECTS, obs.objects.len() as f32, 64.0);
    g.set(game::EVENTS, obs.events.len() as f32, 32.0);
    g.nums[game::CAP_HIT] = cap_hit as u8 as f32;
    g.ptrs[0] = Ref::Player(obs.turn.active);
    g.ptrs[1] = Ref::Player(obs.turn.priority);
    if let Some(q) = &obs.request {
        g.ptrs[2] = Ref::Player(q.player);
        g.nums[game::ASKED_SEAT] = (q.player == seat) as u8 as f32;
    }
    if let Some(a) = ask {
        let (k, numbers) = ask_numbers(&a.kind);
        g.cats[cat::ASK] = 1 + k as u8;
        g.ptrs[3] = a.source;
        g.nums[game::MAY_CANCEL] = a.may_cancel as u8 as f32;
        for (i, &v) in numbers.iter().enumerate() {
            g.set(game::ASK + i, v as f32, 8.0);
        }
        let cost = |g: &mut Row, c: &CostObs| {
            g.set(game::COST, c.generic as f32, 8.0);
            for (i, &s) in c.symbols.iter().enumerate() {
                g.set(game::COST + 1 + i, s as f32, 8.0);
            }
            g.set(game::HYBRID, c.hybrid.len() as f32, 8.0);
            g.set(
                game::PHYREXIAN,
                c.phyrexian.iter().map(|&p| p as u32).sum::<u32>() as f32,
                8.0,
            );
        };
        match &a.kind {
            AskKind::PayCosts {
                remaining,
                announcing,
                x,
                x_most,
                loyalty,
                ..
            } => {
                cost(&mut g, remaining);
                g.set(game::X, *x as f32, 8.0);
                g.set(game::X_MOST, *x_most as f32, 8.0);
                g.nums[game::ANNOUNCING] = *announcing as u8 as f32;
                g.nums[game::LOYALTY] = *loyalty as u8 as f32;
            }
            AskKind::UnlessPays {
                remaining,
                x,
                x_most,
                ..
            } => {
                cost(&mut g, remaining);
                g.set(game::X, x.unwrap_or(0) as f32, 8.0);
                g.set(game::X_MOST, *x_most as f32, 8.0);
            }
            _ => {}
        }
    }
    rows.push(g);

    // The players.
    for (p, pl) in obs.players.iter().enumerate() {
        let p8 = p as u8;
        let mut row = Row::new(kind::PLAYER, Origin::Player(p8));
        row.cats[cat::OWNER] = r(p8);
        row.cats[cat::CONTROLLER] = r(p8);
        row.set(player::LIFE, pl.life as f32, 16.0);
        row.set(player::POISON, pl.poison as f32, 8.0);
        row.set(player::ENERGY, pl.energy as f32, 8.0);
        row.nums[player::LOST] = pl.lost as u8 as f32;
        row.set(player::LANDS_PLAYED, pl.lands_played as f32, 1.0);
        row.set(player::MULLIGANS, pl.mulligans as f32, 8.0);
        for (i, &m) in pl.pool.iter().enumerate() {
            row.set(player::POOL + i, m as f32, 8.0);
        }
        row.set(player::HAND, pl.hand as f32, 8.0);
        row.set(player::LIBRARY, pl.library as f32, 64.0);
        row.nums[player::MONARCH] = pl.monarch as u8 as f32;
        if let Some(s) = pl.speed {
            row.nums[player::HAS_SPEED] = 1.0;
            row.set(player::SPEED, s as f32, 4.0);
        }
        row.set(player::LIBRARY_KNOWN, pl.library_known.len() as f32, 8.0);
        row.nums[player::SEAT] = (p8 == seat) as u8 as f32;
        row.nums[player::ACTIVE] = (p8 == obs.turn.active) as u8 as f32;
        row.nums[player::PRIORITY] = (p8 == obs.turn.priority) as u8 as f32;
        row.nums[player::ASKED] = obs.request.as_ref().is_some_and(|q| q.player == p8) as u8 as f32;
        row.nums[player::STARTING] = (p8 == obs.turn.starting) as u8 as f32;
        let sum = |c: Option<&Vec<Count>>| {
            c.map_or(0, |c| c.iter().map(|c| c.count as u32).sum::<u32>()) as f32
        };
        row.set(player::SEEN, sum(obs.seen.get(p)), 16.0);
        row.set(player::POOL_LEFT, sum(obs.pools.get(p)), 64.0);
        rows.push(row);
    }

    // The objects, and the pools where their first object is.
    let full = obs.mode == InfoMode::Full;
    for entry in &order {
        match *entry {
            Ok(i) => rows.push(object_row(obs, i as u32, &pinned, full, cards, r)),
            Err(group) => {
                let members: Vec<usize> = (0..obs.objects.len())
                    .filter(|&i| pooled[i] && groups[i] == group)
                    .collect();
                let mut row = Row::new(kind::POOLED, Origin::Pooled(group.0, group.1));
                row.cats[cat::ZONE] = 1 + group.0 as u8;
                row.cats[cat::OWNER] = r(group.1);
                row.cats[cat::CONTROLLER] = r(group.1);
                for &i in &members {
                    let one = object_row(obs, i as u32, &pinned, full, cards, r);
                    for k in 0..NUMS {
                        row.nums[k] += one.nums[k];
                    }
                }
                for k in 0..NUMS {
                    row.nums[k] /= members.len() as f32;
                }
                row.set(object::COUNT, members.len() as f32, 16.0);
                rows.push(row);
            }
        }
    }

    for (p, name, count, from_pool) in &pool_cards {
        let mut row = Row::new(
            kind::POOL_CARD,
            Origin::PoolCard {
                player: *p,
                name: name.clone(),
            },
        );
        row.card = cards.row(name);
        row.cats[cat::OWNER] = r(*p);
        row.set(pool_card::COUNT, *count as f32, 4.0);
        row.nums[if *from_pool {
            pool_card::FROM_POOL
        } else {
            pool_card::LIBRARY_KNOWN
        }] = 1.0;
        rows.push(row);
    }

    let first_event = obs.events.len() - events.len();
    for (k, e) in events.iter().enumerate() {
        let mut row = Row::new(kind::EVENT, Origin::Event(first_event + k));
        row.cats[cat::EVENT] = 1 + e.kind.min(64);
        if let Some(to) = e.to {
            row.cats[cat::ZONE] = 1 + to.zone as u8;
        }
        if let Some(from) = e.from {
            row.cats[cat::FROM] = 1 + from.zone as u8;
        }
        if let Some(p) = e.player {
            row.cats[cat::OWNER] = r(p);
            row.ptrs[2] = Ref::Player(p);
        }
        row.ptrs[0] = e.objects.first().copied().unwrap_or(Ref::None);
        row.ptrs[1] = e.objects.get(1).copied().unwrap_or(Ref::None);
        row.set(
            event::AGO,
            obs.turn.number.saturating_sub(e.turn) as f32,
            4.0,
        );
        row.set(event::AMOUNT, e.amount as f32, 8.0);
        row.set(event::OBJECTS, e.objects.len() as f32, 1.0);
        row.set(event::RECENCY, (events.len() - 1 - k) as f32, 8.0);
        rows.push(row);
    }

    let option_row = |t: &Token, origin: Origin| {
        let mut row = Row::new(kind::OPTION, origin);
        row.cats[cat::TOK] = 1 + t.kind as u8;
        row.ptrs[0] = t.a;
        row.ptrs[1] = t.b;
        row.set(option::N, t.n as f32, 64.0);
        match t.kind {
            Tok::X => row.set(option::X, t.n as f32, 8.0),
            Tok::Color => row.nums[option::COLOR + (t.n as usize).min(6)] = 1.0,
            Tok::Name => {
                row.card = names
                    .get(t.n as usize)
                    .map_or(UNKNOWN_CARD, |s| cards.row(s))
            }
            Tok::Player => row.cats[cat::OWNER] = r(t.n as u8),
            Tok::Option => {
                if let Some(o) = ask.and_then(|a| ask_options(&a.kind).get(t.n as usize)) {
                    row.cats[cat::ACTION] = 1 + o.kind.min(11);
                    row.set(option::INDEX, o.index as f32, 8.0);
                    row.set(option::TARGETS_REQUIRED, o.targets_required as f32, 4.0);
                    row.nums[option::TARGETS_OPTIONAL] = o.targets_optional as u8 as f32;
                    row.set(option::CANDIDATES, o.target_candidates.len() as f32, 16.0);
                    row.bits(option::COLOR_CHOICES, 5, o.color_choices as u64);
                    row.nums[option::NAMES_NOTHING] = o.names_nothing as u8 as f32;
                    if let Some(m) = o.mana {
                        row.nums[option::MANA + (m as usize).min(5)] = 1.0;
                    }
                    row.nums[option::RESTRICTED] = o.restriction.is_some() as u8 as f32;
                    row.nums[option::PAYS_NOTHING] = o.pays_nothing as u8 as f32;
                }
            }
            _ => {}
        }
        row
    };
    for (k, t) in chosen.iter().enumerate() {
        let mut row = option_row(t, Origin::Chosen(chosen_from + k));
        row.nums[option::CHOSEN] = 1.0;
        row.set(option::ORDER, k as f32, 16.0);
        rows.push(row);
    }
    let first_option = rows.len();
    for (k, t) in step.tokens[..options].iter().enumerate() {
        rows.push(option_row(t, Origin::Option(k)));
    }

    let len = rows.len();
    assert!(len <= MAX_TOKENS);
    let bucket = *BUCKETS.iter().find(|&&b| b >= len).expect("within the cap");
    let mut e = Encoded {
        bucket,
        len,
        kind: vec![kind::PAD; bucket],
        card: vec![UNKNOWN_CARD; bucket],
        cats: vec![0; bucket * CATS],
        ptrs: vec![NO_POINTER; bucket * PTRS],
        nums: vec![0.0; bucket * NUMS],
        legal: vec![false; bucket],
        first_option,
        options,
        cap_hit,
        origin: Vec::with_capacity(len),
    };
    for (t, row) in rows.into_iter().enumerate() {
        e.kind[t] = row.kind;
        e.card[t] = row.card;
        e.cats[t * CATS..(t + 1) * CATS].copy_from_slice(&row.cats);
        for (k, &p) in row.ptrs.iter().enumerate() {
            e.ptrs[t * PTRS + k] = pointer(p);
        }
        for (k, &v) in row.nums.iter().enumerate() {
            e.nums[t * NUMS + k] = round16(v);
        }
        e.origin.push(row.origin);
    }
    for k in 0..options {
        e.legal[first_option + k] = step.mask.get(k).copied().unwrap_or(false);
    }
    e
}

fn object_row(
    obs: &Observation,
    i: u32,
    pinned: &[bool],
    full: bool,
    cards: &impl Cards,
    r: impl Fn(u8) -> u8,
) -> Row {
    let seat = obs.seat;
    let o = &obs.objects[i as usize];
    let mut row = Row::new(kind::CARD, Origin::Object(i));
    row.nums[object::NAMED] = pinned[i as usize] as u8 as f32;
    row.nums[object::COUNT] = 1.0 / 16.0;
    let position = |row: &mut Row, p: Option<u16>| {
        if let Some(p) = p {
            row.nums[object::HAS_POSITION] = 1.0;
            row.set(object::POSITION, p as f32, 16.0);
        }
    };
    // Hidden from the seat, but for Full mode; the seat's own, hidden from the others.
    let hidden = |zone: ZoneObs, owner: u8, controller: u8, face_down: bool| match zone {
        ZoneObs::Library => (true, owner == seat),
        ZoneObs::Hand | ZoneObs::Sideboard => (owner != seat, owner == seat),
        _ => (
            face_down && controller != seat,
            face_down && controller == seat,
        ),
    };
    match o {
        ObjectObs::Visible(v) => {
            row.card = v.name.as_deref().map_or(UNKNOWN_CARD, |s| cards.row(s));
            row.cats[cat::ZONE] = 1 + v.place.zone as u8;
            row.cats[cat::OWNER] = r(v.owner);
            row.cats[cat::CONTROLLER] = r(v.controller);
            let c = &v.chars;
            if let (Some(p), Some(t)) = (c.power, c.toughness) {
                row.nums[object::HAS_PT] = 1.0;
                row.set(object::POWER, p as f32, 8.0);
                row.set(object::TOUGHNESS, t as f32, 8.0);
            } else if let Some(p) = c.power.or(c.toughness) {
                row.set(
                    if c.power.is_some() {
                        object::POWER
                    } else {
                        object::TOUGHNESS
                    },
                    p as f32,
                    8.0,
                );
            }
            row.set(object::MANA_VALUE, c.mana_value as f32, 8.0);
            row.bits(object::TYPES, 9, c.types as u64);
            row.bits(object::SUPERTYPES, 3, c.supertypes as u64);
            row.bits(object::COLORS, 5, c.colors as u64);
            row.bits(object::KEYWORDS, 48, c.keywords);
            row.set(object::DAMAGE, v.damage as f32, 8.0);
            for &(k, n) in &v.counters {
                let at = match k {
                    0 => object::PLUS_ONE,
                    1 => object::MINUS_ONE,
                    2 => object::LOYALTY,
                    5 => object::STUN,
                    7 => object::DEFENSE,
                    _ => object::OTHER_COUNTERS,
                };
                row.nums[at] += n as f32 / 8.0;
            }
            row.nums[object::TAPPED] = v.tapped as u8 as f32;
            row.nums[object::SICK] = v.sick as u8 as f32;
            row.nums[object::FACE_DOWN] = v.face_down as u8 as f32;
            row.nums[object::TOKEN] = v.token as u8 as f32;
            row.nums[object::ATTACKING] = v.attacking.is_some() as u8 as f32;
            row.nums[object::BLOCKED] = v.blocked as u8 as f32;
            row.set(object::BLOCKING, v.blocking.len() as f32, 1.0);
            row.nums[object::ATTACHED] = (v.attached != Ref::None) as u8 as f32;
            row.nums[object::IDENTITY_KNOWN] = v.name.is_some() as u8 as f32;
            position(&mut row, v.position);
            let (would, others) = hidden(v.place.zone, v.owner, v.controller, v.face_down);
            row.nums[object::WOULD_BE_HIDDEN] = (full && would) as u8 as f32;
            row.nums[object::HIDDEN_FROM_OTHERS] = others as u8 as f32;
            row.ptrs[0] = v.attached;
            row.ptrs[1] = v.attacking.unwrap_or(Ref::None);
            row.ptrs[2] = v.blocking.first().copied().unwrap_or(Ref::None);
            if let Some(s) = &v.spell {
                row.nums[object::SPELL] = 1.0;
                row.set(object::X, s.x as f32, 8.0);
                row.set(object::TARGETS, s.targets.len() as f32, 1.0);
                row.ptrs[3] = s.targets.first().copied().unwrap_or(Ref::None);
            }
        }
        ObjectObs::Hidden {
            place,
            owner,
            position: p,
            identity,
            face_down,
        } => {
            row.kind = kind::HIDDEN;
            row.card = identity.as_deref().map_or(UNKNOWN_CARD, |s| cards.row(s));
            row.cats[cat::ZONE] = 1 + place.zone as u8;
            row.cats[cat::OWNER] = r(*owner);
            row.cats[cat::CONTROLLER] = r(*owner);
            row.nums[object::FACE_DOWN] = *face_down as u8 as f32;
            row.nums[object::IDENTITY_KNOWN] = identity.is_some() as u8 as f32;
            row.nums[object::WOULD_BE_HIDDEN] = 1.0;
            row.nums[object::HIDDEN_FROM_OTHERS] =
                hidden(place.zone, *owner, *owner, *face_down).1 as u8 as f32;
            position(&mut row, *p);
        }
        ObjectObs::Ability {
            position: p,
            source,
            name,
            kind: k,
            controller,
            targets,
            x,
        } => {
            row.card = cards.row(name);
            row.cats[cat::ZONE] = 1 + ZoneObs::Stack as u8;
            row.cats[cat::OWNER] = r(*controller);
            row.cats[cat::CONTROLLER] = r(*controller);
            row.nums[object::ABILITY] = 1.0;
            row.nums[object::TRIGGERED] = (*k == 2) as u8 as f32;
            row.nums[object::IDENTITY_KNOWN] = 1.0;
            row.set(object::X, *x as f32, 8.0);
            row.set(object::TARGETS, targets.len() as f32, 1.0);
            position(&mut row, Some(*p));
            row.ptrs[0] = *source;
            row.ptrs[3] = targets.first().copied().unwrap_or(Ref::None);
        }
    }
    row
}

/// What a position's option tokens name, read back through their pointers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pointee {
    None,
    Player(u8),
    Object(u32),
    /// An object pooled into this zone's token.
    Pooled(ZoneObs, u8),
    /// A token of another kind.
    Other(usize),
}

/// A position read back: the options (their token kind, what they point at, their number) and how many objects each
/// zone holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    pub options: Vec<(Tok, Pointee, Pointee, u16)>,
    pub chosen: Vec<(Tok, Pointee, Pointee, u16)>,
    /// Objects by (zone, the seat whose they are: as `Origin::Pooled`), pools counted by their size.
    pub zones: BTreeMap<(ZoneObs, u8), u32>,
}

/// Reads an encoded position back (it needs `origin`, so not one read from its compact form).
pub fn decode(e: &Encoded) -> Decoded {
    let pointee = |p: u8| -> Pointee {
        if p == NO_POINTER {
            return Pointee::None;
        }
        match &e.origin[p as usize] {
            Origin::Player(s) => Pointee::Player(*s),
            Origin::Object(i) => Pointee::Object(*i),
            Origin::Pooled(z, s) => Pointee::Pooled(*z, *s),
            _ => Pointee::Other(p as usize),
        }
    };
    let option = |t: usize| {
        let tok = TOKS[(e.cats_of(t)[cat::TOK] - 1) as usize];
        let ptrs = e.ptrs_of(t);
        (
            tok,
            pointee(ptrs[0]),
            pointee(ptrs[1]),
            (e.nums_of(t)[option::N] * 64.0).round() as u16,
        )
    };
    let mut d = Decoded {
        options: Vec::new(),
        chosen: Vec::new(),
        zones: BTreeMap::new(),
    };
    for t in 0..e.len {
        match &e.origin[t] {
            Origin::Option(_) => d.options.push(option(t)),
            Origin::Chosen(_) => d.chosen.push(option(t)),
            Origin::Object(_) | Origin::Pooled(..) => {
                let zone = ZONE_OF[(e.cats_of(t)[cat::ZONE] - 1) as usize];
                let whose = match &e.origin[t] {
                    Origin::Pooled(_, s) => *s,
                    _ => decode_group(e, t),
                };
                *d.zones.entry((zone, whose)).or_default() +=
                    (e.nums_of(t)[object::COUNT] * 16.0).round() as u32;
            }
            _ => {}
        }
    }
    d
}

/// The seat whose an object token's object is, as pooling groups it, from its categorical ids: the controller on the
/// battlefield and the stack, else the owner. Relative ids are turned back into seats with the game token's seat.
fn decode_group(e: &Encoded, t: usize) -> u8 {
    let players = (0..e.len).filter(|&i| e.kind[i] == kind::PLAYER).count();
    let seat = (1..=players)
        .find(|&i| e.nums_of(i)[player::SEAT] == 1.0)
        .map_or(0, |i| i - 1);
    let zone = ZONE_OF[(e.cats_of(t)[cat::ZONE] - 1) as usize];
    let id = match zone {
        ZoneObs::Battlefield | ZoneObs::PhasedOut | ZoneObs::Stack => e.cats_of(t)[cat::CONTROLLER],
        _ => e.cats_of(t)[cat::OWNER],
    };
    ((id as usize - 1 + seat) % players.max(1)) as u8
}

const ZONE_OF: [ZoneObs; 9] = [
    ZoneObs::Library,
    ZoneObs::Hand,
    ZoneObs::Graveyard,
    ZoneObs::Exile,
    ZoneObs::Sideboard,
    ZoneObs::Battlefield,
    ZoneObs::PhasedOut,
    ZoneObs::Stack,
    ZoneObs::Command,
];

/// An f32 as the nearest f16 (round to nearest, ties to even), as bits; a finite number beyond the f16's range is held at
/// the largest finite f16 (65504), never made an infinity. An encoded number has no bound of its own (an event's amount:
/// damage, life, counters put); written as an infinity it reached the network as one and came back a NaN value (a search
/// root value of null in self-play shards, 2026-10-28). Only the network's input is held there: the game's numbers are the
/// engine's, exact. `decode`'s sanitising in `ai/cantrip_ai/dataset.py` reads an old shard's infinity the same way.
pub fn f16_bits(x: f32) -> u16 {
    let b = x.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32;
    let man = b & 0x7f_ffff;
    if exp == 0xff {
        return sign | 0x7c00 | if man != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 0x1f {
        return sign | 0x7bff;
    }
    if e <= 0 {
        if e < -10 {
            return sign;
        }
        let m = man | 0x80_0000;
        let shift = (14 - e) as u32;
        let half = 1u32 << (shift - 1);
        let rest = m & ((1 << shift) - 1);
        let mut h = m >> shift;
        if rest > half || (rest == half && h & 1 == 1) {
            h += 1;
        }
        return sign | h as u16;
    }
    let mut h = ((e as u32) << 10) | (man >> 13);
    let rest = man & 0x1fff;
    if rest > 0x1000 || (rest == 0x1000 && h & 1 == 1) {
        h += 1;
    }
    // Rounded up past the largest finite f16: held there.
    sign | h.min(0x7bff) as u16
}

pub fn f16_value(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = ((h >> 10) & 0x1f) as i32;
    let man = (h & 0x3ff) as f32;
    sign * match exp {
        0 => man * 2f32.powi(-24),
        0x1f if man == 0.0 => f32::INFINITY,
        0x1f => f32::NAN,
        e => (1.0 + man / 1024.0) * 2f32.powi(e - 15),
    }
}

fn round16(x: f32) -> f32 {
    f16_value(f16_bits(x))
}

/// The compact form, for shards: a header (schema, bucket, len, first option, options, cap hit), the legal bits of the
/// options, then per real token its kind, card (u16), categorical ids, pointers, a bitmask of its non-zero numbers and
/// those numbers as f16. Padding is not stored.
pub fn to_compact(e: &Encoded) -> Vec<u8> {
    let mut out = vec![
        SCHEMA_VERSION as u8,
        e.bucket as u8,
        e.len as u8,
        e.first_option as u8,
        e.options as u8,
        e.cap_hit as u8,
    ];
    let mut bits = vec![0u8; e.options.div_ceil(8)];
    for k in 0..e.options {
        if e.legal[e.first_option + k] {
            bits[k / 8] |= 1 << (k % 8);
        }
    }
    out.extend(bits);
    for t in 0..e.len {
        out.push(e.kind[t]);
        let card = u16::try_from(e.card[t]).expect("a card index of under 65536 rows");
        out.extend(card.to_le_bytes());
        out.extend(e.cats_of(t));
        out.extend(e.ptrs_of(t));
        let nums = e.nums_of(t);
        let mut mask = [0u8; NUMS / 8];
        for (k, &v) in nums.iter().enumerate() {
            if v != 0.0 {
                mask[k / 8] |= 1 << (k % 8);
            }
        }
        out.extend(mask);
        for &v in nums.iter().filter(|&&v| v != 0.0) {
            out.extend(f16_bits(v).to_le_bytes());
        }
    }
    out
}

/// A position from its compact form (without `origin`). `None` for another schema or bytes that are not one.
pub fn from_compact(bytes: &[u8]) -> Option<Encoded> {
    let mut at = 0;
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = bytes.get(at..at + n)?;
        at += n;
        Some(s)
    };
    let h = take(6)?;
    if h[0] as u32 != SCHEMA_VERSION {
        return None;
    }
    let (bucket, len, first_option, options, cap_hit) = (
        h[1] as usize,
        h[2] as usize,
        h[3] as usize,
        h[4] as usize,
        h[5] != 0,
    );
    if !BUCKETS.contains(&bucket) || len > bucket || first_option + options > len {
        return None;
    }
    let bits = take(options.div_ceil(8))?.to_vec();
    let mut e = Encoded {
        bucket,
        len,
        kind: vec![kind::PAD; bucket],
        card: vec![UNKNOWN_CARD; bucket],
        cats: vec![0; bucket * CATS],
        ptrs: vec![NO_POINTER; bucket * PTRS],
        nums: vec![0.0; bucket * NUMS],
        legal: vec![false; bucket],
        first_option,
        options,
        cap_hit,
        origin: Vec::new(),
    };
    for k in 0..options {
        e.legal[first_option + k] = bits[k / 8] & (1 << (k % 8)) != 0;
    }
    for t in 0..len {
        e.kind[t] = take(1)?[0];
        let c = take(2)?;
        e.card[t] = u16::from_le_bytes([c[0], c[1]]) as u32;
        e.cats[t * CATS..(t + 1) * CATS].copy_from_slice(take(CATS)?);
        e.ptrs[t * PTRS..(t + 1) * PTRS].copy_from_slice(take(PTRS)?);
        let mask = take(NUMS / 8)?.to_vec();
        for k in 0..NUMS {
            if mask[k / 8] & (1 << (k % 8)) != 0 {
                let v = take(2)?;
                e.nums[t * NUMS + k] = f16_value(u16::from_le_bytes([v[0], v[1]]));
            }
        }
    }
    (at == bytes.len()).then_some(e)
}

/// The tensors (not `origin`) of two positions are equal.
pub fn same_tensors(a: &Encoded, b: &Encoded) -> bool {
    (
        a.bucket,
        a.len,
        &a.kind,
        &a.card,
        &a.cats,
        &a.ptrs,
        &a.legal,
        a.first_option,
        a.options,
        a.cap_hit,
    ) == (
        b.bucket,
        b.len,
        &b.kind,
        &b.card,
        &b.cats,
        &b.ptrs,
        &b.legal,
        b.first_option,
        b.options,
        b.cap_hit,
    ) && a
        .nums
        .iter()
        .zip(&b.nums)
        .all(|(x, y)| x.to_bits() == y.to_bits())
}

fn tok_name(t: Tok) -> String {
    format!("{t:?}")
}

/// The layout, as JSON: what the Python side reads (checked in as `ai/encoding.json`), and what `encoder_hash` hashes.
pub fn layout_json() -> String {
    let list = |v: Vec<String>| format!("[{}]", v.join(", "));
    let q = |s: &str| format!("\"{s}\"");
    let features = |names: &[(&str, usize, usize, f32)]| {
        list(
            names
                .iter()
                .map(|(n, o, w, s)| {
                    format!(
                        "{{\"name\": {}, \"offset\": {o}, \"width\": {w}, \"scale\": {s}}}",
                        q(&n.to_lowercase())
                    )
                })
                .collect(),
        )
    };
    let mut s = String::from("{\n");
    s += &format!("  \"schema\": {SCHEMA_VERSION},\n");
    s += &format!("  \"observation_schema\": {},\n", crate::SCHEMA_VERSION);
    s += &format!(
        "  \"buckets\": {},\n",
        list(BUCKETS.iter().map(|b| b.to_string()).collect())
    );
    s += &format!(
        "  \"max_tokens\": {MAX_TOKENS},\n  \"nums\": {NUMS},\n  \"cats\": {CATS},\n  \"ptrs\": {PTRS},\n"
    );
    s += &format!(
        "  \"no_pointer\": {NO_POINTER},\n  \"unknown_card\": {UNKNOWN_CARD},\n  \"library_top\": {LIBRARY_TOP},\n"
    );
    s += &format!(
        "  \"max_events\": {MAX_EVENTS},\n  \"max_chosen\": {MAX_CHOSEN},\n  \"max_players\": {MAX_PLAYERS},\n"
    );
    s += &format!(
        "  \"kinds\": {},\n",
        list(kind::NAMES.iter().map(|n| q(n)).collect())
    );
    s += &format!(
        "  \"categorical\": {},\n",
        list(
            cat::NAMES
                .iter()
                .enumerate()
                .map(|(i, (n, v))| format!("{{\"name\": {}, \"slot\": {i}, \"vocab\": {v}}}", q(n)))
                .collect()
        )
    );
    s += "  \"features\": {\n";
    let all: [(&str, &[(&str, usize, usize, f32)]); 6] = [
        ("object", object::NAMES),
        ("player", player::NAMES),
        ("game", game::NAMES),
        ("event", event::NAMES),
        ("option", option::NAMES),
        ("pool_card", pool_card::NAMES),
    ];
    s += &all
        .iter()
        .map(|(k, f)| format!("    {}: {}", q(k), features(f)))
        .collect::<Vec<_>>()
        .join(",\n");
    s += "\n  },\n";
    s += &format!(
        "  \"feature_lists\": {{\"card\": \"object\", \"hidden\": \"object\", \"pooled\": \"object\", \"player\": \"player\", \"game\": \"game\", \"event\": \"event\", \"option\": \"option\", \"pool_card\": \"pool_card\"}},\n"
    );
    s += &format!(
        "  \"pointers\": {{{}}},\n",
        POINTERS
            .iter()
            .map(|(k, p)| format!("{}: {}", q(k), list(p.iter().map(|x| q(x)).collect())))
            .collect::<Vec<_>>()
            .join(", ")
    );
    s += &format!(
        "  \"zones\": {},\n",
        list(ZONES.iter().map(|z| q(z)).collect())
    );
    s += &format!(
        "  \"pool_order\": {},\n",
        list(POOL_ORDER.iter().map(|z| q(ZONES[*z as usize])).collect())
    );
    s += &format!(
        "  \"toks\": {},\n",
        list(TOKS.iter().map(|t| q(&tok_name(*t))).collect())
    );
    s += &format!(
        "  \"ask_kinds\": {},\n",
        list(ASK_KINDS.iter().map(|t| q(t)).collect())
    );
    s += &format!(
        "  \"actions\": {},\n",
        list(ACTIONS.iter().map(|t| q(t)).collect())
    );
    s += "  \"ask_numbers\": \"per ask kind: see cantrip_obs::encode::ask_numbers\"\n";
    s += "}\n";
    s
}

/// FNV-1a, 64 bits.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The hash of the layout (`layout_json`): a shard's header and a model's manifest carry it, and a loader refuses a
/// model whose hash differs.
pub fn encoder_hash() -> u64 {
    fnv1a64(layout_json().as_bytes())
}

/// A readable dump of a position, token by token (the golden files).
pub fn dump(e: &Encoded) -> String {
    let mut s = format!(
        "bucket {} len {} first_option {} options {} cap_hit {} compact {} bytes dense {} bytes\n",
        e.bucket,
        e.len,
        e.first_option,
        e.options,
        e.cap_hit,
        to_compact(e).len(),
        e.dense_bytes()
    );
    for t in 0..e.len {
        let nums: Vec<String> = e
            .nums_of(t)
            .iter()
            .enumerate()
            .filter(|(_, v)| **v != 0.0)
            .map(|(k, v)| format!("{k}:{v}"))
            .collect();
        let ptrs: Vec<String> = e
            .ptrs_of(t)
            .iter()
            .map(|&p| {
                if p == NO_POINTER {
                    "-".into()
                } else {
                    p.to_string()
                }
            })
            .collect();
        s += &format!(
            "{t:3} {:9} card {:5} cats {:?} ptrs [{}]{} nums {}\n",
            kind::NAMES[e.kind[t] as usize],
            e.card[t],
            e.cats_of(t),
            ptrs.join(" "),
            if e.legal[t] { " legal" } else { "" },
            nums.join(" ")
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_fit_and_do_not_overlap() {
        for names in [
            object::NAMES,
            player::NAMES,
            game::NAMES,
            event::NAMES,
            option::NAMES,
            pool_card::NAMES,
        ] {
            let mut end = 0;
            for &(name, offset, width, scale) in names {
                assert_eq!(offset, end, "{name} starts where the one before it ends");
                assert!(
                    scale >= 1.0 && (scale as u32).is_power_of_two(),
                    "{name}: a power of two keeps integers exact"
                );
                end = offset + width;
            }
            assert!(end <= NUMS);
        }
        for (i, t) in TOKS.iter().enumerate() {
            assert_eq!(*t as usize, i);
        }
        assert_eq!(cat::NAMES.len(), CATS);
    }

    #[test]
    fn a_finite_number_beyond_f16_is_held_at_its_largest_never_an_infinity() {
        for x in [65520.0f32, 1.0e6, 3.0e38, -1.0e6] {
            let h = f16_bits(x);
            assert_eq!(h & 0x7fff, 0x7bff, "{x}");
            assert_eq!(f16_value(h).abs(), 65504.0, "{x}");
        }
        // Rounding up into the overflow is held too.
        assert_eq!(f16_bits(65519.0) & 0x7fff, 0x7bff);
    }

    #[test]
    fn f16_round_trips_and_rounds_to_nearest() {
        for x in [
            0.0f32,
            1.0,
            -1.0,
            0.125,
            17.0 / 16.0,
            65504.0,
            1.0 / 3.0,
            2.5e-5,
            -0.0625,
            1000.0 / 64.0,
        ] {
            let h = f16_bits(x);
            assert_eq!(f16_bits(f16_value(h)), h, "{x}");
            assert!(
                (f16_value(h) - x).abs() <= x.abs() / 1024.0 + 6e-8,
                "{x} -> {}",
                f16_value(h)
            );
        }
        for n in 0..2048u16 {
            assert_eq!(f16_value(f16_bits(n as f32 / 64.0)) * 64.0, n as f32);
        }
    }

    fn row(name: &str) -> u32 {
        1 + (fnv1a64(name.as_bytes()) % 1000) as u32
    }

    fn step(tokens: &[Token], legal: &[bool]) -> Step {
        Step {
            tokens: tokens.to_vec(),
            mask: legal.to_vec(),
        }
    }

    fn blocks() -> Step {
        step(
            &[
                Token {
                    kind: Tok::NoBlock,
                    a: Ref::Object(2),
                    b: Ref::None,
                    n: 0,
                },
                Token {
                    kind: Tok::Block,
                    a: Ref::Object(2),
                    b: Ref::Object(3),
                    n: 0,
                },
                Token::new(Tok::Done),
            ],
            &[true, true, false],
        )
    }

    fn bear(zone: ZoneObs, player: u8, position: Option<u16>) -> ObjectObs {
        let ObjectObs::Visible(mut v) = crate::tests::sample().objects[2].clone() else {
            unreachable!()
        };
        v.place = Place { zone, player };
        v.owner = player;
        v.controller = player;
        v.position = position;
        ObjectObs::Visible(v)
    }

    /// The sample with `library` cards in the seat's library and `battlefield` bears on its battlefield.
    fn crowded(library: u16, battlefield: usize) -> Observation {
        let mut obs = crate::tests::sample();
        obs.mode = InfoMode::Full;
        let mut objects: Vec<ObjectObs> = (0..library)
            .map(|p| bear(ZoneObs::Library, 0, Some(p)))
            .collect();
        objects.extend((0..battlefield).map(|_| bear(ZoneObs::Battlefield, 0, None)));
        let at = objects.len() as u32;
        objects.push(obs.objects[2].clone());
        let ObjectObs::Ability {
            source, targets, ..
        } = &mut obs.objects[3]
        else {
            unreachable!()
        };
        *source = Ref::Object(at);
        *targets = vec![Ref::Object(0), Ref::Player(0)];
        objects.push(obs.objects[3].clone());
        obs.objects = objects;
        obs.events.clear();
        obs
    }

    fn crowded_step(obs: &Observation) -> Step {
        let blocker = Ref::Object(obs.objects.len() as u32 - 2);
        let attacker = Ref::Object(obs.objects.len() as u32 - 1);
        step(
            &[
                Token {
                    kind: Tok::Block,
                    a: blocker,
                    b: attacker,
                    n: 0,
                },
                Token::new(Tok::Done),
            ],
            &[true, true],
        )
    }

    #[test]
    fn the_sample_encodes_in_the_smallest_bucket_and_round_trips() {
        let obs = crate::tests::sample();
        let s = blocks();
        let e = encode(&obs, &s, &[], &[], &row);
        assert_eq!((e.bucket, e.cap_hit), (32, false));
        // The game, two players, four objects, two pool cards (Shock in the pool, Shock known in the library), an
        // event, three options.
        assert_eq!(e.len, 1 + 2 + 4 + 2 + 1 + 3);
        assert_eq!((e.first_option, e.options), (10, 3));
        assert_eq!(e.legal.iter().filter(|&&l| l).count(), 2);
        assert_eq!(e.attention().iter().filter(|&&a| a).count(), e.len);
        // The block names the bear (token 5) and the ability (token 6).
        assert_eq!(e.ptrs_of(11)[..2], [5, 6]);
        assert_eq!(e.card[3], row("Forest"));
        assert_eq!(e.card[4], UNKNOWN_CARD);
        let back = from_compact(&to_compact(&e)).unwrap();
        assert!(same_tensors(&e, &back));
        assert_eq!(to_compact(&back), to_compact(&e));
        assert!(same_tensors(&e, &encode(&obs, &s, &[], &[], &row)));
        let d = decode(&e);
        assert_eq!(
            d.options[1],
            (Tok::Block, Pointee::Object(2), Pointee::Object(3), 0)
        );
        assert_eq!(d.zones[&(ZoneObs::Library, 0)], 2);
        assert_eq!(d.zones[&(ZoneObs::Stack, 1)], 1);
        let mut bad = to_compact(&e);
        bad[0] += 1;
        assert!(from_compact(&bad).is_none());
        assert!(from_compact(&to_compact(&e)[..20]).is_none());
    }

    #[test]
    fn choices_made_are_tokens_before_the_options_and_never_legal() {
        let obs = crate::tests::sample();
        let chosen = [Token {
            kind: Tok::Block,
            a: Ref::Object(2),
            b: Ref::Object(3),
            n: 0,
        }];
        let e = encode(&obs, &blocks(), &chosen, &[], &row);
        assert_eq!(e.first_option, 11);
        assert_eq!(e.kind[10], kind::OPTION);
        assert_eq!(e.nums_of(10)[option::CHOSEN], 1.0);
        assert!(!e.legal[10]);
        assert_eq!(decode(&e).chosen.len(), 1);
    }

    #[test]
    fn a_library_past_its_top_is_pooled_without_a_cap_hit() {
        let obs = crowded(20, 0);
        let e = encode(&obs, &crowded_step(&obs), &[], &[], &row);
        assert!(!e.cap_hit);
        let pooled: Vec<usize> = (0..e.len).filter(|&t| e.kind[t] == kind::POOLED).collect();
        assert_eq!(pooled.len(), 1);
        // The top three and the one the ability targets (the top card) are cards; the pool is the other seventeen.
        assert_eq!(e.nums_of(pooled[0])[object::COUNT] * 16.0, 17.0);
        assert_eq!(decode(&e).zones[&(ZoneObs::Library, 0)], 20);
        assert_eq!(e.nums_of(pooled[0])[object::WOULD_BE_HIDDEN], 1.0);
    }

    #[test]
    fn past_the_cap_zones_are_pooled_keeping_what_the_options_name() {
        for (battlefield, bucket) in [(40, 64), (100, 128), (300, 32)] {
            let obs = crowded(60, battlefield);
            let s = crowded_step(&obs);
            let e = encode(&obs, &s, &[], &[], &row);
            assert_eq!(e.bucket, bucket, "{battlefield}");
            assert!(e.len <= MAX_TOKENS);
            assert_eq!(e.cap_hit, battlefield > 100, "{battlefield}");
            let d = decode(&e);
            assert_eq!(d.zones[&(ZoneObs::Battlefield, 0)], battlefield as u32);
            assert_eq!(d.zones[&(ZoneObs::Battlefield, 1)], 1);
            assert_eq!(d.zones[&(ZoneObs::Library, 0)], 60);
            // Pooling the battlefield made pooling the library needless: its top is back.
            let library = (0..e.len)
                .filter(|&t| e.cats_of(t)[cat::ZONE] == 1 + ZoneObs::Library as u8)
                .count();
            assert_eq!(library, 4, "{battlefield}");
            let n = obs.objects.len() as u32;
            assert_eq!(
                d.options[0],
                (
                    Tok::Block,
                    Pointee::Object(n - 2),
                    Pointee::Object(n - 1),
                    0
                ),
                "{battlefield}"
            );
            assert!(same_tensors(&e, &from_compact(&to_compact(&e)).unwrap()));
        }
    }

    #[test]
    fn options_past_the_cap_are_cut() {
        let obs = crowded(3, 10);
        let tokens: Vec<Token> = (0..200)
            .map(|n| Token {
                kind: Tok::Pick,
                a: Ref::None,
                b: Ref::None,
                n,
            })
            .collect();
        let e = encode(&obs, &step(&tokens, &vec![true; 200]), &[], &[], &row);
        assert!(e.cap_hit);
        assert_eq!((e.bucket, e.len), (128, 128));
        // Room is kept for a token per place: the seat's library and battlefield, the opponent's battlefield, the stack.
        assert_eq!(e.options, 128 - 1 - obs.players.len() - 4);
    }

    #[test]
    fn full_mode_marks_what_would_be_hidden() {
        let mut obs = crowded(1, 1);
        let mut hand = bear(ZoneObs::Hand, 1, None);
        if let ObjectObs::Visible(v) = &mut hand {
            v.owner = 1;
        }
        obs.objects.insert(0, hand);
        obs.objects.insert(0, bear(ZoneObs::Hand, 0, None));
        let e = encode(&obs, &Step::default(), &[], &[], &row);
        let flags = |t: usize| {
            (
                e.nums_of(t)[object::WOULD_BE_HIDDEN],
                e.nums_of(t)[object::HIDDEN_FROM_OTHERS],
            )
        };
        assert_eq!(flags(3), (0.0, 1.0), "the seat's hand");
        assert_eq!(flags(4), (1.0, 0.0), "the opponent's hand");
        assert_eq!(flags(5), (1.0, 1.0), "the seat's library");
        assert_eq!(flags(6), (0.0, 0.0), "the battlefield");
        assert_eq!(e.nums_of(0)[game::MODE], 1.0);
    }

    /// `ai/encoding.json` is the layout the code has (`CANTRIP_GOLDEN=1` writes it).
    #[test]
    fn the_checked_in_layout_matches_the_code() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ai/encoding.json");
        if std::env::var_os("CANTRIP_GOLDEN").is_some() {
            std::fs::write(&path, layout_json()).unwrap();
        }
        let file = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            file == layout_json(),
            "ai/encoding.json is not the layout the code has: rerun with CANTRIP_GOLDEN=1"
        );
    }

    #[test]
    fn the_layout_hash_is_stable_for_the_layout() {
        assert_eq!(encoder_hash(), fnv1a64(layout_json().as_bytes()));
        assert!(layout_json().contains("\"schema\": 1"));
    }
}
