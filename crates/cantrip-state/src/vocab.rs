//! The game's vocabulary as enums and bitsets: nothing here is a string at runtime.

use std::fmt;

/// A player, by seat: 0 is the first seat (Bot1 in a scenario).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PlayerId(pub u8);

/// The most seats a game deals: every table the server and the tools deal seats eight at most (a queue's table, a draft
/// pod's game). A set of seats is a bit each in a `u8` (`Obj::seen_by`, `Chars::protection_players`), which this fills; a
/// deal of more is refused (the host's `Session::new`).
pub const MAX_SEATS: usize = 8;

impl PlayerId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// An object (CR 109). An object that changes zones becomes a new object with a new id (CR 400.7); the old row stays
/// behind, frozen, as its last-known information.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ObjectId(pub u32);

impl ObjectId {
    /// No object: the source of mana a player's own mana ability adds (CR 605.1a: such an ability has no source object).
    pub const NONE: ObjectId = ObjectId(u32::MAX);

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A card in the immutable card table, built once per process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CardId(pub u16);

/// A cast's way number (`ActionOption::index`) names which of the card's own options it is - the face it casts and the
/// cost of its own it is cast for - and never anything another object gives: 0 is the card itself for its mana cost, n
/// the card's nth printed alternative way (`CardDef::ways`), and the constants below its other faces and casts. What
/// another object adds (a permission, an alternative cost) is the action's `Grant`.
///
/// How a spell cast as its Adventure is cast (CR 715.3), as a way number: after every alternative cost.
pub const WAY_ADVENTURE: u8 = 254;

/// How a spell cast as its Omen is cast (CR 720.3), as a way number: beside the Adventure, after every alternative cost.
pub const WAY_OMEN: u8 = 249;

/// How a modal double-faced card is cast as its back face (CR 712.2, 712.8a), as a way number: beside the front face's
/// casts, after every alternative cost.
pub const WAY_BACK_FACE: u8 = 253;

/// How a card with bestow is cast bestowed (CR 702.103a), as a way number: beside the other casts, after every alternative
/// cost.
pub const WAY_BESTOW: u8 = 251;

/// How a card with overload is cast overloaded (CR 702.96a), as a way number: for its overload cost, with "each" where it
/// says "target", so with no targets at all.
pub const WAY_OVERLOAD: u8 = 250;

/// How the halves of a split card are cast (CR 709.3), as way numbers: the left half and the right.
pub const WAY_SPLIT_LEFT: u8 = 239;
pub const WAY_SPLIT_RIGHT: u8 = 238;

/// How a split card with fuse is cast as both halves at once from its owner's hand (CR 702.102a), as a way number: the
/// fused split spell, which is the whole card on the stack.
pub const WAY_FUSED: u8 = 237;

/// The doors of a Room (CR 709.5, 709.5j), as the bits of `Marks::doors`: the left half and the right.
pub const DOOR_LEFT: u8 = 1;
pub const DOOR_RIGHT: u8 = 2;

/// How a card is cast transformed (CR 712.11a): a Siege's "cast it transformed", as a way number.
pub const WAY_TRANSFORMED: u8 = 252;

/// How a prototype card is cast prototyped (CR 702.160a, 718.3), as a way number: as its prototyped face, for that face's mana
/// cost or an alternative cost (the ruling: casting it prototyped is no alternative cost).
pub const WAY_PROTOTYPE: u8 = 248;

/// What made an action possible beyond the rules every player has (CR 601.3, 305.1, 118.9, 113.6): the object, and
/// which of its abilities. A permanent's static ability (Lurrus of the Dream-Den's way from the graveyard, Bolas's Citadel's
/// and Mystic Forge's rules for the top of the library, Primal Prayers' alternative cost), the spell or ability whose effect
/// gave a permission (Snapcaster Mage's trigger, Yawgmoth's Will), or a card's own keyword that lets it be cast from where it
/// is (retrace). An object, not a card: two Lurruses are two permissions, and a permanent that changes zones is a new
/// object whose permission is a new one (CR 400.7). The static abilities a card lists apart from `abilities` are named by
/// the `AbilityRef` bands below (`AbilityRef::granted_way` and the rest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Grant {
    pub object: ObjectId,
    pub ability: AbilityRef,
}

impl Grant {
    /// No grant, where a record keeps one in eight bytes rather than an `Option`'s twelve (`Cast::by`,
    /// `Event::LandPlayed`): the history is the engine's largest buffer, and its records are read on every question.
    pub const NONE: Grant = Grant { object: ObjectId::NONE, ability: AbilityRef { card: CardId(0), index: 0 } };

    /// The grant, or none where this is `Grant::NONE`.
    pub fn some(self) -> Option<Grant> {
        (self.object != ObjectId::NONE).then_some(self)
    }

    /// `Grant::NONE` for none.
    pub fn of(grant: Option<Grant>) -> Grant {
        grant.unwrap_or(Grant::NONE)
    }
}

/// One ability of one card: the card's definition and the ability's index in it. A copy of a card has the copied
/// card's abilities, so a computed ability list is a list of these, never of the object's own card alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AbilityRef {
    pub card: CardId,
    pub index: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Blue = 1,
    Black = 2,
    Red = 3,
    Green = 4,
}

impl Color {
    pub const ALL: [Color; 5] = [Color::White, Color::Blue, Color::Black, Color::Red, Color::Green];

