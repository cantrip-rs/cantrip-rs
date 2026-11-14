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

/// Mana of each kind, a `u8` each: what one payment spent from one source (`Event::ManaSpent`) and what one activation
/// produced (`Event::TappedForMana`). These are read only for the source and the kinds (a spell's whole spending is
/// `Obj::spent`, in 32 bits), so they stay narrow - the history holds one of them per such event. Past 255 a kind saturates
/// (RX-5, "the mana a cast records").
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

/// What mana may be spent on a cost's X (CR 107.3, 601.2h): "Spend only black mana on X" (Drain Life, Consume Spirit,
/// Crypt Rats), "Spend only red mana on X" (Crimson Hellkite), "Spend only white mana on X" (Atalya, Samite Master),
/// "Spend only black and/or red mana on X" (Soul Burn), and "Spend only colored mana on X. No more than one mana of
/// each color may be spent this way." (Emblazoned Golem). X itself is generic mana (CR 107.3a): a generic reduction may
/// reduce it (Drain Life's ruling), and what pays generic mana without spending mana (convoke, delve, improvise) may pay
/// it. A cost increase is not X (Atalya's ruling), so it is not restricted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct XMana {
    /// The types of mana that may be spent on X: a bit for each `Mana`, W U B R G C.
    pub types: u8,
    /// No more than one mana of each type may be spent on X: once a type has paid for X, `types` loses it.
    pub one_each: bool,
}

impl XMana {
    /// Any mana: no restriction.
    pub const ANY: XMana = XMana { types: 0b11_1111, one_each: false };
    /// "Spend only colored mana on X. No more than one mana of each color may be spent this way." (Emblazoned Golem)
    pub const COLORED_ONE_EACH: XMana = XMana { types: 0b1_1111, one_each: true };

    /// "Spend only [type] mana on X."
    pub const fn only(mana: Mana) -> XMana {
        XMana { types: 1 << mana as u8, one_each: false }
    }

    /// "Spend only [type] and/or [type] mana on X."
    pub const fn either(a: Mana, b: Mana) -> XMana {
        XMana { types: 1 << a as u8 | 1 << b as u8, one_each: false }
    }

    pub const fn is_any(self) -> bool {
        self.types == XMana::ANY.types && !self.one_each
    }

    /// Whether a unit of `mana` may be spent on X now.
    pub const fn may_pay(self, mana: Mana) -> bool {
        self.types & (1 << mana as u8) != 0
    }

    /// After a unit of `mana` was spent on X: one of each type takes that type away.
    pub const fn spent(self, mana: Mana) -> XMana {
        if self.one_each { XMana { types: self.types & !(1 << mana as u8), one_each: true } } else { self }
    }

    /// How many units of X it could still pay at most: one for each type where it takes one of each, unbounded otherwise.
    pub const fn most(self) -> u32 {
        if self.one_each { self.types.count_ones() } else { u32::MAX }
    }
}

impl Default for XMana {
    fn default() -> XMana {
        XMana::ANY
    }
}

/// A mana cost (CR 202): generic, and a count of each colored symbol; hybrid, monocolored hybrid, Phyrexian and {X}
/// symbols, and snow-source requirements ({S}, CR 107.4h), which are not mana types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ManaCost {
    pub generic: u32,
    /// {S} symbols: each requires one mana of any type produced by a snow source (CR 107.4h).
    pub snow: u32,
    /// W U B R G C, as `Mana` orders them.
    pub symbols: [u32; 6],
    /// Hybrid symbols (CR 107.4e): each paid with either of two types of mana. Two colors ({W/U}), or a color and
    /// colorless ({C/W}, a monocolored hybrid symbol whose other half is {C}).
    pub hybrid: [Option<(Mana, Mana)>; 6],
    /// Monocolored hybrid symbols with a generic half (CR 107.4e), {2/W} of each color, W U B R G: each paid with one mana
    /// of that color or two generic mana, chosen as the cost is announced (CR 601.2b, 118.13a); each counts 2 toward the
    /// mana value (CR 202.3f) and is its color (CR 202.2d).
    pub twobrid: [u32; 5],
    /// Phyrexian symbols (CR 107.4f) of each color, W U B R G: each paid with that color or 2 life.
    pub phyrexian: [u32; 5],
    /// How many {X} symbols it has (CR 107.3): {X}{X} is 2.
    pub x: u8,
    /// What mana may be spent on its X (`XMana`); `XMana::ANY` for most costs, where `not_x` means nothing and is 0.
    pub x_mana: XMana,
    /// Where `x_mana` restricts X: how much of `generic` is not X's, which any mana pays; the rest of `generic` is X's
    /// (`x_part`). X is added with `plus_x`, other generic mana with `add_generic` (a cost increase, CR 601.2f, is not X:
    /// Atalya's ruling). A reduction of generic mana (CR 118.7a) takes X's first, and so does what pays generic mana
    /// instead of mana (convoke, delve, improvise): each could pay either, and X's is the part fewer kinds of mana may pay,
    /// so that is never worse for the payer (Drain Life's ruling: "Cost reducers can be used to reduce the X part").
    pub not_x: u32,
}

impl ManaCost {
    pub const ZERO: ManaCost = ManaCost { generic: 0, snow: 0, symbols: [0; 6], hybrid: [None; 6], twobrid: [0; 5], phyrexian: [0; 5], x: 0, x_mana: XMana::ANY, not_x: 0 };

    /// The cost with `x_mana` restricting what pays its X, once announced (`plus_x`): what generic mana it has now is not X's.
    pub fn with_x_mana(mut self, x_mana: XMana) -> ManaCost {
        if !x_mana.is_any() && self.x_mana.is_any() {
            self.not_x = self.generic;
            self.x_mana = x_mana;
        }
        self
    }

    /// The generic mana only `x_mana` may pay: X's (CR 107.3).
    pub fn x_part(&self) -> u32 {
        if self.x_mana.is_any() { 0 } else { self.generic - self.not_x.min(self.generic) }
    }