    pub fn name(self) -> &'static str {
        match self {
            Color::White => "White",
            Color::Blue => "Blue",
            Color::Black => "Black",
            Color::Red => "Red",
            Color::Green => "Green",
        }
    }

    pub fn symbol(self) -> char {
        match self {
            Color::White => 'W',
            Color::Blue => 'U',
            Color::Black => 'B',
            Color::Red => 'R',
            Color::Green => 'G',
        }
    }

    pub fn mana(self) -> Mana {
        match self {
            Color::White => Mana::W,
            Color::Blue => Mana::U,
            Color::Black => Mana::B,
            Color::Red => Mana::R,
            Color::Green => Mana::G,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorSet(pub u8);

impl ColorSet {
    pub const NONE: ColorSet = ColorSet(0);

    pub const fn of(color: Color) -> ColorSet {
        ColorSet(1 << color as u8)
    }

    pub fn has(self, color: Color) -> bool {
        self.0 & (1 << color as u8) != 0
    }

    pub fn with(self, color: Color) -> ColorSet {
        ColorSet(self.0 | (1 << color as u8))
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn intersects(self, other: ColorSet) -> bool {
        self.0 & other.0 != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Color> {
        Color::ALL.into_iter().filter(move |c| self.has(*c))
    }
}

impl fmt::Debug for ColorSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// A kind of mana (CR 106): the five colors and colorless.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Mana {
    W = 0,
    U = 1,
    B = 2,
    R = 3,
    G = 4,
    C = 5,
}

impl Mana {
    pub const ALL: [Mana; 6] = [Mana::W, Mana::U, Mana::B, Mana::R, Mana::G, Mana::C];

    pub fn color(self) -> Option<Color> {
        match self {
            Mana::W => Some(Color::White),
            Mana::U => Some(Color::Blue),
            Mana::B => Some(Color::Black),
            Mana::R => Some(Color::Red),
            Mana::G => Some(Color::Green),
            Mana::C => None,
        }
    }

    pub fn symbol(self) -> char {
        match self {
            Mana::W => 'W',
            Mana::U => 'U',
            Mana::B => 'B',
            Mana::R => 'R',
            Mana::G => 'G',
            Mana::C => 'C',
        }
    }
}

/// Mana in a pool, by kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ManaPool(pub [u32; 6]);

impl ManaPool {
    pub fn get(&self, mana: Mana) -> u32 {
        self.0[mana as usize]
    }

    pub fn total(&self) -> u32 {
        self.0.iter().sum()
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// Mana of each kind, a `u8` each: what one payment spent (`Event::ManaSpent`), what a spell's casting spent (`Obj::spent`)
/// and what one activation produced (`Event::TappedForMana`). A pool has no limit (`ManaPool`), but one payment spends at most
/// a cost's worth, and what an activation produced is read only for its kinds, so these stay narrow - the history holds
/// one of them per such event. Past 255 a kind saturates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ManaTally(pub [u8; 6]);

impl ManaTally {
    pub fn get(&self, mana: Mana) -> u8 {
        self.0[mana as usize]
    }

    pub fn total(&self) -> u32 {
        self.0.iter().map(|&n| n as u32).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&n| n == 0)
    }

    /// `n` more of `mana`, saturating at 255.
    pub fn add(&mut self, mana: Mana, n: u32) {
        let at = mana as usize;
        self.0[at] = (u32::from(self.0[at]) + n).min(u32::from(u8::MAX)) as u8;
    }

    /// A pool's counts, each saturating at 255.
    pub fn of(pool: ManaPool) -> ManaTally {
        ManaTally(pool.0.map(|n| n.min(u32::from(u8::MAX)) as u8))
    }
}

/// A mana cost (CR 202): generic, and a count of each colored symbol; hybrid, monocolored hybrid, Phyrexian and {X}
/// symbols. Snow ({S}) is not built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ManaCost {
    pub generic: u8,
    /// W U B R G C, as `Mana` orders them.
    pub symbols: [u8; 6],
    /// Hybrid symbols (CR 107.4e): each paid with either of two types of mana. Two colors ({W/U}), or a color and
    /// colorless ({C/W}, a monocolored hybrid symbol whose other half is {C}).
    pub hybrid: [Option<(Mana, Mana)>; 6],
    /// Monocolored hybrid symbols with a generic half (CR 107.4e), {2/W} of each color, W U B R G: each paid with one mana
    /// of that color or two generic mana, chosen as the cost is announced (CR 601.2b, 118.13a); each counts 2 toward the
    /// mana value (CR 202.3f) and is its color (CR 202.2d).
    pub twobrid: [u8; 5],
    /// Phyrexian symbols (CR 107.4f) of each color, W U B R G: each paid with that color or 2 life.
    pub phyrexian: [u8; 5],
    /// How many {X} symbols it has (CR 107.3): {X}{X} is 2.
    pub x: u8,
}

impl ManaCost {
    pub const ZERO: ManaCost = ManaCost { generic: 0, symbols: [0; 6], hybrid: [None; 6], twobrid: [0; 5], phyrexian: [0; 5], x: 0 };

    pub fn mana_value(&self) -> u32 {
        self.generic as u32
            + self.symbols.iter().map(|&n| n as u32).sum::<u32>()
            + self.hybrid.iter().flatten().count() as u32
            + 2 * self.twobrid.iter().map(|&n| n as u32).sum::<u32>()
            + self.phyrexian.iter().map(|&n| n as u32).sum::<u32>()
    }

    pub fn is_zero(&self) -> bool {
        self.mana_value() == 0
    }

    /// How many {2/W}-style symbols the cost has.
    pub fn twobrids(&self) -> u32 {
        self.twobrid.iter().map(|&n| n as u32).sum()
    }

    /// The nonhybrid equivalent of its monocolored hybrid symbols (CR 601.2b): the symbols `generic` lists (by their order
    /// in `twobrid`, W first) paid with {2}, the others with their color.
    pub fn with_twobrids_paid(&self, generic: &[bool]) -> ManaCost {
        let mut out = *self;
        let mut at = 0;
        for (i, &n) in self.twobrid.iter().enumerate() {
            for _ in 0..n {
                if generic.get(at).copied().unwrap_or(false) {
                    out.generic = out.generic.saturating_add(2);
                } else {
                    let colored = &mut out.symbols[Color::ALL[i].mana() as usize];
                    *colored = colored.saturating_add(1);
                }
                at += 1;
            }
        }
        out.twobrid = [0; 5];
        out
    }

    /// The cost as a player who "may spend mana as though it were mana of any color" pays it (CR 609.4b): each colored
    /// symbol and each hybrid symbol may be paid with any mana, colorless included (Mycosynth Lattice's ruling), so it is
    /// paid as generic, a {2/W} as one mana; a {C} symbol still takes colorless mana, which no color stands for, and a
    /// Phyrexian symbol keeps its 2-life choice (announced before the mana is paid, which turns it into its color's
    /// symbol, and that then any mana pays). The cost itself, and its mana value, are not changed (CR 609.4b): this is the
    /// payment's view of it only.
    pub fn paid_with_any_color(&self) -> ManaCost {
        let mut paid = *self;
        let mut generic = self.generic as u32;
        for i in 0..5 {
            generic += self.symbols[i] as u32;
            paid.symbols[i] = 0;
        }
        // A hybrid symbol has a colored half, which any mana may now pay.
        generic += paid.hybrid.iter().flatten().count() as u32 + self.twobrids();
        paid.hybrid = [None; 6];
        paid.twobrid = [0; 5];
        paid.generic = generic.min(255) as u8;
        paid
    }

    /// The cost as it is paid when "mana of any type can be spent" for it (CR 609.4b: Cruelclaw's Heist, Null Summoner):
    /// every symbol a type of mana is asked for by - colored, {C} (colorless is a type, CR 106.1b) and either half of a
    /// hybrid one - is paid by one mana of any type, so it is paid as generic, a {2/W} as one mana (its colored half). The
    /// rest of its structure stays: the generic mana, the {X} symbols and a Phyrexian symbol's choice of 2 life, which is
    /// announced before the mana is paid (CR 601.2b) and turns the symbol into its color's, which any mana then pays. The
    /// cost itself, and its mana value, are not changed: this is the payment's view of it only, so what pays only generic
    /// mana (delve, improvise) is still held to the cost's own generic part.
    pub fn paid_with_any_type(&self) -> ManaCost {
        let mut paid = *self;
        let symbols: u32 = self.symbols.iter().map(|&n| n as u32).sum();
        let hybrid = self.hybrid.iter().flatten().count() as u32 + self.twobrids();
        paid.symbols = [0; 6];
        paid.hybrid = [None; 6];
        paid.twobrid = [0; 5];
        paid.generic = (self.generic as u32 + symbols + hybrid).min(255) as u8;
        paid
    }

    /// The two costs added symbol by symbol (CR 709.4b: a split card's mana cost is its halves' combined; a Room's
    /// is that of its unlocked doors, CR 709.5).
    pub fn plus(&self, other: &ManaCost) -> ManaCost {
        let mut sum = *self;
        sum.generic = sum.generic.saturating_add(other.generic);
        sum.x = sum.x.saturating_add(other.x);
        for i in 0..6 {
            sum.symbols[i] = sum.symbols[i].saturating_add(other.symbols[i]);
        }
        for i in 0..5 {
            sum.phyrexian[i] = sum.phyrexian[i].saturating_add(other.phyrexian[i]);
            sum.twobrid[i] = sum.twobrid[i].saturating_add(other.twobrid[i]);
        }
        for pair in other.hybrid.iter().flatten() {
            if let Some(slot) = sum.hybrid.iter().position(|h| h.is_none()) {
                sum.hybrid[slot] = Some(*pair);
            }
        }
        sum
    }

    /// The cost paid `n` times over, each symbol `n` times (CR 603.12a: "you may pay {G} up to three times" is {G}{G} paid
    /// twice). `None` when the result does not fit the cost's shape: a symbol count past a byte, or more hybrid symbols
    /// than its slots hold. Never a smaller cost than `n` payments.
    pub fn times(&self, n: u16) -> Option<ManaCost> {
        let mul = |k: u8| u8::try_from(u32::from(k) * u32::from(n)).ok();
        let mut out = ManaCost { generic: mul(self.generic)?, x: mul(self.x)?, hybrid: [None; 6], ..*self };
        for i in 0..6 {
            out.symbols[i] = mul(self.symbols[i])?;
        }
        for i in 0..5 {
            out.phyrexian[i] = mul(self.phyrexian[i])?;
            out.twobrid[i] = mul(self.twobrid[i])?;
        }
        let mut slot = 0;
        for _ in 0..n {
            for pair in self.hybrid.iter().flatten() {
                *out.hybrid.get_mut(slot)? = Some(*pair);
                slot += 1;
            }
        }
        Some(out)
    }

    pub fn colors(&self) -> ColorSet {
        let mut set = ColorSet::NONE;
        for color in Color::ALL {
            if self.symbols[color.mana() as usize] > 0 {
                set = set.with(color);
            }
        }
        for (a, b) in self.hybrid.iter().flatten() {
            for m in [a, b] {
                if let Some(c) = m.color() {
                    set = set.with(c);
                }
            }
        }
        for i in 0..5 {
            if self.phyrexian[i] > 0 || self.twobrid[i] > 0 {
                set = set.with(Color::ALL[i]);
            }
        }
        set
    }
}

impl fmt::Display for ManaCost {
    /// As the C# engine writes a cost: `{2}{R}`, and `{0}` for nothing.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.x {
            f.write_str("{X}")?;
        }
        if self.generic > 0 || self.mana_value() == 0 && self.x == 0 {
            write!(f, "{{{}}}", self.generic)?;
        }
        for (a, b) in self.hybrid.iter().flatten() {
            write!(f, "{{{}/{}}}", a.symbol(), b.symbol())?;
        }
        for (i, &n) in self.twobrid.iter().enumerate() {
            for _ in 0..n {
                write!(f, "{{2/{}}}", Color::ALL[i].symbol())?;
            }
        }
        for (i, &n) in self.phyrexian.iter().enumerate() {
            for _ in 0..n {
                write!(f, "{{{}/P}}", Color::ALL[i].symbol())?;
            }
        }
        for mana in Mana::ALL {
            for _ in 0..self.symbols[mana as usize] {
                write!(f, "{{{}}}", mana.symbol())?;
            }
        }
        Ok(())
    }
}

/// Card types (CR 300), as a bitset.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TypeSet(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CardType {
    Artifact = 0,
    Creature = 1,
    Enchantment = 2,
    Instant = 3,
    Land = 4,
    Planeswalker = 5,
    Sorcery = 6,
    Battle = 7,
    Kindred = 8,
}

impl CardType {
    pub const ALL: [CardType; 9] = [
        CardType::Kindred,
        CardType::Artifact,
        CardType::Battle,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Instant,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Sorcery,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CardType::Artifact => "Artifact",
            CardType::Creature => "Creature",
            CardType::Enchantment => "Enchantment",
            CardType::Instant => "Instant",
            CardType::Land => "Land",
            CardType::Planeswalker => "Planeswalker",
            CardType::Sorcery => "Sorcery",
            CardType::Battle => "Battle",
            CardType::Kindred => "Kindred",
        }
    }
}

impl TypeSet {
    pub const NONE: TypeSet = TypeSet(0);

    pub const fn of(types: &[CardType]) -> TypeSet {
        let mut bits = 0u16;
        let mut i = 0;
        while i < types.len() {
            bits |= 1 << types[i] as u8;
            i += 1;
        }
        TypeSet(bits)
    }

    pub fn has(self, t: CardType) -> bool {
        self.0 & (1 << t as u8) != 0
    }

    pub fn with(self, t: CardType) -> TypeSet {
        TypeSet(self.0 | (1 << t as u8))
    }

    pub fn iter(self) -> impl Iterator<Item = CardType> {
        CardType::ALL.into_iter().filter(move |t| self.has(*t))
    }
}

impl fmt::Debug for TypeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// The qualities of "hexproof from [quality]" abilities (CR 702.11d): a permanent with one can't be the target of
/// [quality] spells its controller's opponents control, nor of abilities they control from [quality] sources. A bit
/// for each quality: the five colors (`ColorSet`'s bits), monocolored and multicolored (CR 105.2a-b), and card types
/// (`TypeSet`'s bits, from bit 8: "hexproof from artifacts", "from instants"). "Hexproof from [A] and from [B]" is two
/// of them (CR 702.11f), "hexproof from each color" one for each color (CR 702.11g), and the same one twice is
/// redundant (CR 702.11h).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct HexproofFrom(pub u32);

impl HexproofFrom {
    pub const NONE: HexproofFrom = HexproofFrom(0);
    /// "Hexproof from monocolored": sources that are exactly one color (CR 105.2a).
    pub const MONOCOLORED: HexproofFrom = HexproofFrom(1 << 5);
    /// "Hexproof from multicolored": sources that are two or more colors (CR 105.2b).
    pub const MULTICOLORED: HexproofFrom = HexproofFrom(1 << 6);
    /// "Hexproof from each color" (CR 702.11g).
    pub const EACH_COLOR: HexproofFrom = HexproofFrom(0x1f);
    const TYPES_SHIFT: u32 = 8;

    /// "Hexproof from [color]".
    pub const fn color(color: Color) -> HexproofFrom {
        HexproofFrom(1 << color as u8)
    }

    /// "Hexproof from [card type]s": sources of that card type.
    pub const fn card_type(t: CardType) -> HexproofFrom {
        HexproofFrom(1 << (t as u32 + Self::TYPES_SHIFT))
    }