    /// `amount` generic mana of X added (CR 107.3a: each {X} announced is that much generic mana), restricted to what
    /// `x_mana` allows.
    pub fn plus_x(self, amount: u32, x_mana: XMana) -> ManaCost {
        let mut out = if amount > 0 { self.with_x_mana(x_mana) } else { self };
        out.generic += amount;
        out
    }

    /// `n` generic mana added that is not X's (a cost increase, CR 601.2f; another cost's generic mana).
    pub fn add_generic(&mut self, n: u32) {
        self.generic += n;
        if !self.x_mana.is_any() {
            self.not_x += n;
        }
    }

    /// The generic mana any mana may pay: all of it but X's.
    pub fn plain_generic(&self) -> u32 {
        self.generic - self.x_part()
    }

    /// One unit of `mana` spent on the generic mana (CR 601.2h): on X's where it may pay that and some is owed - the part
    /// fewer kinds of mana may pay, so never worse for the payer - and otherwise on the rest. Whether it paid any.
    pub fn spend_generic(&mut self, mana: Mana) -> bool {
        if self.x_part() > 0 && self.x_mana.may_pay(mana) {
            self.generic -= 1;
            self.x_mana = self.x_mana.spent(mana);
            true
        } else if self.plain_generic() > 0 {
            self.generic -= 1;
            if !self.x_mana.is_any() {
                self.not_x = self.not_x.min(self.generic + 1) - 1;
            }
            true
        } else {
            false
        }
    }

    /// Up to `n` of the generic mana that is not X's paid (what any mana pays).
    pub fn spend_plain(&mut self, n: u32) {
        let plain = self.plain_generic();
        let pay = n.min(plain);
        self.generic -= pay;
        if !self.x_mana.is_any() {
            self.not_x = plain - pay;
        }
    }

    /// One unit of mana spent as though it were any type that may pay X's part (CR 609.4b): `mana`'s own where it may, else
    /// the first that may. Whether X's part had any left to pay.
    pub fn spend_x_as_any(&mut self, mana: Mana) -> bool {
        if self.x_part() == 0 || self.x_mana.types == 0 {
            return false;
        }
        let as_type = if self.x_mana.may_pay(mana) { mana } else { Mana::ALL[self.x_mana.types.trailing_zeros() as usize] };
        self.spend_generic(as_type)
    }

    /// The shapes its X's part could be paid in (CR 601.2h), each the cost with that part made symbols of the types that pay
    /// it: one of each of a set of colors where it takes one of each (Emblazoned Golem: every set of as many colors as
    /// X's part), or each split of it among the types it allows (one type: that many symbols of it). The cost itself where
    /// it has no such part; none where it can't be paid at all (more of one each than there are types).
    pub fn x_shapes(&self) -> Vec<ManaCost> {
        let part = self.x_part();
        if part == 0 {
            return vec![*self];
        }
        let mut base = *self;
        base.generic -= part;
        base.x_mana = XMana::ANY;
        base.not_x = 0;
        let types: Vec<usize> = (0..6).filter(|&m| self.x_mana.types & (1 << m) != 0).collect();
        let mut out = Vec::new();
        if self.x_mana.one_each {
            if part as usize > types.len() {
                return out;
            }
            for mask in 0u32..1 << types.len() {
                if mask.count_ones() == part {
                    let mut shape = base;
                    for (i, &m) in types.iter().enumerate() {
                        if mask & (1 << i) != 0 {
                            shape.symbols[m] += 1;
                        }
                    }
                    out.push(shape);
                }
            }
            return out;
        }
        // Each split of `part` among `types`: how many of each.
        fn split(base: ManaCost, types: &[usize], left: u32, out: &mut Vec<ManaCost>) {
            let Some((&m, rest)) = types.split_first() else { return };
            if rest.is_empty() {
                let mut shape = base;
                shape.symbols[m] += left;
                out.push(shape);
                return;
            }
            for here in 0..=left {
                let mut shape = base;
                shape.symbols[m] += here;
                split(shape, rest, left - here, out);
            }
        }
        split(base, &types, part, &mut out);
        out
    }

    pub fn mana_value(&self) -> u32 {
        self.generic
            + self.snow
            + self.symbols.iter().sum::<u32>()
            + self.hybrid.iter().flatten().count() as u32
            + 2 * self.twobrid.iter().sum::<u32>()
            + self.phyrexian.iter().sum::<u32>()
    }

    pub fn is_zero(&self) -> bool {
        self.mana_value() == 0
    }

    /// How many {2/W}-style symbols the cost has.
    pub fn twobrids(&self) -> u32 {
        self.twobrid.iter().sum()
    }