    /// Both sets of qualities (CR 702.11f).
    pub const fn and(self, other: HexproofFrom) -> HexproofFrom {
        HexproofFrom(self.0 | other.0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The colors it names.
    pub fn colors(self) -> ColorSet {
        ColorSet((self.0 & 0x1f) as u8)
    }

    /// These qualities with the colors it names replaced by `colors`: a text-changing effect's new color word (CR 612.2).
    pub fn with_colors(self, colors: ColorSet) -> HexproofFrom {
        HexproofFrom((self.0 & !0x1f) | colors.0 as u32)
    }

    /// The card types it names.
    pub fn types(self) -> TypeSet {
        TypeSet((self.0 >> Self::TYPES_SHIFT) as u16)
    }

    pub fn has(self, quality: HexproofFrom) -> bool {
        self.0 & quality.0 == quality.0 && quality.0 != 0
    }

    /// Whether a source with these colors and card types has one of the qualities (CR 702.11d): one of the colors, one
    /// color only (monocolored) or two or more (multicolored), one of the card types. A colorless source is none of them.
    pub fn covers(self, colors: ColorSet, types: TypeSet) -> bool {
        let n = colors.0.count_ones();
        self.colors().intersects(colors)
            || (self.has(HexproofFrom::MONOCOLORED) && n == 1)
            || (self.has(HexproofFrom::MULTICOLORED) && n >= 2)
            || self.types().0 & types.0 != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Supertypes {
    pub basic: bool,
    pub legendary: bool,
    pub snow: bool,
}

pub use crate::subtypes::{SUBTYPE_WORDS, Subtype};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SubtypeSet(pub [u64; SUBTYPE_WORDS]);

impl SubtypeSet {
    pub const NONE: SubtypeSet = SubtypeSet([0; SUBTYPE_WORDS]);
    /// Every creature type (CR 205.3m): what Changeling makes an object (CR 702.73a), and what "all creature types"
    /// gives one (Mutavault).
    pub const CREATURE_TYPES: SubtypeSet = SubtypeSet::of(&Subtype::CREATURE_TYPES);
    /// Every land type (CR 205.3i): what an effect that sets a land's subtype replaces (CR 305.7).
    pub const LAND_TYPES: SubtypeSet = SubtypeSet::of(&[
        Subtype::Cave,
        Subtype::Desert,
        Subtype::Forest,
        Subtype::Gate,
        Subtype::Island,
        Subtype::Lair,
        Subtype::Locus,
        Subtype::Mine,
        Subtype::Mountain,
        Subtype::Plains,
        Subtype::Planet,
        Subtype::PowerPlant,
        Subtype::Sphere,
        Subtype::Swamp,
        Subtype::Tower,
        Subtype::Town,
        Subtype::Urzas,
    ]);

    pub const fn of(subtypes: &[Subtype]) -> SubtypeSet {
        let mut bits = [0u64; SUBTYPE_WORDS];
        let mut i = 0;
        while i < subtypes.len() {
            let n = subtypes[i] as usize;
            bits[n / 64] |= 1 << (n % 64);
            i += 1;
        }
        SubtypeSet(bits)
    }

    pub fn has(self, s: Subtype) -> bool {
        let n = s as usize;
        self.0[n / 64] & (1 << (n % 64)) != 0
    }

    pub fn with(self, s: Subtype) -> SubtypeSet {
        let mut bits = self.0;
        let n = s as usize;
        bits[n / 64] |= 1 << (n % 64);
        SubtypeSet(bits)
    }

    pub fn union(self, other: SubtypeSet) -> SubtypeSet {
        let mut bits = self.0;
        for (a, b) in bits.iter_mut().zip(other.0) {
            *a |= b;
        }
        SubtypeSet(bits)
    }

    pub fn is_empty(self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }

    /// These subtypes but none of `other`'s.
    pub fn minus(self, other: SubtypeSet) -> SubtypeSet {
        let mut bits = self.0;
        for (a, b) in bits.iter_mut().zip(other.0) {
            *a &= !b;
        }
        SubtypeSet(bits)
    }

    pub fn without(self, s: Subtype) -> SubtypeSet {
        let mut bits = self.0;
        let n = s as usize;
        bits[n / 64] &= !(1 << (n % 64));
        SubtypeSet(bits)
    }

    /// The subtypes in the set, in `Subtype` order: only the set bits are visited (`Subtype::ALL` is in that order).
    pub fn iter(self) -> SubtypeIter {
        SubtypeIter { bits: self.0, word: 0 }
    }
}

/// `SubtypeSet::iter`: the set bits, lowest first, word by word.
pub struct SubtypeIter {
    bits: [u64; SUBTYPE_WORDS],
    word: usize,
}

impl Iterator for SubtypeIter {
    type Item = Subtype;

    fn next(&mut self) -> Option<Subtype> {
        while self.word < SUBTYPE_WORDS {
            let bits = &mut self.bits[self.word];
            if *bits == 0 {
                self.word += 1;
                continue;
            }
            let n = self.word * 64 + bits.trailing_zeros() as usize;
            *bits &= *bits - 1;
            // Every set bit is a subtype's (the set is only built from subtypes), so `get` never misses.
            if let Some(&s) = Subtype::ALL.get(n) {
                return Some(s);
            }
        }
        None
    }
}

/// A word a text-changing effect can replace (CR 612.2): a color word, or a subtype used as a subtype - a basic land
/// type or a creature type. Mana symbols are not words, and a name is never changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Word {
    Color(Color),
    Subtype(Subtype),
}

/// The words an object's text has now, against the words printed on it (CR 612): the text-changing effects on it in
/// timestamp order, each replacing every instance of one word with another. Card code names a word in its rules text
/// through these (`Query::words`), so "a blue spell" asks `words.color(Color::Blue)` and reads red once that word was
/// changed to red.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Words {
    changes: [Option<(Word, Word)>; Words::MAX],
}

impl Words {
    /// More text changes than any object gets; past it, the rest are not kept.
    pub const MAX: usize = 4;
    pub const NONE: Words = Words { changes: [None; Words::MAX] };

    /// Whether the text is as printed.
    pub fn is_printed(&self) -> bool {
        self.changes[0].is_none()
    }

    /// The text with every instance of `from` replaced by `to` (CR 612.1), after the changes made so far.
    pub fn changed(mut self, from: Word, to: Word) -> Words {
        if let Some(slot) = self.changes.iter().position(|c| c.is_none()) {
            self.changes[slot] = Some((from, to));
        }
        self
    }

    /// Every change of `other` after these.
    pub fn then(self, other: Words) -> Words {
        other.changes.into_iter().flatten().fold(self, |w, (from, to)| w.changed(from, to))
    }

    /// The word printed as `printed`, as the text has it now. The changes apply in timestamp order (CR 613.7) unless one
    /// depends on another (CR 613.8a): a change of "Goblin" depends on a later change that makes a word "Goblin", as
    /// applying that one changes what the first does, so it waits for it (613.8b). For one word that is: the earliest change
    /// not yet applied whose word it is now, again and again, each change once. Two changes of the same word each take the
    /// other's word away, a loop applied in timestamp order: the earlier.
    pub fn word(&self, printed: Word) -> Word {
        let mut word = printed;
        let mut applied = [false; Words::MAX];
        while let Some(i) = (0..Words::MAX).find(|&i| !applied[i] && self.changes[i].is_some_and(|(from, _)| from == word)) {
            applied[i] = true;
            if let Some((_, to)) = self.changes[i] {
                word = to;
            }
        }
        word
    }

    /// The color word printed as `printed`, as the text has it now: "destroy target blue permanent" says `color(Blue)`.
    pub fn color(&self, printed: Color) -> Color {
        match self.word(Word::Color(printed)) {
            Word::Color(c) => c,
            Word::Subtype(_) => printed,
        }
    }

    /// Each color word of `printed`, as the text has it now: "a 1/1 white and black Inkling" made under changed text.
    pub fn colors(&self, printed: ColorSet) -> ColorSet {
        if self.is_printed() {
            return printed;
        }
        printed.iter().fold(ColorSet::NONE, |set, c| set.with(self.color(c)))
    }

    /// The land or creature type printed as `printed`, as the text has it now: "other Goblins get +1/+1" says
    /// `subtype(Goblin)`.
    pub fn subtype(&self, printed: Subtype) -> Subtype {
        match self.word(Word::Subtype(printed)) {
            Word::Subtype(s) => s,
            Word::Color(_) => printed,
        }
    }

    /// Each subtype of `printed`, as the text has it now: a type line, or a token's types.
    pub fn subtypes(&self, printed: SubtypeSet) -> SubtypeSet {
        if self.is_printed() {
            return printed;
        }
        printed.iter().fold(SubtypeSet::NONE, |set, s| set.with(self.subtype(s)))
    }
}

impl fmt::Debug for SubtypeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// Keyword abilities the core knows (CR 702), as a bitset. A keyword is here only when it changes a core procedure;
/// the rest are shared mechanics, built from the public API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Keyword {
    Flying = 0,
    Haste = 1,
    Vigilance = 2,
    Trample = 3,
    FirstStrike = 4,
    DoubleStrike = 5,
    Deathtouch = 6,
    Lifelink = 7,
    Reach = 8,
    Defender = 9,
    Flash = 10,
    Hexproof = 11,
    Indestructible = 12,
    Menace = 13,
    Infect = 14,
    Shadow = 15,
    Shroud = 16,
    /// "This spell can't be countered" (CR 101.2, 701.6a): a characteristic of the card.
    CantBeCountered = 17,
    /// "This creature can't block" (CR 509.1b).
    CantBlock = 18,
    /// "This creature can block only creatures with flying" (CR 509.1b).
    CanBlockOnlyCreaturesWithFlying = 19,
    /// "This permanent doesn't untap during your untap step" (CR 502.3).
    DoesNotUntap = 20,
    /// "This creature crews Vehicles as though its power were 2 greater": tapped to pay a crew cost (CR 702.122a-b), it
    /// counts towards the total power as though its power were 2 greater. Its saddle twin is
    /// `SaddlesAsThoughPowerTwoGreater`; a Pilot that "saddles Mounts and crews Vehicles" has both.
    CrewsAsThoughPowerTwoGreater = 21,
    /// "This creature can't be blocked" (CR 509.1b): printed, or granted as an ability ("has" or "gains", CR 113.1a), so losing
    /// its abilities takes it. A resolved "it can't be blocked this turn" is `EffectKind::CantBeBlocked` instead, and another
    /// permanent's "equipped creature can't be blocked" is that permanent's `CardDef::block_rules`: neither is an ability of
    /// the creature, so neither is here.
    CantBeBlocked = 22,
    /// "You can't spend mana to cast this spell" (CR 601.2h, 106.4): only what else pays a cost (convoke's taps,
    /// delve's exiles) can pay for it.
    CantSpendManaToCast = 23,
    /// "You may cast this card from your graveyard" (CR 113.6b, 601.3): for its ordinary cost.
    CastFromYourGraveyard = 24,
    /// Retrace (CR 702.81a): "you may cast this card from your graveyard by discarding a land card in addition to
    /// paying its other costs". Printed, or granted to cards in a graveyard by a static ability.
    Retrace = 25,
    /// Daybound (CR 702.145): it turns over with the day/night designation, and enters transformed at night.
    Daybound = 26,
    /// Nightbound (CR 702.145): the back face of a daybound card.
    Nightbound = 27,
    /// Split second (CR 702.61a): while a spell with it is on the stack, players can't cast spells or activate abilities
    /// that aren't mana abilities.
    SplitSecond = 28,

    /// "This permanent can't be attacked" (CR 508.1c): a planeswalker or battle that no creature may be declared
    /// attacking.
    CantBeAttacked = 29,
    /// "This creature can't be blocked by creatures with power 2 or less" (CR 509.1b): a blocker with no power can't
    /// block it either.
    CantBeBlockedByPowerTwoOrLess = 30,
    /// "This creature can't be blocked except by three or more creatures" (CR 509.1b).
    CantBeBlockedExceptByThreeOrMore = 31,
    /// Changeling (CR 702.73a): "This object is every creature type", a characteristic-defining ability that works in
    /// every zone (CR 604.3). The layers add the creature types (`cantrip-rules` `layers.rs`), so card code asks
    /// `has_subtype` as for any creature type. Printed on Firdoch Core.
    Changeling = 32,
    /// Decayed (CR 702.147a): "This creature can't block" and "When this creature attacks, sacrifice it at end of combat".
    /// A core keyword because a keyword counter gives it (CR 122.1b) in layer 6: the blocking rule reads it, and the
    /// layers give an object that has it once layer 6 is done the rules' triggered ability (`intrinsic::DECAYED`).
    Decayed = 33,
    /// "This creature can't be blocked by more than one creature" (CR 509.1b): an evasion ability, a restriction on the
    /// declaration of blockers. Printed (Stalking Tiger, Charging Rhino, Ironhoof Ox) or given (Meltstrider's Resolve,
    /// Familiar Ground, Challenger Troll, Vorrac Battlehorns).
    CantBeBlockedByMoreThanOne = 34,
    /// Islandwalk (CR 702.14): landwalk naming the land type Island, an evasion ability - "this creature can't be blocked as
    /// long as defending player controls an Island" (CR 702.14c). Printed or given (Lord of Atlantis's "other Merfolk ...
    /// have islandwalk"). Each basic landwalk is a keyword of its own that the blocking rule reads with its land type
    /// (`Keyword::LANDWALKS`), so a text change of the land type gives the landwalk it now names.
    Islandwalk = 35,
    /// Plainswalk (CR 702.14), as Islandwalk.
    Plainswalk = 36,
    /// Swampwalk (CR 702.14), as Islandwalk.
    Swampwalk = 37,
    /// Mountainwalk (CR 702.14), as Islandwalk.
    Mountainwalk = 38,
    /// Forestwalk (CR 702.14), as Islandwalk.
    Forestwalk = 39,
    /// Banding (CR 702.22): it may attack in a band with other attackers (CR 702.22c), and its controller divides the combat
    /// damage of the creatures blocking it or blocked by it (CR 702.22j-k). A static ability that modifies the rules for
    /// combat, which the core plays: Benalish Hero, Mesa Pegasus.
    Banding = 40,
    /// "This creature saddles Mounts as though its power were 2 greater": tapped to pay a saddle cost (CR 702.171a,c), it
    /// counts towards the total power as though its power were 2 greater. Printed only beside crew's, "saddles Mounts and
    /// crews Vehicles as though its power were 2 greater" (Defend the Rider's Pilot, Deathless Pilot): both keywords.
    SaddlesAsThoughPowerTwoGreater = 41,
    /// "This permanent can't be sacrificed" (CR 101.2, 701.21a): no spell, ability or rule can have its controller sacrifice
    /// it - an instruction to sacrifice it does nothing, a choice of what to sacrifice can't take it, and it pays no cost
    /// that sacrifices (CR 118.3). Printed (Hithlain Rope, Alexios, Deimos of Kosmos) or granted, an ability that "loses all
    /// abilities" takes (Jon Irenicus's "it gains 'This creature can't be sacrificed'"). Where it is no ability of the
    /// permanent - "equipped creature can't be sacrificed", "it can't be sacrificed this turn" - it is
    /// `CardDef::sacrifice_bans` or `EffectKind::CantBeSacrificed`; the core reads all three (`Game::can_sacrifice`).
    CantBeSacrificed = 42,
    /// Morph (CR 702.37a), megamorph's included (CR 702.37b): a static ability that functions in every zone. A card has it by
    /// the morph or megamorph way it prints (`WayKind::Morph`, `Megamorph`), which also holds its cost; as a keyword it goes
    /// through the layers as any ability does, so "loses all abilities" takes it, and a face-down permanent whose
    /// characteristics would not have it if it were face up can't be turned face up for its morph cost (CR 702.37e, 708.8).
    Morph = 43,
    /// Disguise (CR 702.168a), as morph: a card has it by its disguise way, and a face-down permanent that would not have it
    /// face up can't be turned face up for its disguise cost (CR 702.168d).
    Disguise = 44,
    /// "This spell can't be copied" (CR 101.2, 707.10): a characteristic of the spell, so no effect copies it. Printed on Choreographed Sparks.
    CantBeCopied = 45,
}

impl Keyword {
    /// Each basic landwalk keyword (CR 702.14a) and the basic land type it names.
    pub const LANDWALKS: [(Keyword, Subtype); 5] = [
        (Keyword::Plainswalk, Subtype::Plains),
        (Keyword::Islandwalk, Subtype::Island),
        (Keyword::Swampwalk, Subtype::Swamp),
        (Keyword::Mountainwalk, Subtype::Mountain),
        (Keyword::Forestwalk, Subtype::Forest),
    ];

    /// The landwalk naming `land_type` (CR 702.14a), if it is a basic land type: what "islandwalk" is once a text-changing
    /// effect has replaced Island with another basic land type (CR 612.2).
    pub fn landwalk(land_type: Subtype) -> Option<Keyword> {
        Keyword::LANDWALKS.iter().find(|&&(_, t)| t == land_type).map(|&(k, _)| k)
    }
}

/// Day and night (CR 731.1): the designation the game itself has, belonging to no player. A game starts with neither
/// and, once it has become day or night, has exactly one of them from then on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum DayNight {
    #[default]
    Neither = 0,
    Day = 1,
    Night = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Keywords(pub u64);

impl Keywords {
    pub const NONE: Keywords = Keywords(0);

    pub const fn of(keywords: &[Keyword]) -> Keywords {
        let mut bits = 0u64;
        let mut i = 0;
        while i < keywords.len() {
            bits |= 1u64 << keywords[i] as u8;
            i += 1;
        }
        Keywords(bits)
    }

    pub fn has(self, k: Keyword) -> bool {
        self.0 & (1u64 << k as u8) != 0
    }

    pub fn with(self, k: Keyword) -> Keywords {
        Keywords(self.0 | (1u64 << k as u8))
    }
}

impl fmt::Debug for Keywords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Keywords({:#x})", self.0)
    }
}

/// Counters an object can have (CR 122), by kind: one line each - the kind, its index in an object's dense counter array,
/// its name as Oracle text writes it (CR 122.1), and the token the scenario dump names it by (`Counters.PlusOnePlusOne`).
/// A kind a new card prints is one more line at the end: the host's view and events, proliferate, "remove all counters"
/// and the client's pills all walk `CounterKind::ALL` and name the kind, so nothing else changes.
macro_rules! counter_kinds {
    ($($kind:ident = $n:literal, $name:literal, $token:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum CounterKind {
            $($kind = $n,)*
        }

        impl CounterKind {
            pub const ALL: [CounterKind; [$($n),*].len()] = [$(CounterKind::$kind),*];
            pub const COUNT: usize = Self::ALL.len();

            /// The counter's name as Oracle text writes it (CR 122.1).
            pub fn name(self) -> &'static str {
                match self {
                    $(CounterKind::$kind => $name,)*
                }
            }

            /// As the scenario dump names it: `Counters.PlusOnePlusOne`.
            pub fn token(self) -> &'static str {
                match self {
                    $(CounterKind::$kind => $token,)*
                }
            }
        }
    };
}