    /// The nonhybrid equivalent of its monocolored hybrid symbols (CR 601.2b): the symbols `generic` lists (by their order
    /// in `twobrid`, W first) paid with {2}, the others with their color.
    pub fn with_twobrids_paid(&self, generic: &[bool]) -> ManaCost {
        let mut out = *self;
        let mut at = 0;
        for (i, &n) in self.twobrid.iter().enumerate() {
            for _ in 0..n {
                if generic.get(at).copied().unwrap_or(false) {
                    out.add_generic(2);
                } else {
                    out.symbols[Color::ALL[i].mana() as usize] += 1;
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
        let mut generic = self.generic;
        for i in 0..5 {
            generic += self.symbols[i];
            paid.symbols[i] = 0;
        }
        // A hybrid symbol has a colored half, which any mana may now pay.
        generic += paid.hybrid.iter().flatten().count() as u32 + self.twobrids();
        paid.hybrid = [None; 6];
        paid.twobrid = [0; 5];
        paid.generic = generic;
        // Mana spent on X as though it were black is black mana spent on X (Celestial Dawn's ruling: white mana spent as
        // though it were another color pays what only that color may).
        if self.x_mana.types & 0b1_1111 != 0 {
            paid.lift_x(self);
        } else {
            paid.not_x = self.not_x + (generic - self.generic);
        }
        paid
    }

    /// The payment's view of X's part where any mana may stand for the types that may pay it (CR 609.4b): no restriction,
    /// unless it takes one of each and is more than there are types, which no mana can pay.
    fn lift_x(&mut self, of: &ManaCost) {
        if of.x_mana.is_any() || of.x_mana.one_each && of.x_part() > of.x_mana.most() {
            self.not_x = if of.x_mana.is_any() { 0 } else { of.not_x + (self.generic - of.generic) };
            return;
        }
        self.x_mana = XMana::ANY;
        self.not_x = 0;
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
        let symbols: u32 = self.symbols.iter().sum();
        let hybrid = self.hybrid.iter().flatten().count() as u32 + self.twobrids();
        paid.symbols = [0; 6];
        paid.hybrid = [None; 6];
        paid.twobrid = [0; 5];
        paid.generic = self.generic + symbols + hybrid;
        paid.lift_x(self);
        paid
    }

    /// The two costs added symbol by symbol (CR 709.4b: a split card's mana cost is its halves' combined; a Room's
    /// is that of its unlocked doors, CR 709.5).
    pub fn plus(&self, other: &ManaCost) -> ManaCost {
        let mut sum = *self;
        sum.generic += other.generic;
        // X's part stays X's, whichever side has it; the rest of the generic mana is not X's.
        if !other.x_mana.is_any() && self.x_mana.is_any() {
            sum.x_mana = other.x_mana;
            sum.not_x = self.generic + other.plain_generic();
        } else if !self.x_mana.is_any() {
            sum.not_x = self.plain_generic() + other.plain_generic();
        }
        sum.snow += other.snow;
        sum.x += other.x;
        for i in 0..6 {
            sum.symbols[i] += other.symbols[i];
        }
        for i in 0..5 {
            sum.phyrexian[i] += other.phyrexian[i];
            sum.twobrid[i] += other.twobrid[i];
        }
        for pair in other.hybrid.iter().flatten() {
            if let Some(slot) = sum.hybrid.iter().position(|h| h.is_none()) {
                sum.hybrid[slot] = Some(*pair);
            }
        }
        sum
    }

    /// The cost paid `n` times over, each symbol `n` times (CR 603.12a: "you may pay {G} up to three times" is {G}{G} paid
    /// twice). `None` when the result does not fit the cost's shape: more {X} symbols than a byte counts, or more hybrid
    /// symbols than its slots hold. Never a smaller cost than `n` payments.
    pub fn times(&self, n: u32) -> Option<ManaCost> {
        let mut out = ManaCost { generic: self.generic * n, snow: self.snow * n, not_x: self.not_x * n, x: u8::try_from(u32::from(self.x) * n).ok()?, hybrid: [None; 6], ..*self };
        for i in 0..6 {
            out.symbols[i] = self.symbols[i] * n;
        }
        for i in 0..5 {
            out.phyrexian[i] = self.phyrexian[i] * n;
            out.twobrid[i] = self.twobrid[i] * n;
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
        for _ in 0..self.snow { f.write_str("{S}")?; }
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

/// Which Auras an instance of protection doesn't remove (CR 702.16n): "This effect doesn't remove this Aura" (Black Ward,
/// Cho-Manno's Blessing, Tattoo Ward: the Aura whose effect it is) or "This effect doesn't remove Auras" (Spectra Ward).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Spares {
    /// This Aura, the source of the effect.
    Aura(ObjectId),
    /// Every Aura.
    Auras,
}

/// Protection from colors and card types whose instances don't remove some Auras (CR 702.16n): it does everything protection
/// does (CR 702.16b-e, `Chars::protected_from`), but an Aura it spares is not put into its owner's graveyard by it as a
/// state-based action (CR 704.5m); another instance of protection from the same quality, one that doesn't spare that Aura,
/// still removes it. It does not let an Aura be attached that the protection forbids (CR 702.16c; Spectra Ward's ruling: the
/// creature can't be the target of further Aura spells), nor spare Equipment (CR 702.16d), which is no Aura.
///
/// Per quality - the five colors (`Color` as the index), then the card types (5 + `CardType` as the index) - every instance
/// adds only bits, so what one adds never depends on what is there (the contract of a static ability's apply): `ids` ORs the
/// ids of the Auras whose instances spare only themselves, `not_ids` ORs their complements. One such Aura alone leaves
/// `ids == !not_ids`, its id; two different ones leave them apart, and each removes the other. A quality given with no Aura of
/// its own is spared to every Aura (Spectra Ward's) unless one such instance also gives it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct SparedProtection {
    colors: ColorSet,
    types: TypeSet,
    ids: [u32; Self::QUALITIES],
    not_ids: [u32; Self::QUALITIES],
}

impl SparedProtection {
    const QUALITIES: usize = 5 + CardType::ALL.len();
    pub const NONE: SparedProtection =
        SparedProtection { colors: ColorSet::NONE, types: TypeSet::NONE, ids: [0; Self::QUALITIES], not_ids: [0; Self::QUALITIES] };

    /// Protection from these colors and card types that doesn't remove `spares` (CR 702.16n).
    pub fn add(&mut self, colors: ColorSet, types: TypeSet, spares: Spares) {
        self.colors = ColorSet(self.colors.0 | colors.0);
        self.types = TypeSet(self.types.0 | types.0);
        if let Spares::Aura(aura) = spares {
            let at = colors.iter().map(|c| c as usize).chain(types.iter().map(|t| 5 + t as usize));
            for i in at {
                self.ids[i] |= aura.0;
                self.not_ids[i] |= !aura.0;
            }
        }
    }

    /// Every color it is protection from.
    pub fn colors(&self) -> ColorSet {
        self.colors
    }

    /// Every card type it is protection from.
    pub fn types(&self) -> TypeSet {
        self.types
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty() && self.types == TypeSet::NONE
    }

    /// The colors and card types from which an instance that doesn't spare `aura` gives protection: one of an Aura of its
    /// own that is another Aura, or two such Auras' (CR 702.16n).
    pub fn removing(&self, aura: ObjectId) -> (ColorSet, TypeSet) {
        let removes = |i: usize| (self.ids[i] | self.not_ids[i]) != 0 && !(self.ids[i] == aura.0 && self.not_ids[i] == !aura.0);
        let colors = self.colors.iter().filter(|&c| removes(c as usize)).fold(ColorSet::NONE, ColorSet::with);
        let types = self.types.iter().filter(|&t| removes(5 + t as usize)).fold(TypeSet::NONE, TypeSet::with);
        (colors, types)
    }

    /// Its bits, for a check that compares characteristics field by field.
    pub fn bits(&self) -> Vec<u64> {
        let mut bits = vec![u64::from(self.colors.0) | u64::from(self.types.0) << 8];
        bits.extend(self.ids.iter().zip(self.not_ids).map(|(&id, not)| u64::from(id) | u64::from(not) << 32));
        bits
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
    /// World permanents obey the world state-based action (CR 205.4f, 704.5k).
    pub world: bool,
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

    /// Whether the two sets share a subtype.
    pub fn intersects(self, other: SubtypeSet) -> bool {
        self.0.iter().zip(other.0).any(|(a, b)| a & b != 0)
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
///
/// Every change is kept, however many an object gets: what a word reads as depends on all of them and their order
/// (`word`), so none may be dropped or merged. The first `INLINE` are held in the value itself, as every real game's
/// are, so reading and changing them touches nothing shared; the rest, past that, are interned (`TextChanges`): `more`
/// names the list of the changes after the first `INLINE`, one entry per distinct such list ever made in the process, each
/// keyed by the list it extends and the change added, so that two equal lists are always one value and `==` is the
/// lists' equality. `Hash` hashes the changes themselves, never an entry's place, so a hash is the same in every process
/// and run.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Words {
    inline: [Option<(Word, Word)>; Words::INLINE],
    /// 0 while the changes fit `inline`; else the `TextChanges` entry (`more - 1`) of the changes after them.
    more: u32,
}

/// The interned lists of the text changes past a `Words`'s inline ones (entry `n - 1` for `more == n`): each the list it
/// extends and one change more, with the whole list kept for reading. Entries are only ever added, so a `Words` stays good
/// for the life of the process, through every undo and copy of a game.
struct TextChanges {
    lists: Vec<Box<[(Word, Word)]>>,
    by_extension: std::collections::HashMap<(u32, Word, Word), u32>,
}

fn text_changes() -> &'static std::sync::RwLock<TextChanges> {
    static TABLE: std::sync::OnceLock<std::sync::RwLock<TextChanges>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| std::sync::RwLock::new(TextChanges { lists: Vec::new(), by_extension: std::collections::HashMap::new() }))
}

impl Words {
    /// The changes held in the value itself; the rest are interned.
    const INLINE: usize = 4;
    pub const NONE: Words = Words { inline: [None; Words::INLINE], more: 0 };

    /// Whether the text is as printed.
    pub fn is_printed(&self) -> bool {
        self.inline[0].is_none()
    }

    /// The text with every instance of `from` replaced by `to` (CR 612.1), after the changes made so far.
    pub fn changed(mut self, from: Word, to: Word) -> Words {
        if self.more == 0
            && let Some(slot) = self.inline.iter().position(|c| c.is_none())
        {
            self.inline[slot] = Some((from, to));
            return self;
        }
        let key = (self.more, from, to);
        let table = text_changes();
        if let Some(&n) = table.read().unwrap_or_else(|e| e.into_inner()).by_extension.get(&key) {
            return Words { more: n, ..self };
        }
        let mut t = table.write().unwrap_or_else(|e| e.into_inner());
        if let Some(&n) = t.by_extension.get(&key) {
            return Words { more: n, ..self };
        }
        // An entry for each distinct list of changes past the inline ones ever made: 2^32 of them would take hundreds of
        // gigabytes first (docs/known-issues.md, KI-150). Past that the change is not kept.
        let Ok(n) = u32::try_from(t.lists.len() + 1) else { return self };
        let mut list: Vec<(Word, Word)> = if self.more == 0 { Vec::new() } else { t.lists[self.more as usize - 1].to_vec() };
        list.push((from, to));
        t.lists.push(list.into_boxed_slice());
        t.by_extension.insert(key, n);
        Words { more: n, ..self }
    }

    /// Every change of `other` after these.
    pub fn then(self, other: Words) -> Words {
        if other.more == 0 {
            return other.inline.into_iter().flatten().fold(self, |w, (from, to)| w.changed(from, to));
        }
        let changes = other.with_changes(|c| c.to_vec());
        changes.into_iter().fold(self, |w, (from, to)| w.changed(from, to))
    }

    /// The inline changes, in timestamp order.
    fn inline_changes(&self) -> impl Iterator<Item = (Word, Word)> + '_ {
        self.inline.iter().map_while(|c| *c)
    }

    /// `read` of every change, in timestamp order: collected, with the shared table read, only past the inline ones.
    fn with_changes<T>(&self, read: impl FnOnce(&[(Word, Word)]) -> T) -> T {
        let mut all: Vec<(Word, Word)> = self.inline_changes().collect();
        if self.more != 0 {
            let t = text_changes().read().unwrap_or_else(|e| e.into_inner());
            all.extend_from_slice(&t.lists[self.more as usize - 1]);
        }
        read(&all)
    }

    /// The word printed as `printed`, as the text has it now. The changes apply in timestamp order (CR 613.7) unless one
    /// depends on another (CR 613.8a): a change of "Goblin" depends on a later change that makes a word "Goblin", as
    /// applying that one changes what the first does, so it waits for it (613.8b). For one word that is: the earliest change
    /// not yet applied whose word it is now, again and again, each change once. Two changes of the same word each take the
    /// other's word away, a loop applied in timestamp order: the earlier.
    pub fn word(&self, printed: Word) -> Word {
        if self.more != 0 {
            return self.with_changes(|changes| {
                let mut word = printed;
                let mut applied = vec![false; changes.len()];
                while let Some(i) = (0..changes.len()).find(|&i| !applied[i] && changes[i].0 == word) {
                    applied[i] = true;
                    word = changes[i].1;
                }
                word
            });
        }
        let mut word = printed;
        let mut applied = [false; Words::INLINE];
        while let Some(i) = (0..Words::INLINE).find(|&i| !applied[i] && self.inline[i].is_some_and(|(from, _)| from == word)) {
            applied[i] = true;
            if let Some((_, to)) = self.inline[i] {
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

impl std::hash::Hash for Words {
    /// As the slice of every change hashes (its length, then each change), so that inline and interned changes hash alike.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.more != 0 {
            return self.with_changes(|changes| changes.hash(state));
        }
        state.write_usize(self.inline_changes().count());
        for change in self.inline_changes() {
            change.hash(state);
        }
    }
}

impl fmt::Debug for Words {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_changes(|changes| f.debug_tuple("Words").field(&changes).finish())
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
    /// Horsemanship (CR 702.31): an evasion ability - it can't be blocked by creatures without horsemanship (CR 702.31b).
    Horsemanship = 45,
    /// Fear (CR 702.36): an evasion ability - it can't be blocked except by artifact creatures and/or black creatures
    /// (CR 702.36b).
    Fear = 46,
    /// Intimidate (CR 702.13): an evasion ability - it can't be blocked except by artifact creatures and/or creatures that
    /// share a color with it (CR 702.13b).
    Intimidate = 47,
    /// Skulk (CR 702.118): an evasion ability - it can't be blocked by creatures with greater power (CR 702.118b).
    Skulk = 48,
    /// Nonbasic landwalk (CR 702.14): it can't be blocked as long as the defending player controls a land without the
    /// supertype basic (CR 702.14c). Dryad Sophisticate.
    NonbasicLandwalk = 49,
    /// Legendary landwalk (CR 702.14): it can't be blocked as long as the defending player controls a legendary land
    /// (CR 702.14c). Ayumi, the Last Visitor.
    LegendaryLandwalk = 50,
    /// Wither (CR 702.80): damage it deals to a creature isn't marked on that creature but is dealt as that many -1/-1
    /// counters (CR 702.80a, 120.3d), whatever zone it deals the damage from (CR 702.80c).
    Wither = 51,
    /// Phasing (CR 702.26): a static ability that modifies the untap step - during its controller's untap step it phases out
    /// if it is phased in, and the permanents that phased out under that player's control phase in (CR 702.26a). The core
    /// plays it (`phasing::untap_step`). Breezekeeper, Merfolk Raiders, Sandbar Crocodile, Teferi's Drake, Tolarian Drake.
    Phasing = 56,
    /// "This spell can't be copied" (CR 101.2, 707.10): a characteristic of the spell, so no effect copies it. Printed on Choreographed Sparks.
    CantBeCopied = 57,
    /// Mutate (CR 702.140a): a static ability that functions while the spell with it is on the stack. A card has it by its
    /// mutate way (`WayKind::Mutate`), which holds its cost, so "if it has mutate" (Pollywog Symbiote) reads it.
    Mutate = 58,
    /// Read ahead (CR 702.155): a Saga with it chooses, as it enters, a number from one to its final chapter number and
    /// enters with that many lore counters instead of one (CR 702.155b, 714.3b), and its chapter abilities can't trigger the
    /// turn it entered unless it has exactly the number of lore counters of the chapter (CR 702.155a). Both change core
    /// procedures - how a permanent enters and when a chapter ability triggers - so the core plays it (`ops::prepare_entry`,
    /// `settle`). Printed (Founding the Third Path, The Cruelty of Gix, Urza Assembles the Titans) or granted (Barbara
    /// Wright's "Sagas you control have read ahead"); instances are redundant (CR 702.155c).
    ReadAhead = 59,
    /// "This creature assigns combat damage equal to its toughness rather than its power" (CR 510.1a): printed, or granted as
    /// an ability ("has" or "gains", CR 113.1a), so the layers decide whether it has it (CR 613.1f, 613.7): "loses all
    /// abilities" takes it from a creature, and one granted later is kept. It changes the damage assigned and nothing else:
    /// everything that reads power, a fight included, reads its real power (Streetwise Negotiator's ruling). Where it is no
    /// ability of the creature - "each creature you control assigns", "equipped creature assigns" - it is that permanent's
    /// `CardDef::damage_by_toughness`, or a player's `EffectKind::CreaturesAssignDamageByToughness`; `static_rules::combat_damage_of`
    /// reads all three.
    AssignsDamageByToughness = 60,
    /// Suspend (CR 702.62a): a card has it by the suspend it prints (`CardDef::suspend`, which holds its number and cost, for
    /// the static ability that exiles it from the hand), or gains it - Delay's "if it doesn't have suspend, it gains suspend",
    /// Epochrasite's "it gains suspend" - so "suspended" (CR 702.62b) reads the object's characteristics, not its card. The
    /// layers give an object that has it once layer 6 is done suspend's two triggered abilities, which function in exile
    /// (`intrinsic::SUSPEND_UPKEEP`, `SUSPEND_CAST`).
    Suspend = 61,
    /// Snow landwalk variants (CR 702.14a,c). Rime Dryad and Legions of Lim-Dûl print snow forestwalk and snow
    /// swampwalk; all five land types are needed when an effect changes a basic land type word (CR 612.2).
    SnowPlainswalk = 62,
    SnowIslandwalk = 63,
    SnowSwampwalk = 64,
    SnowMountainwalk = 65,
    SnowForestwalk = 66,
    /// Snow landwalk (CR 702.14c): any snow land, as printed on Zombie Musher.
    SnowLandwalk = 67,
    /// Desertwalk (CR 702.14c): a land with the Desert subtype, as printed on Desert Nomads.
    Desertwalk = 68,
}

impl Keyword {
    /// Every supported landwalk variant (CR 702.14a), for effects referring to landwalk without a qualifier.
    pub const ALL_LANDWALKS: [Keyword; 14] = [
        Self::Plainswalk, Self::Islandwalk, Self::Swampwalk, Self::Mountainwalk, Self::Forestwalk,
        Self::NonbasicLandwalk, Self::LegendaryLandwalk, Self::SnowPlainswalk, Self::SnowIslandwalk,
        Self::SnowSwampwalk, Self::SnowMountainwalk, Self::SnowForestwalk, Self::SnowLandwalk, Self::Desertwalk,
    ];

    /// Each snow basic-landwalk and the land type it names. Snow remains a separate restriction after a text change.
    pub const SNOW_LANDWALKS: [(Keyword, Subtype); 5] = [
        (Self::SnowPlainswalk, Subtype::Plains), (Self::SnowIslandwalk, Subtype::Island),
        (Self::SnowSwampwalk, Subtype::Swamp), (Self::SnowMountainwalk, Subtype::Mountain),
        (Self::SnowForestwalk, Subtype::Forest),
    ];

    /// The snow counterpart of an ordinary basic-landwalk. A reference to islandwalk includes snow islandwalk
    /// (Merfolk Assassin's official 2006-10-15 ruling); granting islandwalk still grants only islandwalk.
    pub const fn snow_landwalk(self) -> Option<Keyword> {
        match self {
            Self::Plainswalk => Some(Self::SnowPlainswalk), Self::Islandwalk => Some(Self::SnowIslandwalk),
            Self::Swampwalk => Some(Self::SnowSwampwalk), Self::Mountainwalk => Some(Self::SnowMountainwalk),
            Self::Forestwalk => Some(Self::SnowForestwalk), _ => None,
        }
    }

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
pub struct Keywords(pub u128);

impl Keywords {
    pub const NONE: Keywords = Keywords(0);
    /// All landwalk variants, for an unqualified reference such as "creatures with landwalk" or "lose landwalk".
    pub const LANDWALKS: Keywords = Keywords::of(&Keyword::ALL_LANDWALKS);

    /// Whether an effect's reference to an ability matches this set (CR 702.1d). Unlike exact `has`, a reference to a
    /// basic-landwalk also includes its snow counterpart (Merfolk Assassin's official 2006-10-15 ruling).
    pub fn matches(self, keyword: Keyword) -> bool {
        self.has(keyword) || keyword.snow_landwalk().is_some_and(|snow| self.has(snow))
    }

    /// Expand named ability references for effects that check or remove them. Grants must use the original exact set:
    /// granting forestwalk does not grant snow forestwalk. An unqualified landwalk reference uses `LANDWALKS`.
    pub fn for_effect(self) -> Keywords {
        let mut result = self;
        for (ordinary, _) in Keyword::LANDWALKS {
            if self.has(ordinary) {
                result = result.with(ordinary.snow_landwalk().unwrap());
            }
        }
        result
    }

    /// Whether this set has any landwalk variant (CR 702.14a).
    pub fn has_landwalk(self) -> bool {
        self.0 & Self::LANDWALKS.0 != 0
    }

    pub const fn of(keywords: &[Keyword]) -> Keywords {
        let mut bits = 0u128;
        let mut i = 0;
        while i < keywords.len() {
            bits |= 1u128 << keywords[i] as u8;
            i += 1;
        }
        Keywords(bits)
    }

    pub fn has(self, k: Keyword) -> bool {
        self.0 & (1u128 << k as u8) != 0
    }

    pub fn with(self, k: Keyword) -> Keywords {
        Keywords(self.0 | (1u128 << k as u8))
    }
}

impl fmt::Debug for Keywords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Keywords({:#x})", self.0)
    }
}

/// Counters an object can have (CR 122), by kind: one line each - the kind, its index in an object's dense counter array,
/// its name as Oracle text writes it (CR 122.1), and the token the scenario dump names it by (`Counters.PlusOnePlusOne`).
/// A power/toughness counter also names its signed deltas (CR 122.1a), applied in layer 7c. Armor Thrull's +1/+2 and
/// Clockwork Avian's +1/+0 use the same rule. A kind a new card prints is one more line at the end: the host's view and events, proliferate, "remove all counters"
/// and the client's pills all walk `CounterKind::ALL` and name the kind, so nothing else changes.
macro_rules! counter_kinds {
    ($($kind:ident = $n:literal, $name:literal, $token:literal $(, ($power:literal, $toughness:literal))?;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum CounterKind {
            $($kind = $n,)*
        }

        impl CounterKind {
            pub const ALL: [CounterKind; [$($n),*].len()] = [$(CounterKind::$kind),*];
            pub const COUNT: usize = Self::ALL.len();

            /// Every power/toughness counter and the signed change one counter makes (CR 122.1a, 613.4c).
            /// Other named counters have no intrinsic power/toughness effect.
            pub const POWER_TOUGHNESS: &[(CounterKind, i16, i16)] = &[
                $($( (CounterKind::$kind, $power, $toughness), )?)*
            ];

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
    PlusOne = 0, "+1/+1", "PlusOnePlusOne", (1, 1);
    MinusOne = 1, "-1/-1", "MinusOneMinusOne", (-1, -1);
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
    // CR 702.24a: cumulative upkeep puts one on its permanent each upkeep and charges its cost once for each.
    Age = 52, "age", "Age";
    Growth = 53, "growth", "Growth";
    Invasion = 54, "invasion", "Invasion";
    Dream = 55, "dream", "Dream";
    Promotion = 56, "promotion", "Promotion";
    Bounty = 57, "bounty", "Bounty";
    Pupa = 58, "pupa", "Pupa";
    Matrix = 59, "matrix", "Matrix";
    Scream = 60, "scream", "Scream";
    Vitality = 61, "vitality", "Vitality";
    Fire = 62, "fire", "Fire";
    Conqueror = 63, "conqueror", "Conqueror";

    Film = 64, "film", "Film";
    Peace = 65, "peace", "Peace";
    Mire = 66, "mire", "Mire";

    // Private foundation additions follow the published counter prefix (CR 122.1).
    Wind = 67, "wind", "Wind";
    Hatchling = 68, "hatchling", "Hatchling";
    Sleep = 69, "sleep", "Sleep";
    Plus1Plus0 = 70, "+1/+0", "Plus1Plus0", (1, 0);
    Plus0Plus1 = 71, "+0/+1", "Plus0Plus1", (0, 1);
    Plus2Plus2 = 72, "+2/+2", "Plus2Plus2", (2, 2);
    Minus2Minus2 = 73, "-2/-2", "Minus2Minus2", (-2, -2);
    Plus0Minus1 = 74, "-0/-1", "Plus0Minus1", (0, -1);
    Plus0Minus2 = 75, "-0/-2", "Plus0Minus2", (0, -2);
    Plus1Plus2 = 76, "+1/+2", "Plus1Plus2", (1, 2);
    Minus2Minus1 = 77, "-2/-1", "Minus2Minus1", (-2, -1);
    Plus2Plus0 = 78, "+2/+0", "Plus2Plus0", (2, 0);
    Plus0Plus2 = 79, "+0/+2", "Plus0Plus2", (0, 2);
    Minus1Plus0 = 80, "-1/-0", "Minus1Plus0", (-1, 0);
    Carrion = 81, "carrion", "Carrion";
    Credit = 82, "credit", "Credit";
    Depletion = 83, "depletion", "Depletion";
    Healing = 84, "healing", "Healing";
    Javelin = 85, "javelin", "Javelin";
    Paralyzation = 86, "paralyzation", "Paralyzation";
    Spore = 87, "spore", "Spore";
    Storage = 88, "storage", "Storage";
    Tide = 89, "tide", "Tide";
    Music = 90, "music", "Music";
    Vow = 91, "vow", "Vow";
    Cube = 92, "cube", "Cube";
    Doom = 93, "doom", "Doom";
    Glyph = 94, "glyph", "Glyph";
    Pin = 95, "pin", "Pin";
    Sleight = 96, "sleight", "Sleight";

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

/// A phase of a turn (CR 500.1). Main phases are classified when they begin (CR 505.1a).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase { Beginning, Main, Combat, Ending }

/// A whole phase (CR 500.8), or a phase containing just the named step (CR 500.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhaseSpec { Whole(Phase), Only(Step) }

impl PhaseSpec {
    pub fn phase(self) -> Phase {
        match self { Self::Whole(phase) => phase, Self::Only(step) => step.phase() }
    }
}

/// The phase after which an effect inserts phases. Main ordinals count this turn only (CR 505.1b).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhaseAnchor { Current, Main(u32), Id(u32) }

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
    pub fn phase(self) -> Phase {
        match self {
            Self::Untap | Self::Upkeep | Self::Draw => Phase::Beginning,
            Self::PrecombatMain | Self::PostcombatMain => Phase::Main,
            Self::End | Self::Cleanup => Phase::Ending,
            _ => Phase::Combat,
        }
    }

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
    /// The kth of a card's `lasting_spell_alternatives`: an alternative cost a resolved spell or ability of it gave the spells a
    /// player casts for a duration (`EffectKind::SpellAlternative`, CR 118.9).
    pub const LASTING_SPELL_ALTERNATIVE: u8 = 112;
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

    pub fn lasting_spell_alternative(card: CardId, k: usize) -> AbilityRef {
        debug_assert!(k < Self::BAND as usize);
        AbilityRef { card, index: Self::LASTING_SPELL_ALTERNATIVE + k as u8 }
    }

    pub fn as_lasting_spell_alternative(self) -> Option<usize> {
        self.band(Self::LASTING_SPELL_ALTERNATIVE, Self::BAND)
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
        assert_eq!(g.times(300).map(|c| c.symbols[Mana::G as usize]), Some(300), "past a byte");
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

    /// CR 612.1, 613.7: every text change an object gets is kept, however many: six changes chained one after another
    /// take "Elf" through all six, and the same changes made again are the same words.
    #[test]
    fn every_text_change_is_kept() {
        let chain = [Subtype::Elf, Subtype::Goblin, Subtype::Dragon, Subtype::Merfolk, Subtype::Zombie, Subtype::Vampire, Subtype::Wizard].map(Word::Subtype);
        let changed = |n: usize| chain.windows(2).take(n).fold(Words::NONE, |w, pair| w.changed(pair[0], pair[1]));
        let six = changed(6);
        assert_eq!(six.word(chain[0]), chain[6], "the sixth change applies");
        assert_eq!(six.word(chain[5]), chain[6]);
        assert_eq!(changed(5).word(chain[0]), chain[5]);
        assert_eq!(six, changed(6), "the same changes are the same words");
        assert_ne!(six, changed(5));
        let hash = |w: Words| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            w.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(six), hash(changed(6)));
        // The hash is the changes', never the place of their list in the process's table, which depends on what was
        // interned first: the same in every process and run.
        let pairs: Vec<(Word, Word)> = chain.windows(2).map(|pair| (pair[0], pair[1])).collect();
        let slice_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            pairs.as_slice().hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(six), slice_hash, "a list of changes hashes as the changes");
        let three_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            pairs[..3].hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(changed(3)), three_hash, "changes held inline hash as the changes too");
        assert_eq!(Words::NONE.then(six), six);
        // Four changes fill the inline ones and a fifth is the first interned: the two lists are told apart, and each hashes as
        // its changes. The same list reached another way (two changes, then the other four after them) is the same words.
        assert_ne!(changed(4), changed(5));
        let slice = |n: usize| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            pairs[..n].hash(&mut h);
            h.finish()
        };
        assert_eq!((hash(changed(4)), hash(changed(5))), (slice(4), slice(5)));
        let rest = pairs[2..].iter().fold(Words::NONE, |w, &(from, to)| w.changed(from, to));
        assert_eq!(changed(2).then(rest), six, "the same list by another route");
        assert_eq!(hash(changed(2).then(rest)), hash(six));
        assert!(!six.is_printed() && Words::NONE.is_printed());
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

    /// CR 122.1: every kind of counter is named once, as Oracle text writes it, and the scenario dump's token for it is its
    /// own: a kind added at the end (Cyclopean Tomb's mire counter) takes the next index and clashes with none before it.
    #[test]
    fn every_counter_kind_has_its_own_name_and_token() {
        for (i, kind) in CounterKind::ALL.into_iter().enumerate() {
            assert_eq!(kind as usize, i, "{kind:?} sits at its index");
            for other in &CounterKind::ALL[..i] {
                assert_ne!(kind.name(), other.name());
                assert_ne!(kind.token(), other.token());
            }
        }
        assert_eq!((CounterKind::Mire.name(), CounterKind::Mire.token()), ("mire", "Mire"));
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


#[cfg(test)]
mod carrion_counter_tests {
    use super::*;
    #[test]
    fn published_counter_ids_survive_foundation_composition() {
        // Published IDs are a raw observation contract, independent of private prototype order.
        for (kind, id) in [
            (CounterKind::Pupa, 58), (CounterKind::Matrix, 59), (CounterKind::Scream, 60),
            (CounterKind::Vitality, 61), (CounterKind::Fire, 62), (CounterKind::Conqueror, 63),
            (CounterKind::Film, 64), (CounterKind::Peace, 65), (CounterKind::Mire, 66),
        ] {
            assert_eq!(kind as usize, id);
        }
        for (index, kind) in CounterKind::ALL.iter().enumerate() {
            assert_eq!(*kind as usize, index);
        }
    }

    #[test]
    fn carrion_is_a_distinct_named_counter_with_an_appended_dense_id() {
        let carrion = CounterKind::ALL.iter().copied().find(|kind| kind.name() == "carrion").expect("Osai Vultures uses carrion counters");
        assert_eq!(carrion as usize, 81);
        assert_eq!(carrion.token(), "Carrion");
        assert_ne!(carrion, CounterKind::Corpse);
        assert_eq!(CounterKind::ALL[carrion as usize], carrion);
        assert_eq!(CounterKind::Music as usize, 90);
        assert_eq!(CounterKind::Film as usize, 64);
        assert_eq!(CounterKind::Peace as usize, 65);
        assert_eq!(CounterKind::Vow as usize, 91);
        assert_eq!(CounterKind::COUNT, 97);
        assert!(!CounterKind::POWER_TOUGHNESS.iter().any(|(kind, _, _)| *kind == carrion));
    }
    #[test]
    fn early_set_counter_names_append_without_changing_existing_ids() {
        assert_eq!(CounterKind::Music as u8, 90);
        for (kind, id, name, token) in [(CounterKind::Cube, 92, "cube", "Cube"), (CounterKind::Doom, 93, "doom", "Doom"), (CounterKind::Glyph, 94, "glyph", "Glyph"), (CounterKind::Pin, 95, "pin", "Pin"), (CounterKind::Sleight, 96, "sleight", "Sleight")] {
            assert_eq!(kind as usize, id);
            assert_eq!(CounterKind::ALL[id], kind);
            assert_eq!(kind.name(), name);assert_eq!(kind.token(), token);
            assert!(!CounterKind::POWER_TOUGHNESS.iter().any(|(k, _, _)| *k == kind));
        }
        assert_ne!(CounterKind::Cube, CounterKind::Doom);
    }

}

#[cfg(test)]
mod landwalk_keyword_tests {
    use super::*;

    #[test]
    fn published_keyword_id_and_appended_landwalk_ids_are_distinct() {
        assert_eq!(Keyword::AssignsDamageByToughness as u8, 60);
        for (offset, keyword) in [Keyword::SnowPlainswalk, Keyword::SnowIslandwalk, Keyword::SnowSwampwalk,
            Keyword::SnowMountainwalk, Keyword::SnowForestwalk, Keyword::SnowLandwalk, Keyword::Desertwalk].into_iter().enumerate() {
            assert_eq!(keyword as usize, 62 + offset);
        }
    }

    #[test]
    fn effect_references_include_the_corresponding_snow_variant_without_changing_raw_membership() {
        let cases = [
            (Keyword::Plainswalk, Keyword::SnowPlainswalk, Subtype::Plains),
            (Keyword::Islandwalk, Keyword::SnowIslandwalk, Subtype::Island),
            (Keyword::Swampwalk, Keyword::SnowSwampwalk, Subtype::Swamp),
            (Keyword::Mountainwalk, Keyword::SnowMountainwalk, Subtype::Mountain),
            (Keyword::Forestwalk, Keyword::SnowForestwalk, Subtype::Forest),
        ];
        for (ordinary, snow, ty) in cases {
            assert_eq!(ordinary.snow_landwalk(), Some(snow));
            let set = Keywords::of(&[snow]);
            assert!(set.matches(ordinary));
            assert!(set.has(snow));
            assert!(!set.has(ordinary));
            assert!(!Keywords::of(&[ordinary]).has(snow));
            for (other, _, other_ty) in cases {
                assert_eq!(set.matches(other), ty == other_ty);
            }
            assert!(!set.matches(Keyword::SnowLandwalk), "snow landwalk is a distinct ability");
        }
        assert!(!Keywords::NONE.has_landwalk());
        assert!(!Keywords::of(&[Keyword::Flying]).has_landwalk());
        for walk in Keyword::ALL_LANDWALKS {
            assert!(Keywords::of(&[walk]).has_landwalk());
            assert!(Keywords::LANDWALKS.has(walk));
        }
        assert_eq!(Keywords::LANDWALKS.0.count_ones(), 14);
        assert_eq!(Keywords::of(&[Keyword::Flying]).for_effect(), Keywords::of(&[Keyword::Flying]));
    }
}