counter_kinds! {
    PlusOne = 0, "+1/+1", "PlusOnePlusOne";
    MinusOne = 1, "-1/-1", "MinusOneMinusOne";
    Loyalty = 2, "loyalty", "Loyalty";
    Time = 3, "time", "Time";
    Lore = 4, "lore", "Lore";
    Stun = 5, "stun", "Stun";
    Charge = 6, "charge", "Charge";
    Defense = 7, "defense", "Defense";
    Finality = 8, "finality", "Finality";
    Flying = 9, "flying", "Flying";
    Lifelink = 10, "lifelink", "Lifelink";
    Indestructible = 11, "indestructible", "Indestructible";
    Hexproof = 12, "hexproof", "Hexproof";
    Corpse = 13, "corpse", "Corpse";
    Fade = 14, "fade", "Fade";
    Level = 15, "level", "Level";
    Story = 16, "story", "Story";
    Wish = 17, "wish", "Wish";
    Rev = 18, "rev", "Rev";
    Mining = 19, "mining", "Mining";
    Influence = 20, "influence", "Influence";
    Oil = 21, "oil", "Oil";
    Shield = 22, "shield", "Shield";
    Fellowship = 23, "fellowship", "Fellowship";
    Hope = 24, "hope", "Hope";
    Void = 25, "void", "Void";
    Luck = 26, "luck", "Luck";
    Silver = 27, "silver", "Silver";
    Chorus = 28, "chorus", "Chorus";
    Burden = 29, "burden", "Burden";
    Incubation = 30, "incubation", "Incubation";
    Plan = 31, "plan", "Plan";
    // CR 122.1j: each one on an Equipment gives the creature it is attached to +1/+0 (the layers, 613.4c).
    Hone = 32, "hone", "Hone";
    Quest = 33, "quest", "Quest";
    // The rest of the keyword counters of CR 122.1b (flying, lifelink, indestructible and hexproof are above).
    FirstStrike = 34, "first strike", "FirstStrike";
    DoubleStrike = 35, "double strike", "DoubleStrike";
    Deathtouch = 36, "deathtouch", "Deathtouch";
    Decayed = 37, "decayed", "Decayed";
    Haste = 38, "haste", "Haste";
    Menace = 39, "menace", "Menace";
    Reach = 40, "reach", "Reach";
    Shadow = 41, "shadow", "Shadow";
    Trample = 42, "trample", "Trample";
    Vigilance = 43, "vigilance", "Vigilance";
    Page = 44, "page", "Page";
    Flood = 45, "flood", "Flood";
    Ingenuity = 46, "ingenuity", "Ingenuity";
    Net = 47, "net", "Net";
    Blight = 48, "blight", "Blight";
    Ice = 49, "ice", "Ice";
    Soul = 50, "soul", "Soul";
    Invitation = 51, "invitation", "Invitation";
    Growth = 52, "growth", "Growth";
}

impl CounterKind {
    /// The keyword counters (CR 122.1b): "flying, first strike, double strike, deathtouch, decayed, exalted, haste, hexproof,
    /// indestructible, lifelink, menace, reach, shadow, trample, and vigilance". Exalted is not here: it is a triggered
    /// ability the core does not give (`triggers::EXALTED` is printed on the cards that have it), and no card in content puts
    /// an exalted counter.
    pub const KEYWORDS: [CounterKind; 14] = [
        CounterKind::Flying,
        CounterKind::FirstStrike,
        CounterKind::DoubleStrike,
        CounterKind::Deathtouch,
        CounterKind::Decayed,
        CounterKind::Haste,
        CounterKind::Hexproof,
        CounterKind::Indestructible,
        CounterKind::Lifelink,
        CounterKind::Menace,
        CounterKind::Reach,
        CounterKind::Shadow,
        CounterKind::Trample,
        CounterKind::Vigilance,
    ];

    /// The keyword a keyword counter grants (CR 122.1b).
    pub fn keyword(self) -> Option<Keyword> {
        match self {
            CounterKind::Flying => Some(Keyword::Flying),
            CounterKind::FirstStrike => Some(Keyword::FirstStrike),
            CounterKind::DoubleStrike => Some(Keyword::DoubleStrike),
            CounterKind::Deathtouch => Some(Keyword::Deathtouch),
            CounterKind::Decayed => Some(Keyword::Decayed),
            CounterKind::Haste => Some(Keyword::Haste),
            CounterKind::Hexproof => Some(Keyword::Hexproof),
            CounterKind::Indestructible => Some(Keyword::Indestructible),
            CounterKind::Lifelink => Some(Keyword::Lifelink),
            CounterKind::Menace => Some(Keyword::Menace),
            CounterKind::Reach => Some(Keyword::Reach),
            CounterKind::Shadow => Some(Keyword::Shadow),
            CounterKind::Trample => Some(Keyword::Trample),
            CounterKind::Vigilance => Some(Keyword::Vigilance),
            _ => None,
        }
    }
}

/// The steps of a turn (CR 500), in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Step {
    Untap = 0,
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

impl Step {
    pub fn next(self) -> Option<Step> {
        use Step::*;
        Some(match self {
            Untap => Upkeep,
            Upkeep => Draw,
            Draw => PrecombatMain,
            PrecombatMain => BeginCombat,
            BeginCombat => DeclareAttackers,
            DeclareAttackers => DeclareBlockers,
            DeclareBlockers => CombatDamage,
            CombatDamage => EndOfCombat,
            EndOfCombat => PostcombatMain,
            PostcombatMain => End,
            End => Cleanup,
            Cleanup => return None,
        })
    }

    pub fn is_main(self) -> bool {
        matches!(self, Step::PrecombatMain | Step::PostcombatMain)
    }

    /// The C# engine's name for the step, which scenario waits and dumps use.
    pub fn name(self) -> &'static str {
        use Step::*;
        match self {
            Untap => "Untap",
            Upkeep => "Upkeep",
            Draw => "Draw",
            PrecombatMain => "PrecombatMain",
            BeginCombat => "BeginCombat",
            DeclareAttackers => "DeclareAttackers",
            DeclareBlockers => "DeclareBlockers",
            CombatDamage => "CombatDamage",
            EndOfCombat => "EndOfCombat",
            PostcombatMain => "PostcombatMain",
            End => "End",
            Cleanup => "Cleanup",
        }
    }
}

/// The kinds of zone (CR 400). Hand, library and graveyard are each player's own; the battlefield, the stack and
/// exile are shared, though exile is kept by owner for reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ZoneKind {
    Library = 0,
    Hand = 1,
    Graveyard = 2,
    Exile = 3,
    Battlefield = 4,
    Stack = 5,
    /// Not a zone: a token that has left the battlefield has ceased to exist (CR 111.7), and an object that has
    /// changed zones is a new object; the old row is kept here as last-known information.
    Gone = 6,
    /// The command zone (CR 114, 408): where emblems are, and nothing leaves it.
    Command = 7,
    /// Not a zone: the cards a player owns outside the game (CR 400.11), which in a match are their sideboard (CR 100.4,
    /// 400.11a, 108.3b). Nothing affects them but what brings them into the game (CR 400.11c), and only their owner may
    /// look at them.
    Sideboard = 8,
    /// Not a zone: the battlefield's phased-out permanents (CR 110.5, 702.26b), kept apart from the battlefield's list because
    /// a phased-out permanent is treated as though it does not exist. Phasing moves no object between zones (CR 702.26d): a
    /// permanent here is the same object it was, with its counters, status and attachments, and a token here goes on
    /// existing.
    PhasedOut = 9,
}

impl ZoneKind {
    pub fn is_hidden(self) -> bool {
        matches!(self, ZoneKind::Library | ZoneKind::Hand | ZoneKind::Sideboard)
    }

    pub fn name(self) -> &'static str {
        match self {
            ZoneKind::Library => "Library",
            ZoneKind::Hand => "Hand",
            ZoneKind::Graveyard => "Graveyard",
            ZoneKind::Exile => "Exile",
            ZoneKind::Battlefield => "Battlefield",
            ZoneKind::Stack => "Stack",
            ZoneKind::Gone => "Gone",
            ZoneKind::Command => "Command",
            ZoneKind::Sideboard => "Sideboard",
            ZoneKind::PhasedOut => "PhasedOut",
        }
    }
}

/// A set of zone kinds (CR 400.1): the zones a rule names, as "can't cast spells from graveyards or libraries" (CR 101.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ZoneKinds(pub u16);

impl ZoneKinds {
    pub const NONE: ZoneKinds = ZoneKinds(0);
    /// Every zone: "can't cast spells" wherever they are.
    pub const ALL: ZoneKinds = ZoneKinds(u16::MAX);
    /// The zones a spell can be cast from (CR 601.2a): every zone but the battlefield and the stack.
    pub const CAST_FROM: [ZoneKind; 5] = [ZoneKind::Hand, ZoneKind::Graveyard, ZoneKind::Exile, ZoneKind::Library, ZoneKind::Command];

    pub const fn of(kinds: &[ZoneKind]) -> ZoneKinds {
        let mut bits = 0u16;
        let mut i = 0;
        while i < kinds.len() {
            bits |= 1 << kinds[i] as u16;
            i += 1;
        }
        ZoneKinds(bits)
    }

    /// Every zone but these: "from anywhere other than their hands".
    pub const fn all_but(kinds: &[ZoneKind]) -> ZoneKinds {
        ZoneKinds(!ZoneKinds::of(kinds).0)
    }

    pub const fn contains(self, kind: ZoneKind) -> bool {
        self.0 & (1 << kind as u16) != 0
    }

    pub const fn union(self, other: ZoneKinds) -> ZoneKinds {
        ZoneKinds(self.0 | other.0)
    }

    pub const fn with(self, kind: ZoneKind) -> ZoneKinds {
        ZoneKinds(self.0 | (1 << kind as u16))
    }
}

/// A zone: its kind and, for the zones a player has, whose. The battlefield and the stack belong to nobody; their
/// `player` is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Zone {
    pub kind_raw: u8,
    pub player: PlayerId,
}

impl Zone {
    pub const fn new(kind: ZoneKind, player: PlayerId) -> Zone {
        Zone { kind_raw: kind as u8, player }
    }

    pub const BATTLEFIELD: Zone = Zone::new(ZoneKind::Battlefield, PlayerId(0));
    pub const STACK: Zone = Zone::new(ZoneKind::Stack, PlayerId(0));
    pub const GONE: Zone = Zone::new(ZoneKind::Gone, PlayerId(0));
    /// The command zone, shared: an emblem in it is its owner's by its `owner`.
    pub const COMMAND: Zone = Zone::new(ZoneKind::Command, PlayerId(0));
    /// The phased-out permanents, shared as the battlefield is (CR 702.26b): each is its controller's by its `controller`.
    pub const PHASED_OUT: Zone = Zone::new(ZoneKind::PhasedOut, PlayerId(0));

    pub fn library(p: PlayerId) -> Zone {
        Zone::new(ZoneKind::Library, p)
    }
    pub fn hand(p: PlayerId) -> Zone {
        Zone::new(ZoneKind::Hand, p)
    }
    pub fn graveyard(p: PlayerId) -> Zone {
        Zone::new(ZoneKind::Graveyard, p)
    }
    pub fn exile(p: PlayerId) -> Zone {
        Zone::new(ZoneKind::Exile, p)
    }
    /// The cards `p` owns outside the game (CR 400.11a): not a zone of the game, though the state keeps them as one.
    pub fn sideboard(p: PlayerId) -> Zone {
        Zone::new(ZoneKind::Sideboard, p)
    }

    pub fn kind(self) -> ZoneKind {
        match self.kind_raw {
            0 => ZoneKind::Library,
            1 => ZoneKind::Hand,
            2 => ZoneKind::Graveyard,
            3 => ZoneKind::Exile,
            4 => ZoneKind::Battlefield,
            5 => ZoneKind::Stack,
            7 => ZoneKind::Command,
            8 => ZoneKind::Sideboard,
            9 => ZoneKind::PhasedOut,
            _ => ZoneKind::Gone,
        }
    }

    pub fn is(self, kind: ZoneKind) -> bool {
        self.kind_raw == kind as u8
    }

    /// The zone's slot in the state's zone lists.
    pub fn slot(self, players: usize) -> usize {
        match self.kind() {
            ZoneKind::Battlefield => 4 * players,
            ZoneKind::Stack => 4 * players + 1,
            ZoneKind::Gone => 4 * players + 2,
            ZoneKind::Command => 4 * players + 3,
            ZoneKind::Sideboard => 4 * players + 4 + self.player.index(),
            ZoneKind::PhasedOut => 5 * players + 4,
            k => (k as usize) * players + self.player.index(),
        }
    }

    pub fn slots(players: usize) -> usize {
        5 * players + 5
    }
}

/// Why an object moved between zones, which triggers and replacements read (CR 700.4 "dies", 701.9 discard...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoneReason {
    Other,
    Draw,
    Discard,
    Mill,
    Sacrifice,
    Destroy,
    /// Destruction that "can't be regenerated" (CR 701.19c): regeneration shields do not replace it.
    DestroyNoRegen,
    PlayLand,
    Cast,
    Resolve,
    Counter,
    StateBased,
    Exile,
    Created,
    Search,
}

impl AbilityRef {
    /// Delayed abilities are numbered after a card's printed ones.
    pub const DELAYED: u8 = 128;
    /// Madness's linked triggered ability (CR 702.35a): the core's own, not an entry of any card's list.
    pub const MADNESS: u8 = 127;
    /// Rebound's delayed triggered ability (CR 702.88a): the core's own.
    pub const REBOUND: u8 = 126;
    /// Miracle's linked triggered ability (CR 702.94a): the core's own.
    pub const MIRACLE: u8 = 125;
    /// A card's retrace (CR 702.81a), printed or given, as the ability a cast from the graveyard by it is made by (`Grant`).
    pub const RETRACE: u8 = 124;
    /// The bands a `Grant` names a card's listed static abilities by: the kth of its `granted_ways`, `exile_casts`,
    /// `spell_alternatives`, and the kth play rule its `play_rules` hold (counted through every `PlayRule::While`), each
    /// past any ability a card prints.
    pub const GRANTED_WAY: u8 = 64;
    pub const EXILE_CAST: u8 = 72;
    pub const SPELL_ALTERNATIVE: u8 = 80;
    pub const PLAY_RULE: u8 = 88;
    /// The kth of a card's `ability_alternatives`: a cost it lets a player pay rather than an activated ability's (CR 118.9).
    pub const ABILITY_ALTERNATIVE: u8 = 104;
    const BAND: u8 = 8;
    const PLAY_RULES: u8 = 16;
    /// The history's marker of extra loyalty activations allowed this turn (`actions::EXTRA_LOYALTY_ALLOWED`).
    pub const EXTRA_LOYALTY_ALLOWED: u8 = 123;
    /// A spell itself, in the history's record that it resolved (`Event::Resolved`, CR 608.2): no ability of the card.
    pub const SPELL: u8 = 122;
    /// An enchant ability (CR 702.5a) an object has, as one of its abilities (CR 613.1f): the card's printed `enchant`.
    pub const ENCHANT: u8 = 121;
    /// An enchant ability an effect gave an object (`EffectKind::Enchant`): the `becomes_aura_enchant` of the card whose
    /// effect it is, never its printed `enchant`.
    pub const ENCHANT_GIVEN: u8 = 120;

    pub fn granted_way(card: CardId, k: usize) -> AbilityRef {
        debug_assert!(k < Self::BAND as usize);
        AbilityRef { card, index: Self::GRANTED_WAY + k as u8 }
    }

    pub fn exile_cast(card: CardId, k: usize) -> AbilityRef {
        debug_assert!(k < Self::BAND as usize);
        AbilityRef { card, index: Self::EXILE_CAST + k as u8 }
    }

    pub fn spell_alternative(card: CardId, k: usize) -> AbilityRef {
        debug_assert!(k < Self::BAND as usize);
        AbilityRef { card, index: Self::SPELL_ALTERNATIVE + k as u8 }
    }

    pub fn ability_alternative(card: CardId, k: usize) -> AbilityRef {
        debug_assert!(k < Self::BAND as usize);
        AbilityRef { card, index: Self::ABILITY_ALTERNATIVE + k as u8 }
    }

    pub fn as_ability_alternative(self) -> Option<usize> {
        self.band(Self::ABILITY_ALTERNATIVE, Self::BAND)
    }

    pub fn play_rule(card: CardId, k: usize) -> AbilityRef {
        debug_assert!(k < Self::PLAY_RULES as usize);
        AbilityRef { card, index: Self::PLAY_RULE + k as u8 }
    }

    pub fn retrace(card: CardId) -> AbilityRef {
        AbilityRef { card, index: Self::RETRACE }
    }

    fn band(self, start: u8, len: u8) -> Option<usize> {
        self.index.checked_sub(start).filter(|&k| k < len).map(usize::from)
    }

    /// Which of its card's `granted_ways` this names, if it names one.
    pub fn as_granted_way(self) -> Option<usize> {
        self.band(Self::GRANTED_WAY, Self::BAND)
    }

    pub fn as_exile_cast(self) -> Option<usize> {
        self.band(Self::EXILE_CAST, Self::BAND)
    }

    pub fn as_spell_alternative(self) -> Option<usize> {
        self.band(Self::SPELL_ALTERNATIVE, Self::BAND)
    }

    pub fn as_play_rule(self) -> Option<usize> {
        self.band(Self::PLAY_RULE, Self::PLAY_RULES)
    }

    pub fn is_retrace(self) -> bool {
        self.index == Self::RETRACE
    }

    pub fn spell(card: CardId) -> AbilityRef {
        AbilityRef { card, index: AbilityRef::SPELL }
    }

    pub fn is_spell(self) -> bool {
        self.index == AbilityRef::SPELL
    }

    /// The enchant ability `card` prints (`ENCHANT`).
    pub fn enchant(card: CardId) -> AbilityRef {
        AbilityRef { card, index: AbilityRef::ENCHANT }
    }

    /// The enchant ability `card`'s effects give (`ENCHANT_GIVEN`).
    pub fn enchant_given(card: CardId) -> AbilityRef {
        AbilityRef { card, index: AbilityRef::ENCHANT_GIVEN }
    }

    /// An enchant ability, printed or given.
    pub fn is_enchant(self) -> bool {
        self.index == AbilityRef::ENCHANT || self.index == AbilityRef::ENCHANT_GIVEN
    }

    pub fn miracle(card: CardId) -> AbilityRef {
        AbilityRef { card, index: AbilityRef::MIRACLE }
    }

    pub fn is_miracle(self) -> bool {
        self.index == AbilityRef::MIRACLE
    }

    pub fn madness(card: CardId) -> AbilityRef {
        AbilityRef { card, index: AbilityRef::MADNESS }
    }

    pub fn rebound(card: CardId) -> AbilityRef {
        AbilityRef { card, index: AbilityRef::REBOUND }
    }

    pub fn is_rebound(self) -> bool {
        self.index == AbilityRef::REBOUND
    }

    pub fn is_madness(self) -> bool {
        self.index == AbilityRef::MADNESS
    }

    pub fn delayed(card: CardId, index: u8) -> AbilityRef {
        AbilityRef { card, index: AbilityRef::DELAYED + index }
    }

    pub fn is_delayed(self) -> bool {
        self.index >= AbilityRef::DELAYED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cost paid `n` times over (CR 603.12a): each symbol `n` times, none past what the cost's shape holds.
    #[test]
    fn a_mana_cost_times_n_holds_each_symbol_n_times_or_nothing() {
        let mut one = ManaCost { generic: 1, ..ManaCost::ZERO };
        one.symbols[Mana::G as usize] = 1;
        one.hybrid[0] = Some((Mana::W, Mana::U));
        let three = one.times(3).expect("fits");
        assert_eq!((three.generic, three.symbols[Mana::G as usize], three.hybrid.iter().flatten().count()), (3, 3, 3));
        assert_eq!(three.mana_value(), 9);
        assert_eq!(one.times(0).expect("fits").mana_value(), 0);
        assert!(one.times(7).is_none(), "seven hybrid symbols don't fit six slots");
        let g = ManaCost { symbols: [0, 0, 0, 0, 1, 0], ..ManaCost::ZERO };
        assert_eq!(g.times(255).map(|c| c.symbols[Mana::G as usize]), Some(255));
        assert!(g.times(256).is_none(), "past a byte");
    }

    /// CR 613.7, 613.8a(b): text changes chain whichever order they came in; a loop of two changes of one word applies the
    /// earlier.
    #[test]
    fn text_changes_of_a_word_apply_by_dependency() {
        let (elf, goblin, dragon) = (Word::Subtype(Subtype::Elf), Word::Subtype(Subtype::Goblin), Word::Subtype(Subtype::Dragon));
        assert_eq!(Words::NONE.changed(elf, goblin).changed(goblin, dragon).word(elf), dragon);
        assert_eq!(Words::NONE.changed(goblin, dragon).changed(elf, goblin).word(elf), dragon);
        assert_eq!(Words::NONE.changed(elf, goblin).changed(elf, dragon).word(elf), goblin);
        assert_eq!(Words::NONE.changed(elf, goblin).changed(goblin, elf).word(elf), elf);
    }

    /// CR 609.4b: paid as though mana were of any color, {1}{R}{R/G}{C}{X} is paid as {3}{C}{X} - the colored and hybrid
    /// symbols by any mana, the {C} by colorless only - and its mana value is unchanged.
    #[test]
    fn a_cost_paid_with_any_color_keeps_its_colorless_symbol() {
        let mut cost = ManaCost { generic: 1, x: 1, ..ManaCost::ZERO };
        cost.symbols[Mana::R as usize] = 1;
        cost.symbols[Mana::C as usize] = 1;
        cost.hybrid[0] = Some((Mana::R, Mana::G));
        let paid = cost.paid_with_any_color();
        assert_eq!(paid.generic, 3);
        assert_eq!(paid.symbols, [0, 0, 0, 0, 0, 1]);
        assert!(paid.hybrid.iter().all(Option::is_none));
        assert_eq!(paid.x, 1);
        assert_eq!(paid.mana_value(), cost.mana_value());
    }

    /// CR 609.4b: paid with mana of any type, {1}{R}{R/G}{C}{X}{B/P} is paid as {4}{X}{B/P} - the colored, hybrid and {C}
    /// symbols by any mana - keeping the {X} and the Phyrexian symbol's 2-life choice, and its mana value unchanged.
    #[test]
    fn a_cost_paid_with_any_type_keeps_its_x_and_phyrexian_symbols() {
        let mut cost = ManaCost { generic: 1, x: 1, ..ManaCost::ZERO };
        cost.symbols[Mana::R as usize] = 1;
        cost.symbols[Mana::C as usize] = 1;
        cost.hybrid[0] = Some((Mana::R, Mana::G));
        cost.phyrexian[Color::Black as usize] = 1;
        let paid = cost.paid_with_any_type();
        assert_eq!(paid.generic, 4);
        assert_eq!(paid.symbols, [0; 6]);
        assert!(paid.hybrid.iter().all(Option::is_none));
        assert_eq!((paid.x, paid.phyrexian), (1, cost.phyrexian));
        assert_eq!(paid.mana_value(), cost.mana_value());
    }

    /// CR 107.4e, 202.3f, 202.2d: {2/W}{2/B}{2/G} (Kin-Tree Severance) has mana value 6 and is white, black and green; {C/W}
    /// is a hybrid symbol of colorless and white, mana value 1. Announced (CR 601.2b) with the {2/B} paid as {2}, it is
    /// {2}{W}{G}; paid with mana of any type, each {2/W} is one mana.
    #[test]
    fn a_monocolored_hybrid_symbol_counts_two_and_is_its_color() {
        let mut cost = ManaCost::ZERO;
        cost.twobrid[Color::White as usize] = 1;
        cost.twobrid[Color::Black as usize] = 1;
        cost.twobrid[Color::Green as usize] = 1;
        assert_eq!(cost.mana_value(), 6);
        assert_eq!(cost.colors(), ColorSet::NONE.with(Color::White).with(Color::Black).with(Color::Green));
        assert_eq!(cost.to_string(), "{2/W}{2/B}{2/G}");
        let mut paid = ManaCost { generic: 2, ..ManaCost::ZERO };
        paid.symbols[Mana::W as usize] = 1;
        paid.symbols[Mana::G as usize] = 1;
        assert_eq!(cost.with_twobrids_paid(&[false, true, false]), paid);
        assert_eq!(cost.paid_with_any_type(), ManaCost { generic: 3, ..ManaCost::ZERO });
        assert_eq!(cost.paid_with_any_color(), ManaCost { generic: 3, ..ManaCost::ZERO });
        let colorless_white = ManaCost { hybrid: [Some((Mana::C, Mana::W)), None, None, None, None, None], ..ManaCost::ZERO };
        assert_eq!((colorless_white.mana_value(), colorless_white.colors()), (1, ColorSet::NONE.with(Color::White)));
        assert_eq!(colorless_white.to_string(), "{C/W}");
    }
}

/// A length as a `u32` count: exactly, or past `u32::MAX` that many and no more. A count of things that happen - objects
/// (every one made by an event or dealt), targets and candidates (each a different object or player, CR 115.3), cards
/// chosen, entries of the game's lists - is bounded between frames by the most events a game records before it is a draw
/// (`MAX_GAME_EVENTS`, 2^21; `docs/rules-exceptions.md`, RX-1), which is checked between frames; within one frame, only by
/// memory. The saturation is there so that no length is ever wrapped into a different, wrong count.
pub fn count_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod count_tests {
    #[test]
    fn a_count_past_u32_stops_at_its_bound() {
        assert_eq!(super::count_u32(300), 300);
        assert_eq!(super::count_u32(u32::MAX as usize), u32::MAX);
        assert_eq!(super::count_u32(usize::MAX), u32::MAX);
    }
}
