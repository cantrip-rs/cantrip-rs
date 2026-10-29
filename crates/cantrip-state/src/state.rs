//! The game state and its one write API. Every write records how to undo it, so that a game can be moved back
//! through what happened: make and unmake for search, and the rollback a re-run card effect needs.

use crate::decision::*;
use crate::object::*;
use crate::vocab::*;

/// Everything a game is. Plain data: cloning it is a handful of buffer copies, and nothing in it points anywhere.
///
/// Read freely; write only through the methods below. A field written directly is a write the journal does not
/// know about, and a rollback across it leaves the game wrong; debug builds check that every rollback restores the
/// state exactly.
#[derive(Clone, Debug)]
pub struct GameState {
    pub players: Vec<Player>,
    /// Restricted mana in the players' pools (CR 106.6), in the order each player's restrictions first came; an entry
    /// whose mana is all spent is gone. Empty in nearly every position.
    pub restricted_mana: Vec<RestrictedMana>,
    /// The mana in the players' pools by the source that produced it (CR 106.3), restricted or not: what "mana produced by
    /// [this]" and "mana from a [source]" ask of the mana spent (`Event::ManaSpent`). The entries of a player add up to
    /// their pool and their restricted mana exactly.
    pub mana_from: Vec<ManaFrom>,
    /// The timestamp of each kind of keyword counter on a permanent that has had one put on it since it entered (CR 613.7c):
    /// that of the latest counter of the kind, which every counter of the kind shares. A kind with no entry is as old as the
    /// permanent (it entered with them). Empty in nearly every game; rows of objects that have left are dropped as it is written.
    pub counter_stamps: Vec<CounterStamp>,
    pub objects: Vec<Obj>,
    /// Ordered object lists, one per zone slot (`Zone::slot`). Libraries are top first; the others in the order
    /// objects arrived.
    pub zones: Vec<Vec<ObjectId>>,
    /// Top last.
    pub stack: Vec<StackItem>,
    pub turn: Turn,
    pub effects: Vec<ContinuousEffect>,
    /// Permissions to play cards from where they are, oldest first.
    pub permissions: Vec<PlayPermission>,
    pub triggers: Vec<PendingTrigger>,
    pub delayed: Vec<Delayed>,
    pub last_known: Vec<LastKnown>,
    /// The stack items of spells and activated abilities that have left the stack (countered, resolved, exiled), as they
    /// last were on it: what a rule reads of one that is no longer there (CR 608.2h, 707.10; Rings of Brighthearth's copy of
    /// an ability countered in response). Append-only.
    pub left_stack: Vec<StackItem>,
    /// What happened this turn (CR 603.2h, 608.2m, 601.2i), for the rules that ask about it.
    /// The objects of the batches "one or more" abilities triggered by (`Span` names a run of it). Append-only.
    pub members: Vec<ObjectId>,
    /// The targets of stack items with more than `INLINE_TARGETS` of them, each with the spec it was chosen for
    /// (`StackItem::spill` names a run of it). Append-only, journaled.
    pub spill_targets: Vec<Target>,
    /// Parallel to `spill_targets`: the spec each was chosen for.
    pub spill_specs: Vec<u8>,
    /// The shares of a divided total of stack items that divide one (CR 601.2d; `StackItem::shares` names a run of it, one for
    /// each target). Append-only, journaled.
    pub share_arena: Vec<u32>,
    pub attackers: Vec<Attacker>,
    pub blocks: Vec<Block>,
    /// Extra turns waiting to be taken (CR 500.7): the most recently created last, and taken first.
    pub extra_turns: Vec<ExtraTurn>,
    /// What the rules watch to find a loop of passes (`docs/rules-exceptions.md`, RX-4): see `LoopWatch`.
    pub loop_watch: LoopWatch,
    /// The positions seen at priority in the present run of passes, each with the rounds the loop has come round it (see
    /// `LoopWatch`): apart from the watch, and changed an entry at a time, so a priority journals one entry, not all of them.
    pub loop_keys: Vec<LoopKey>,
    /// Where each of `loop_keys` is, by its key: derived from them, no part of the position, kept as they change.
    pub loop_index: LoopIndex,
    /// The basic land cards a Limited event supplies (Magic Tournament Rules: a Limited sideboard holds any number of basic
    /// lands), of which each player always has one more outside the game (`Zone::sideboard`): one brought into the game is
    /// replaced there. Empty for a constructed game. Fixed at the deal, so no part of the journal.
    pub outside_supply: Vec<CardId>,
    /// The minimum deck size of the game's format (CR 100.2a: sixty cards in Constructed; 100.2b: forty in Limited), which a
    /// companion's condition may name (Yorion, Sky Nomad). Fixed at the deal.
    pub minimum_deck_size: u32,
    /// Whether an object whose card has static abilities has ever been made in this game, or a hone counter put on a
    /// permanent (CR 122.1j): until one has, the layers skip looking for static sources, and hone counters, at all. Only
    /// ever set, so it can be stale only towards looking.
    pub statics_seen: bool,
    pub agenda: Vec<Frame>,
    pub rng: Rng,
    pub next_timestamp: u32,
    /// Request ids are never reused, not even across a rollback, so this only ever counts up and is no part of the
    /// position.
    pub next_request: u32,
    pub outcome: Outcome,
    /// Set by any write that could change what a state-based action or trigger sees; cleared by the check.
    pub unsettled: bool,
    pub journal: Journal,
    /// Counts every write and every undo, never going back but to a position reverted to (`restore_derived`): two reads of
    /// a state at one `epoch` see the same position.
    /// Derived data below is valid for the epoch it was made at, so it is journal-safe by construction. No part of the
    /// position.
    pub epoch: u64,
    /// Counts every write and undo that could change what the layers work out (`Undo::layered`): two reads of a state at one
    /// `chars_epoch` see the same characteristics, whatever else moved. No part of the position.
    pub chars_epoch: u64,
    /// The permanents whose static abilities could apply to anything, found by the layers (a function of the position).
    pub static_sources: std::cell::RefCell<StaticSources>,
    /// The face-down card outside the battlefield and the stack that is being judged as the card it is, while a player who may
    /// play it is asked whether they could (CR 406.3a-b, 601.3): the resulting spell or land play has the card's
    /// characteristics, though the card in exile has none. Set only for the length of that judgment
    /// (`cantrip_rules::layers::judged_face_up`), `None` at any other time; no part of the position.
    pub judged_face_up: std::cell::Cell<Option<ObjectId>>,
    /// The card outside the stack that is being judged as the face it would be cast as, where it is (CR 709.3, 715.3, 712.8,
    /// 720.3): the face's characteristics with the effects that apply to the card there, as a spell. Set only for the length
    /// of that judgment (`cantrip_rules::layers::chars_as_face`), `None` at any other time; no part of the position.
    pub judged_as: std::cell::Cell<Option<(ObjectId, CardId)>>,
    /// The permanents leaving the battlefield in the move whose entering permanents are being worked out (CR 101.4,
    /// 614.12: Goblin Welder's sacrifice and return), which a permanent judged as it would exist there does not see. Set only
    /// for the length of that work, empty at any other time; no part of the position.
    pub leaving_together: std::cell::RefCell<Vec<ObjectId>>,
    /// How many instructions that change several things at once are under way (an exchange of control, CR 701.12a): while
    /// any is, what continuous effects say of control and "for as long as" durations is not settled between their parts,
    /// only once the outermost has finished (`cantrip_rules::ops::settle_at_once`). No part of the position.
    pub settle_held: std::cell::Cell<u32>,
    /// Whether the one-shot effects that return what was exiled or phased out "until [a permanent] leaves the battlefield"
    /// are being carried out (CR 610.3, 610.4; `cantrip_rules::ops::return_until_left`): the moves they make look for no
    /// more returns of their own until they are done. No part of the position.
    pub returning: std::cell::Cell<bool>,
    /// The layers' work for one position and history (`cantrip_rules::layers`): each layer's application order (CR 613.8) and
    /// the characteristics worked out with it. Derived, shared by a clone until either moves on; no part of the position.
    pub layer_pass: LayerCache,
}

/// The layers' cache (`GameState::layer_pass`): passes the rules crate made, each for its own position and history.
#[derive(Default)]
pub struct LayerCache(pub std::cell::RefCell<Vec<std::rc::Rc<dyn std::any::Any>>>);

impl Clone for LayerCache {
    /// A copy starts with no pass: a pass is derived work for one position and history, and sharing it would share
    /// `Rc`s between games that may end up on different threads (a recorded game is walked on a worker).
    fn clone(&self) -> LayerCache {
        LayerCache::default()
    }
}

// SAFETY: every pass is owned by this cache alone (a clone shares none), so moving the cache to another thread moves
// the only references to its `Rc`s. The rules never take a pass out of a game that is then moved while they hold it.
unsafe impl Send for LayerCache {}

impl std::fmt::Debug for LayerCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LayerCache({})", self.0.borrow().len())
    }
}

/// What the rules watch to find a loop of passes (`docs/rules-exceptions.md`, RX-4; `cantrip_rules::loops`): every priority
/// with something on the stack given since a player last chose anything, but the run's first `skipped`, is fingerprinted
/// into `GameState::loop_keys`; any choice, or a question asked of the history, clears them. `last` is the position last
/// fingerprinted, so that a priority asked again (a run redone) is not counted twice.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LoopWatch {
    pub skipped: u8,
    pub last: Option<u64>,
}

/// A position seen at priority in the present run of passes, by `cantrip_rules::loops`'s fingerprint of it (`key`). The
/// first time the game comes round to it again, the position's canonical words are kept (`words`: everything the
/// fingerprint is made of, written out in full); each time after, a round is counted (`rounds`) only if the words are the
/// same, so that no draw rests on a hash alone.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LoopKey {
    pub key: u64,
    pub rounds: u32,
    pub words: Option<Vec<u64>>,
}

/// The hasher of `GameState::loop_index`, whose keys are already fingerprints: the key is its own hash.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyHasher(u64);

impl std::hash::Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(b);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }
}

/// `GameState::loop_index`'s map: a fingerprint to the entry it names.
pub type LoopIndex = std::collections::HashMap<u64, u32, std::hash::BuildHasherDefault<KeyHasher>>;

/// When the keyword counters of one kind on a permanent were put on it (CR 613.7c), for layer 6's timestamp order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CounterStamp {
    pub object: ObjectId,
    pub kind: CounterKind,
    pub timestamp: u32,
}

/// The layers' list of static-ability sources, made for one epoch of the state it sits in.
#[derive(Clone, Debug)]
pub struct StaticSources {
    pub epoch: u64,
    /// Each source, the zone it functions from, and whether a resolved effect took all its abilities away (CR 613.1f):
    /// such a source's static abilities of layers 1 to 5 still apply, as they did before layer 6 took them (CR 613.6),
    /// and those of layer 6 on do not.
    pub list: Vec<(ObjectId, ZoneKind, bool)>,
    /// Those of `list` with a static ability that makes objects lose all abilities (`StaticDef::loses_all`), which no
    /// resolved effect has taken away, in timestamp order.
    pub strippers: Vec<(ObjectId, ZoneKind)>,
    /// Those of `list` with a static ability that sets the land subtypes of what it affects (`StaticDef::sets_land_subtypes`,
    /// CR 305.7), whatever abilities they have lost in layer 6 (CR 613.6), in timestamp order.
    pub setters: Vec<(ObjectId, ZoneKind)>,
    /// Those of `list` on the battlefield with a control-changing static ability (CR 613.1b, layer 2), whatever abilities
    /// they have lost in layer 6 (CR 613.1), in timestamp order.
    pub controllers: Vec<ObjectId>,
    /// The permanents with hone counters on them (CR 122.1j), in battlefield order: what each gives the creature it is
    /// attached to is no ability of it, so it is found apart from `list`, whatever abilities the permanent has lost.
    pub honed: Vec<ObjectId>,
    /// The permanents an effect has given a static ability that affects other objects (CR 613.1f: The Irencrag's
    /// "Equipped creature gets +3/+3"), whatever abilities they have lost before it: the layers ask each whether it still
    /// has the ability.
    pub granted: Vec<ObjectId>,
    /// The permanents on the battlefield whose card has targeting rules (`CardDef::targeting_rules`, CR 115.4, 609.4), in
    /// battlefield order, whatever abilities they have lost: what `can_target` asks, and nothing when there are none.
    pub targeting: Vec<ObjectId>,
    /// Whether a source of `list` has a static ability that continues an effect begun in an earlier layer
    /// (`StaticDef::continues`, CR 613.6): only then do the layers note which effects began to apply to an object.
    pub continuing: bool,
}

impl StaticSources {
    /// None found yet: found again at the next question.
    pub fn none() -> StaticSources {
        StaticSources { epoch: u64::MAX, list: Vec::new(), strippers: Vec::new(), setters: Vec::new(), controllers: Vec::new(), honed: Vec::new(), granted: Vec::new(), targeting: Vec::new(), continuing: false }
    }
}

/// What the layers derived from a position (`GameState::static_sources`, `GameState::layer_pass`), with the counters it is
/// valid for, set aside while a question changes the position for the moment and reverts it (`GameState::set_derived_aside`).
pub struct DerivedAside {
    epoch: u64,
    chars_epoch: u64,
    static_sources: StaticSources,
    layer_pass: Vec<std::rc::Rc<dyn std::any::Any>>,
}

impl GameState {
    /// Sets aside what the layers derived from the position as it is now, for a question that changes it for the moment and
    /// reverts it to `mark()` taken now (a spell judged on the stack before it is cast, `cantrip_rules::ops::as_cast`): put back
    /// with `restore_derived`, what was worked out for the position before the question is used after it again, rather than
    /// worked out anew because the counters moved on.
    pub fn set_derived_aside(&self) -> DerivedAside {
        DerivedAside {
            epoch: self.epoch,
            chars_epoch: self.chars_epoch,
            static_sources: self.static_sources.replace(StaticSources::none()),
            layer_pass: std::mem::take(&mut *self.layer_pass.0.borrow_mut()),
        }
    }

    /// Puts back what `set_derived_aside` set aside, once the position is again the one it was derived for (`revert` to the
    /// mark taken with it). The counters go back with it, and nothing derived in between is kept, so no derived data is
    /// held for a count the position will next reach with something else.
    pub fn restore_derived(&mut self, aside: DerivedAside) {
        self.epoch = aside.epoch;
        self.chars_epoch = aside.chars_epoch;
        *self.static_sources.borrow_mut() = aside.static_sources;
        *self.layer_pass.0.borrow_mut() = aside.layer_pass;
    }

    fn position(
        &self,
    ) -> (
        (&Vec<Player>, &Vec<RestrictedMana>, &Vec<CounterStamp>, &Vec<ManaFrom>),
        &Vec<Obj>,
        &Vec<Vec<ObjectId>>,
        &Vec<StackItem>,
        &Turn,
        &Vec<ContinuousEffect>,
        (&Vec<PendingTrigger>, &Vec<Delayed>, &Vec<PlayPermission>),
        (&Vec<Attacker>, &Vec<Block>, &Vec<ExtraTurn>, (&LoopWatch, &Vec<LoopKey>)),
        &Vec<Frame>,
        (&Rng, u32, &Outcome, bool, &Vec<ObjectId>, &Vec<StackItem>, (&Vec<Target>, &Vec<u8>, &Vec<u32>)),
    ) {
        (
            (&self.players, &self.restricted_mana, &self.counter_stamps, &self.mana_from),
            &self.objects,
            &self.zones,
            &self.stack,
            &self.turn,
            &self.effects,
            (&self.triggers, &self.delayed, &self.permissions),
            (&self.attackers, &self.blocks, &self.extra_turns, (&self.loop_watch, &self.loop_keys)),
            &self.agenda,
            (&self.rng, self.next_timestamp, &self.outcome, self.unsettled, &self.members, &self.left_stack, (&self.spill_targets, &self.spill_specs, &self.share_arena)),
        )
    }
}

/// Two states are the same position whatever their journals and request counters hold.
impl PartialEq for GameState {
    fn eq(&self, other: &GameState) -> bool {
        // Last-known rows are named by the objects that left (`Obj::last_known`), which the position hashes, by the
        // stack items of the spells cast (`StackItem::cast_known`), and by the history's declarations of attackers
        // (`Event::Attacks::known`); the rows themselves are compared, not hashed, since every move, every cast and every
        // attacker declared adds one.
        self.position() == other.position() && self.last_known == other.last_known
    }
}

impl Eq for GameState {}

impl std::hash::Hash for GameState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.position().hash(state);
    }
}

/// The undo log. Not part of the position: two states that are the same game compare equal whatever their
/// journals hold.
/// A copy of a position (`clone`) starts with an empty journal at the same mark: the journal is the path's history,
/// which the game-log tree keeps, not part of a position, and copying it would make a copy cost what the game has
/// done so far. `GameState::clone_with_journal` copies it too.
#[derive(Debug, Default)]
pub struct Journal {
    pub entries: Vec<Undo>,
    /// How many entries have been forgotten from the front: a mark is absolute, so forgetting moves no mark.
    pub base: u32,
    /// Outstanding checkpoints. While there are none, the engine may drop entries no frame still needs.
    pub holds: u32,
}

impl Clone for Journal {
    fn clone(&self) -> Journal {
        Journal { entries: Vec::new(), base: self.base + self.entries.len() as u32, holds: self.holds }
    }
}

impl Journal {
    /// A copy with every entry.
    pub fn deep_clone(&self) -> Journal {
        Journal { entries: self.entries.clone(), base: self.base, holds: self.holds }
    }
}

impl PartialEq for Journal {
    fn eq(&self, _: &Journal) -> bool {
        true
    }
}

impl Eq for Journal {}

impl std::hash::Hash for Journal {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// How to undo one write. A position in a list is a `u32`: every list is of objects or entries the game's events made,
/// which the event cap bounds between frames (`docs/rules-exceptions.md`, RX-1) far inside it; within one frame, memory
/// bounds them first (2^32 entries are tens of gigabytes).
#[derive(Clone, Debug)]
pub enum Undo {
    Object(ObjectId, Box<Obj>),
    ObjectPushed,
    ObjectPopped(Box<Obj>),
    Player(PlayerId, Player),
    RestrictedManaSet(Vec<RestrictedMana>),
    ManaFromPushed,
    ManaFromSet(u32, ManaFrom),
    ManaFromRemoved(u32, ManaFrom),
    ManaFromInserted(u32),
    ManaFromAll(Vec<ManaFrom>),
    CounterStampsSet(Vec<CounterStamp>),
    Turn(Turn),
    ZoneInserted { slot: u32, at: u32 },
    ZoneRemoved { slot: u32, at: u32, id: ObjectId },
    StackPushed,
    StackPopped(Box<StackItem>),
    StackSet(u32, Box<StackItem>),
    EffectPushed,
    EffectRemoved(u32, Box<ContinuousEffect>),
    EffectInserted(u32),
    EffectSet(u32, Box<ContinuousEffect>),
    PermissionPushed,
    PermissionRemoved(u32, PlayPermission),
    PermissionInserted(u32),
    TriggerPushed,
    TriggerRemoved(u32, PendingTrigger),
    TriggerInserted(u32),
    DelayedPushed,
    DelayedRemoved(u32, Delayed),
    DelayedInserted(u32),
    LastKnownPushed,
    LastKnownPopped(Box<LastKnown>),
    LeftStackPushed,
    LeftStackPopped(Box<StackItem>),
    /// Members appended since the list was this long.
    MembersPushed(u32),
    MembersPopped(Vec<ObjectId>),
    /// Spilled targets appended since the arena was this long.
    TargetsSpilled(usize),
    TargetsUnspilled(Vec<Target>, Vec<u8>),
    /// Shares appended since the arena was this long.
    SharesPushed(usize),
    SharesPopped(Vec<u32>),
    AttackersSet(Vec<Attacker>),
    BlocksSet(Vec<Block>),
    ExtraTurnsSet(Vec<ExtraTurn>),
    LoopWatchSet(LoopWatch),
    LoopKeyPushed,
    LoopKeyPopped(LoopKey),
    LoopKeyRounds(u32, u32),
    /// The canonical words an entry had (none): see `LoopKey`.
    LoopKeyWords(u32, Option<Vec<u64>>),
    LoopKeysSet(Vec<LoopKey>),
    StaticsSeen(bool),
    FramePushed,
    FramePopped(Frame),
    FrameRemoved(u32, Frame),
    FrameInserted(u32),
    FrameSet(u32, Frame),
    Rng(Rng),
    Timestamp(u32),
    Outcome(Outcome),
    Unsettled(bool),
}

impl Undo {
    /// Whether the write could change what the layers work out (`GameState::chars_epoch`): anything of the objects, zones,
    /// stack, effects, players, turn, combat or last-known information, which a static ability's reach or condition may read.
    /// Not the rules' own bookkeeping - frames, the loop watch, pending and delayed triggers, the stack's arenas, mana's
    /// sources, play permissions, the random generator and the timestamp counter - which no characteristic depends on.
    pub fn layered(&self) -> bool {
        !matches!(
            self,
            Undo::TriggerPushed
                | Undo::TriggerRemoved(..)
                | Undo::TriggerInserted(_)
                | Undo::DelayedPushed
                | Undo::DelayedRemoved(..)
                | Undo::DelayedInserted(_)
                | Undo::LeftStackPushed
                | Undo::LeftStackPopped(_)
                | Undo::MembersPushed(_)
                | Undo::MembersPopped(_)
                | Undo::TargetsSpilled(_)
                | Undo::TargetsUnspilled(..)
                | Undo::SharesPushed(_)
                | Undo::SharesPopped(_)
                | Undo::LoopWatchSet(_)
                | Undo::LoopKeyPushed
                | Undo::LoopKeyPopped(_)
                | Undo::LoopKeyRounds(..)
                | Undo::LoopKeyWords(..)
                | Undo::LoopKeysSet(_)
                | Undo::FramePushed
                | Undo::FramePopped(_)
                | Undo::FrameRemoved(..)
                | Undo::FrameInserted(_)
                | Undo::FrameSet(..)
                | Undo::Rng(_)
                | Undo::Timestamp(_)
                | Undo::Unsettled(_)
                | Undo::ManaFromPushed
                | Undo::ManaFromSet(..)
                | Undo::ManaFromRemoved(..)
                | Undo::ManaFromInserted(_)
                | Undo::ManaFromAll(_)
                | Undo::PermissionPushed
                | Undo::PermissionRemoved(..)
                | Undo::PermissionInserted(_)
        )
    }
}

/// A point in the journal to come back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark(pub u32);

impl GameState {
    /// A game of `players` seats: at most `MAX_SEATS`, which every way a game is dealt refuses past (the host's
    /// `Session::new`), since a set of seats is a bit each in eight bits.
    pub fn new(players: usize, seed: u64) -> GameState {
        debug_assert!(players <= MAX_SEATS, "a game of {players} seats: at most {MAX_SEATS} are dealt");
        GameState {
            players: vec![Player::starting(20); players],
            restricted_mana: Vec::new(),
            mana_from: Vec::new(),
            counter_stamps: Vec::new(),
            objects: Vec::with_capacity(128),
            zones: vec![Vec::new(); Zone::slots(players)],
            stack: Vec::new(),
            turn: Turn::default(),
            effects: Vec::new(),
            permissions: Vec::new(),
            triggers: Vec::new(),
            delayed: Vec::new(),
            last_known: Vec::new(),
            left_stack: Vec::new(),
            members: Vec::new(),
            spill_targets: Vec::new(),
            spill_specs: Vec::new(),
            share_arena: Vec::new(),
            attackers: Vec::new(),
            blocks: Vec::new(),
            extra_turns: Vec::new(),
            loop_watch: LoopWatch::default(),
            loop_keys: Vec::new(),
            loop_index: LoopIndex::default(),
            outside_supply: Vec::new(),
            minimum_deck_size: 0,
            statics_seen: false,
            agenda: Vec::new(),
            rng: Rng::new(seed),
            next_timestamp: 1,
            next_request: 1,
            outcome: Outcome::InProgress,
            unsettled: true,
            journal: Journal::default(),
            epoch: 0,
            chars_epoch: 0,
            static_sources: std::cell::RefCell::new(StaticSources::none()),
            judged_face_up: std::cell::Cell::new(None),
            judged_as: std::cell::Cell::new(None),
            leaving_together: std::cell::RefCell::new(Vec::new()),
            settle_held: std::cell::Cell::new(0),
            returning: std::cell::Cell::new(false),
            layer_pass: LayerCache::default(),
        }
    }

    // --- Reading ----------------------------------------------------------------------------------------------

    pub fn obj(&self, id: ObjectId) -> &Obj {
        &self.objects[id.index()]
    }

    pub fn player(&self, p: PlayerId) -> &Player {
        &self.players[p.index()]
    }

    pub fn zone(&self, zone: Zone) -> &[ObjectId] {
        &self.zones[zone.slot(self.players.len())]
    }

    pub fn battlefield(&self) -> &[ObjectId] {
        self.zone(Zone::BATTLEFIELD)
    }

    /// Whether a resolved spell or ability's effect has taken all of `id`'s abilities (CR 611.2, 613.1f). Only half of
    /// the answer: a static ability may take them too, which only the layers know, so every rule asks
    /// `Query::lost_all_abilities` (`layers::lost_all_abilities`), never this. Free while no effect is lasting.
    pub fn lost_all_abilities_to_an_effect(&self, id: ObjectId) -> bool {
        !self.effects.is_empty() && self.effects.iter().any(|e| e.affected == id && e.kind == EffectKind::LoseAllAbilities)
    }

    /// Every seat at the table, in seat order, a player who has left the game included: for walking the zones by owner.
    /// "Each player" is `apnap`, never this.
    pub fn seats(&self) -> impl Iterator<Item = PlayerId> + use<> {
        (0..self.players.len() as u8).map(PlayerId)
    }

    /// Whether `p` is still in the game: a player who has lost has left it (CR 800.4a).
    pub fn in_game(&self, p: PlayerId) -> bool {
        !self.players[p.index()].lost
    }

    /// The player who makes `player`'s choices and decisions now (CR 723.5): the one controlling them, by the last effect
    /// created that gives control of them (CR 723.1a), or `player` themselves (723.8, 723.9). Only the routing of
    /// questions and what a seat sees follow it: every request is still asked of `player`, who keeps their objects and
    /// stays the active player (723.3).
    pub fn player_controller(&self, player: PlayerId) -> PlayerId {
        self.player_controller_counting(player, None)
    }

    /// Who controlled `player` before `leaver` left the game (CR 800.4a): `player_controller` with `leaver` still
    /// counted, while the effects that gave them control have not yet ended.
    pub fn player_controller_before_leaving(&self, player: PlayerId, leaver: PlayerId) -> PlayerId {
        self.player_controller_counting(player, Some(leaver))
    }

    fn player_controller_counting(&self, player: PlayerId, leaver: Option<PlayerId>) -> PlayerId {
        let mut by = player;
        let mut latest = None;
        for e in &self.effects {
            if e.kind == EffectKind::ControlsPlayer(player)
                && latest.is_none_or(|t| e.timestamp > t)
                && (self.in_game(e.controller) || leaver == Some(e.controller))
            {
                latest = Some(e.timestamp);
                by = e.controller;
            }
        }
        by
    }

    /// Whether `viewer` sees what `player` sees of the game now (CR 723.4): `player` themselves, or the player controlling
    /// them.
    pub fn sees_as(&self, viewer: PlayerId, player: PlayerId) -> bool {
        viewer == player || self.player_controller(player) == viewer
    }

    /// The players still in the game in turn order starting from the active one (APNAP, CR 101.4, 800.4a).
    pub fn apnap(&self) -> impl Iterator<Item = PlayerId> + use<> {
        self.turn_order_from(self.turn.active)
    }

    /// The players still in the game in turn order starting from `first` (CR 101.4, 800.4a): "starting with you, each
    /// player". A `first` who has left is passed over.
    pub fn turn_order_from(&self, first: PlayerId) -> impl Iterator<Item = PlayerId> + use<> {
        let n = self.players.len() as u8;
        let left = self.left_mask();
        (0..n).map(move |i| (first.0 + i) % n).filter(move |&i| left & (1 << i) == 0).map(PlayerId)
    }

    fn left_mask(&self) -> u32 {
        self.players.iter().enumerate().fold(0, |mask, (i, pl)| mask | (pl.lost as u32) << i)
    }

    pub fn next_player(&self, p: PlayerId) -> PlayerId {
        PlayerId((p.0 + 1) % self.players.len() as u8)
    }

    /// The opponents of `p` still in the game: a player who has lost has left it (CR 800.4a) and is no one's opponent.
    pub fn opponents(&self, p: PlayerId) -> impl Iterator<Item = PlayerId> + use<> {
        let n = self.players.len() as u8;
        let left = self.left_mask();
        (0..n).filter(move |&i| i != p.0 && left & (1 << i) == 0).map(PlayerId)
    }

    pub fn is_over(&self) -> bool {
        self.outcome != Outcome::InProgress
    }

    pub fn mark(&self) -> Mark {
        Mark(self.journal.base + self.journal.entries.len() as u32)
    }

    // --- Writing ----------------------------------------------------------------------------------------------

    fn log(&mut self, undo: Undo) {
        self.epoch += 1;
        if undo.layered() {
            self.chars_epoch += 1;
        }
        self.journal.entries.push(undo);
    }

    /// Changes one object's row.
    pub fn update_obj(&mut self, id: ObjectId, f: impl FnOnce(&mut Obj)) {
        let old = self.objects[id.index()];
        f(&mut self.objects[id.index()]);
        if self.objects[id.index()] != old {
            self.log(Undo::Object(id, Box::new(old)));
            self.set_unsettled();
        }
    }

    /// A new object, in no zone's list yet.
    pub fn push_obj(&mut self, obj: Obj) -> ObjectId {
        // A new object has no counters (CR 400.7): those it arrives with are put on it by the put path once it is here
        // (`cantrip_rules::counters`). `Obj` is `Copy`, so a copy of an object that has some would bring them in past it.
        debug_assert!(!obj.has_counters(), "a new object carries counters past the put path");
        let id = ObjectId(self.objects.len() as u32);
        self.objects.push(obj);
        self.log(Undo::ObjectPushed);
        id
    }

    /// Takes back `id`, the last object pushed, before anything has come to refer to it: an object made only to be asked
    /// about, which turned out not to be needed.
    pub fn pop_obj(&mut self, id: ObjectId) {
        assert_eq!(id.index() + 1, self.objects.len(), "only the last object pushed is taken back");
        let obj = self.objects.pop().expect("an object");
        self.log(Undo::ObjectPopped(Box::new(obj)));
    }

    pub fn update_player(&mut self, p: PlayerId, f: impl FnOnce(&mut Player)) {
        let old = self.players[p.index()];
        f(&mut self.players[p.index()]);
        if self.players[p.index()] != old {
            self.log(Undo::Player(p, old));
            self.set_unsettled();
        }
    }

    /// The restricted mana in `player`'s pool, each restriction with its mana (CR 106.6).
    pub fn restricted_of(&self, player: PlayerId) -> impl Iterator<Item = (ManaRestriction, ManaPool)> + '_ {
        self.restricted_mana.iter().filter(move |r| r.player == player).map(|r| (r.restriction, r.pool))
    }

    /// Whether `player`'s pool holds any restricted mana.
    pub fn has_restricted_mana(&self, player: PlayerId) -> bool {
        self.restricted_mana.iter().any(|r| r.player == player)
    }

    /// The mana in `player`'s pool by the source that produced it (CR 106.3), restricted or not.
    pub fn mana_from_of(&self, player: PlayerId) -> impl Iterator<Item = &ManaFrom> + '_ {
        self.mana_from.iter().filter(move |m| m.player == player)
    }

    /// Adds `n` mana of one kind that `source` produced (CR 106.3) to `player`'s pool (CR 106.4): mana `restriction` limits,
    /// if it has one (CR 106.6), kept apart; mana that lasts as `lasts` says (CR 500.5a), which no restricted mana does.
    pub fn add_mana(&mut self, player: PlayerId, mana: Mana, n: u32, source: ObjectId, restriction: Option<ManaRestriction>, lasts: ManaLasts) {
        let m = mana as usize;
        // A pool has no limit (CR 106.4); its counts are `u32`, past any number of rounds a loop is let run, and saturate
        // rather than wrap so that the sources' entries still add up to them.
        let held = match restriction {
            Some(r) => self.restricted_mana.iter().find(|e| e.player == player && e.restriction == r).map_or(0, |e| e.pool.0[m]),
            None => self.player(player).pool.0[m],
        };
        let n = n.min(u32::MAX - held);
        if n == 0 {
            return;
        }
        match restriction {
            Some(r) => self.add_restricted_mana(player, r, mana, n),
            None => self.update_player(player, |p| {
                if lasts != ManaLasts::Step {
                    p.lasting.0[m] = p.lasting_mana().0[m] + n;
                }
                p.pool.0[m] += n;
            }),
        }
        let lasts = if restriction.is_none() { lasts } else { ManaLasts::Step };
        match self.mana_from.iter().position(|e| e.player == player && e.restriction == restriction && e.source == source && e.lasts == lasts) {
            Some(at) => {
                let old = self.mana_from[at];
                self.mana_from[at].pool.0[m] += n;
                self.log(Undo::ManaFromSet(at as u32, old));
            }
            None => {
                let mut pool = ManaPool::default();
                pool.0[m] = n;
                self.mana_from.push(ManaFrom { player, restriction, source, lasts, pool });
                self.log(Undo::ManaFromPushed);
            }
        }
    }

    /// Spends up to `n` mana of one kind from `player`'s pool: of `restriction`, or ordinary mana for none, and of the
    /// sources `pick` allows - the mana that does not last before the mana that does, then in the order it came. How much
    /// there was to spend; each source's share is pushed onto `taken` (CR 106.3).
    pub fn spend_mana(
        &mut self,
        player: PlayerId,
        restriction: Option<ManaRestriction>,
        mana: Mana,
        n: u8,
        pick: ManaPick,
        taken: &mut Vec<(ObjectId, Mana, u8)>,
    ) -> u8 {
        let m = mana as usize;
        let mut left = n;
        for lasts in [ManaLasts::Step, ManaLasts::Combat, ManaLasts::Turn] {
            let mut at = 0;
            while left > 0 && at < self.mana_from.len() {
                let e = self.mana_from[at];
                if e.player == player && e.restriction == restriction && e.lasts == lasts && e.pool.0[m] > 0 && pick.takes(e.source) {
                    let take = e.pool.0[m].min(u32::from(left)) as u8;
                    left -= take;
                    taken.push((e.source, mana, take));
                    let mut after = e;
                    after.pool.0[m] -= u32::from(take);
                    if after.pool.is_empty() {
                        self.mana_from.remove(at);
                        self.log(Undo::ManaFromRemoved(at as u32, e));
                        continue;
                    }
                    self.mana_from[at] = after;
                    self.log(Undo::ManaFromSet(at as u32, e));
                }
                at += 1;
            }
        }
        let spent = n - left;
        if spent == 0 {
            return 0;
        }
        match restriction {
            Some(r) => {
                self.spend_restricted_mana(player, r, mana, spent);
            }
            None => {
                let lasting: u32 =
                    self.mana_from.iter().filter(|e| e.player == player && e.restriction.is_none() && e.lasts != ManaLasts::Step).map(|e| e.pool.0[m]).sum();
                self.update_player(player, |p| {
                    p.pool.0[m] -= u32::from(spent);
                    p.lasting.0[m] = lasting;
                });
            }
        }
        spent
    }

    /// Empties `player`'s pool as `ending` ends (CR 106.4, 500.4): a step, the combat phase or the turn - all of it but the
    /// mana that lasts past that (`ManaLasts`, CR 500.5a).
    pub fn empty_mana(&mut self, player: PlayerId, ending: ManaLasts) {
        if !self.players[player.index()].pool.is_empty() || !self.players[player.index()].lasting.is_empty() {
            let mut kept = ManaPool::default();
            for e in self.mana_from.iter().filter(|e| e.player == player && e.restriction.is_none() && e.lasts > ending) {
                for (k, n) in kept.0.iter_mut().zip(e.pool.0) {
                    *k += n;
                }
            }
            self.update_player(player, |pl| {
                pl.pool = kept;
                pl.lasting = kept;
            });
        }
        self.empty_restricted_mana(player);
        if self.mana_from.iter().any(|e| e.player == player && e.lasts <= ending) {
            let old = self.mana_from.clone();
            self.mana_from.retain(|e| e.player != player || e.lasts > ending);
            self.log(Undo::ManaFromAll(old));
        }
    }

    /// Adds `n` mana of one kind under `restriction` to `player`'s pool: the pool alone, without its source (`add_mana`).
    fn add_restricted_mana(&mut self, player: PlayerId, restriction: ManaRestriction, mana: Mana, n: u32) {
        if n == 0 {
            return;
        }
        let old = self.restricted_mana.clone();
        match self.restricted_mana.iter_mut().find(|r| r.player == player && r.restriction == restriction) {
            Some(r) => r.pool.0[mana as usize] += n,
            None => {
                let mut pool = ManaPool::default();
                pool.0[mana as usize] = n;
                self.restricted_mana.push(RestrictedMana { player, restriction, pool });
            }
        }
        self.log(Undo::RestrictedManaSet(old));
        self.set_unsettled();
    }

    /// Spends up to `n` mana of one kind under `restriction` from `player`'s pool: the pool alone (`spend_mana`).
    fn spend_restricted_mana(&mut self, player: PlayerId, restriction: ManaRestriction, mana: Mana, n: u8) -> u8 {
        let Some(at) = self.restricted_mana.iter().position(|r| r.player == player && r.restriction == restriction) else {
            return 0;
        };
        let take = self.restricted_mana[at].pool.get(mana).min(u32::from(n)) as u8;
        if take == 0 {
            return 0;
        }
        let old = self.restricted_mana.clone();
        self.restricted_mana[at].pool.0[mana as usize] -= u32::from(take);
        if self.restricted_mana[at].pool.is_empty() {
            self.restricted_mana.remove(at);
        }
        self.log(Undo::RestrictedManaSet(old));
        self.set_unsettled();
        take
    }

    /// Empties `player`'s restricted mana (CR 106.4): the pool alone (`empty_mana`).
    fn empty_restricted_mana(&mut self, player: PlayerId) {
        if !self.has_restricted_mana(player) {
            return;
        }
        let old = self.restricted_mana.clone();
        self.restricted_mana.retain(|r| r.player != player);
        self.log(Undo::RestrictedManaSet(old));
        self.set_unsettled();
    }

    pub fn update_turn(&mut self, f: impl FnOnce(&mut Turn)) {
        let old = self.turn;
        f(&mut self.turn);
        if self.turn != old {
            self.log(Undo::Turn(old));
        }
    }

    pub fn zone_insert(&mut self, zone: Zone, at: usize, id: ObjectId) {
        let slot = zone.slot(self.players.len());
        self.zones[slot].insert(at, id);
        self.log(Undo::ZoneInserted { slot: slot as u32, at: at as u32 });
        self.set_unsettled();
    }

    pub fn zone_push(&mut self, zone: Zone, id: ObjectId) {
        let at = self.zone(zone).len();
        self.zone_insert(zone, at, id);
    }

    pub fn zone_remove(&mut self, zone: Zone, id: ObjectId) -> bool {
        let slot = zone.slot(self.players.len());
        let Some(at) = self.zones[slot].iter().position(|&o| o == id) else {
            return false;
        };
        self.zones[slot].remove(at);
        self.log(Undo::ZoneRemoved { slot: slot as u32, at: at as u32, id });
        self.set_unsettled();
        true
    }

    pub fn stack_push(&mut self, mut item: StackItem) {
        // A new item is named by a timestamp (CR 115.1: abilities on the stack can be targeted); one put back after
        // others were taken off it keeps its name.
        if item.serial == 0 {
            item.serial = self.timestamp();
        }
        self.stack.push(item);
        self.log(Undo::StackPushed);
        self.set_unsettled();
    }

    pub fn stack_pop(&mut self) -> Option<StackItem> {
        let item = self.stack.pop()?;
        self.log(Undo::StackPopped(Box::new(item)));
        self.set_unsettled();
        Some(item)
    }

    pub fn stack_set(&mut self, at: usize, item: StackItem) {
        let old = self.stack[at];
        self.stack[at] = item;
        self.log(Undo::StackSet(at as u32, Box::new(old)));
    }

    /// Removes the item at `at`, keeping the order of the rest (a countered spell below the top).
    pub fn stack_remove(&mut self, at: usize) -> StackItem {
        // Popping down to it and pushing the rest back keeps the journal to its two kinds of stack entry.
        let mut above = Vec::new();
        while self.stack.len() > at + 1 {
            above.push(self.stack_pop().expect("above"));
        }
        let item = self.stack_pop().expect("the item");
        if item.kind != StackKind::Triggered {
            self.left_stack.push(item);
            self.log(Undo::LeftStackPushed);
        }
        while let Some(back) = above.pop() {
            self.stack_push(back);
        }
        item
    }

    pub fn effect_push(&mut self, effect: ContinuousEffect) {
        self.effects.push(effect);
        self.log(Undo::EffectPushed);
        self.set_unsettled();
    }

    /// Replaces the effect at `at` with `effect`, in its place: a prevention shield reduced (CR 615.7).
    pub fn effect_set(&mut self, at: usize, effect: ContinuousEffect) {
        let old = std::mem::replace(&mut self.effects[at], effect);
        self.log(Undo::EffectSet(at as u32, Box::new(old)));
        self.set_unsettled();
    }

    /// Keeps the effects at the indices `keep` marks true (one flag per effect, in order); the rest go. A copy effect
    /// that goes is undone (`effect_remove`).
    pub fn effects_keep(&mut self, keep: &[bool]) {
        debug_assert_eq!(keep.len(), self.effects.len(), "one flag per effect");
        for i in (0..self.effects.len()).rev() {
            if !keep[i] {
                self.effect_remove(i);
            }
        }
    }

    /// Removes every effect `keep` refuses, keeping the order of the rest. A copy effect that goes is undone
    /// (`effect_remove`).
    pub fn effects_retain(&mut self, mut keep: impl FnMut(&ContinuousEffect) -> bool) {
        let mut i = self.effects.len();
        while i > 0 {
            i -= 1;
            if !keep(&self.effects[i]) {
                self.effect_remove(i);
            }
        }
    }

    /// Removes the effect at `at`. Every way an effect ends comes here, so a copy effect (CR 707.2, layer 1) is undone
    /// whatever ended it - the turn's end, a "for as long as" condition (CR 611.2b), its source leaving: the permanent's
    /// copiable values are what the copy effects left on it make them, applied in timestamp order (CR 613.2a, 613.7).
    /// Each copy effect holds the values it was applied over (`EffectKind::RestoreCopy`), so the copy effects on one
    /// permanent are a chain from its own values up to `Obj::copy`: the next later one is handed the ended one's values
    /// to apply over, and with none later the permanent itself is - if it is still that permanent, on the battlefield or
    /// phased out (CR 400.7: an object that has left keeps what it last was).
    fn effect_remove(&mut self, at: usize) {
        let effect = self.effects.remove(at);
        self.log(Undo::EffectRemoved(at as u32, Box::new(effect)));
        self.set_unsettled();
        let EffectKind::RestoreCopy(prior) = effect.kind else { return };
        let above = self
            .effects
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                l.affected == effect.affected && l.timestamp > effect.timestamp && matches!(l.kind, EffectKind::RestoreCopy(_))
            })
            .min_by_key(|(_, l)| l.timestamp)
            .map(|(i, _)| i);
        match above {
            Some(i) => {
                let later = self.effects[i];
                self.effect_set(i, ContinuousEffect { kind: EffectKind::RestoreCopy(prior), ..later });
            }
            None => {
                // On the battlefield or phased out, the same object either way (CR 702.26d).
                let obj = self.obj(effect.affected);
                if obj.became.is_none() && (obj.zone.is(ZoneKind::Battlefield) || obj.zone.is(ZoneKind::PhasedOut)) {
                    self.update_obj(effect.affected, |o| o.copy = prior);
                }
            }
        }
    }

    /// The color chosen for `by`'s object (CR 607.2d), to `by`, one of its abilities: only if `by` is linked to the ability
    /// that chose it. A resolving ability asks with `Query::linked`; a static, mana or triggered ability of the object's text
    /// with `Query::linked_to`.
    pub fn chosen_color(&self, by: Linked) -> Option<Color> {
        self.obj(by.object).marks.choices.color(by)
    }

    /// The creature or land type chosen for `by`'s object, to `by` (`chosen_color`).
    pub fn chosen_subtype(&self, by: Linked) -> Option<Subtype> {
        self.obj(by.object).marks.choices.subtype(by)
    }

    /// The player chosen for `by`'s object, to `by` (`chosen_color`).
    pub fn chosen_player(&self, by: Linked) -> Option<PlayerId> {
        self.obj(by.object).marks.choices.player(by)
    }

    /// The card name chosen for `by`'s object, to `by` (`chosen_color`).
    pub fn chosen_card(&self, by: Linked) -> Option<CardId> {
        self.obj(by.object).marks.choices.card(by)
    }

    /// The word chosen for `by`'s object (CR 607.2f), to `by` (`chosen_color`).
    pub fn chosen_option(&self, by: Linked) -> Option<ChosenOption> {
        self.obj(by.object).marks.choices.option(by)
    }

    /// The object chosen for `by`'s object, to `by` (`chosen_color`), wherever it is now.
    pub fn chosen_object(&self, by: Linked) -> Option<ObjectId> {
        self.obj(by.object).marks.choices.object(by)
    }

    /// How `id` has the abilities of the text it has now (CR 607.5, 707.2): by the latest copy effect still on it, which
    /// made its copiable values what they are (`EffectKind::RestoreCopy`), or as its own. What an ability of that text is
    /// linked by (`Linked::acquired`); an ability an effect granted it is not of that text.
    pub fn copy_acquired(&self, id: ObjectId) -> Acquired {
        self.effects
            .iter()
            .filter(|e| e.affected == id && matches!(e.kind, EffectKind::RestoreCopy(_)))
            .map(|e| e.timestamp)
            .max()
            .map_or(Acquired::OWN, |stamp| Acquired { by: AcquiredBy::Copy, stamp, part: 0 })
    }

    pub fn permission_push(&mut self, permission: PlayPermission) {
        self.permissions.push(permission);
        self.log(Undo::PermissionPushed);
    }

    /// Whether a permission's "for as long as you control" duration still holds (`Expiry::WhileControls`, CR 611.2b): its
    /// holder controls that object, still the object it was (CR 400.7), on the battlefield (phased out is not, CR
    /// 702.26f). Any other expiry holds as far as this goes.
    pub fn permission_lasts(&self, p: &PlayPermission) -> bool {
        p.grant.expiry.controlled().is_none_or(|source| {
            let obj = self.obj(source);
            obj.became.is_none() && obj.zone.is(ZoneKind::Battlefield) && obj.controller == p.player
        })
    }

    /// Ends each "for as long as you control" permission whose duration no longer holds (`permission_lasts`): for good,
    /// whatever happens later (CR 611.2b). Free while there is none.
    pub fn end_lapsed_permissions(&mut self) {
        if self.permissions.iter().any(|p| !self.permission_lasts(p)) {
            let keep: Vec<bool> = self.permissions.iter().map(|p| self.permission_lasts(p)).collect();
            let mut at = keep.len();
            self.permissions_retain(|_| {
                at -= 1;
                keep[at]
            });
        }
    }

    /// Removes every permission `keep` says not to keep, keeping the order of the rest.
    pub fn permissions_retain(&mut self, mut keep: impl FnMut(&PlayPermission) -> bool) {
        let mut i = self.permissions.len();
        while i > 0 {
            i -= 1;
            if !keep(&self.permissions[i]) {
                let p = self.permissions.remove(i);
                self.log(Undo::PermissionRemoved(i as u32, p));
            }
        }
    }

    pub fn trigger_push(&mut self, trigger: PendingTrigger) {
        self.triggers.push(trigger);
        self.log(Undo::TriggerPushed);
    }

    pub fn trigger_remove(&mut self, at: usize) -> PendingTrigger {
        let trigger = self.triggers.remove(at);
        self.log(Undo::TriggerRemoved(at as u32, trigger));
        trigger
    }

    pub fn delayed_push(&mut self, delayed: Delayed) {
        self.delayed.push(delayed);
        self.log(Undo::DelayedPushed);
    }

    pub fn delayed_remove(&mut self, at: usize) -> Delayed {
        let d = self.delayed.remove(at);
        self.log(Undo::DelayedRemoved(at as u32, d));
        d
    }

    /// Keeps last-known information (CR 608.2h): an object's as it ceases to be that object, or a spell's as it becomes
    /// cast (`StackItem::cast_known`). The row it is kept in.
    pub fn last_known_push(&mut self, row: LastKnown) -> u32 {
        self.last_known.push(row);
        self.log(Undo::LastKnownPushed);
        self.last_known.len() as u32 - 1
    }

    /// Records a fact of this turn.


    /// Keeps the objects of a batch for the ability it triggered.
    pub fn members_push(&mut self, objects: &[ObjectId]) -> Span {
        let start = self.members.len();
        if objects.is_empty() {
            return Span::default();
        }
        self.members.extend_from_slice(objects);
        self.log(Undo::MembersPushed(start as u32));
        Span { start: start as u32, len: objects.len() as u32 }
    }

    /// The objects a span names.
    pub fn members(&self, span: Span) -> &[ObjectId] {
        &self.members[span.start as usize..span.start as usize + span.len as usize]
    }

    /// The run of the target arena a stack item with more than `INLINE_TARGETS` targets names.
    fn spilled(item: &StackItem) -> std::ops::Range<usize> {
        item.spill..item.spill + item.target_count as usize
    }

    /// A stack item's targets, in the order chosen (CR 601.2c): those it holds, or all of them from the arena.
    pub fn targets<'a>(&'a self, item: &'a StackItem) -> &'a [Target] {
        if item.holds_its_targets() { &item.inline_targets[..item.target_count as usize] } else { &self.spill_targets[Self::spilled(item)] }
    }

    /// For each of a stack item's targets, the spec it was chosen for, parallel to `targets`.
    pub fn spec_of<'a>(&'a self, item: &'a StackItem) -> &'a [u8] {
        if item.holds_its_targets() { &item.inline_spec_of[..item.target_count as usize] } else { &self.spill_specs[Self::spilled(item)] }
    }

    /// Each of a stack item's targets' share of a divided total, parallel to `targets`; empty for an item with none above 0.
    pub fn shares<'a>(&'a self, item: &'a StackItem) -> &'a [u32] {
        if item.shares == StackItem::NO_SHARES {
            return &[];
        }
        let start = item.shares as usize;
        &self.share_arena[start..start + item.target_count as usize]
    }

    /// A stack item's target `slot`, or `Target::None` past the targets announced.
    pub fn target(&self, item: &StackItem, slot: usize) -> Target {
        self.targets(item).get(slot).copied().unwrap_or(Target::None)
    }

    /// The spec a stack item's target `slot` was chosen for, or 0 past the targets announced.
    pub fn spec(&self, item: &StackItem, slot: usize) -> u8 {
        self.spec_of(item).get(slot).copied().unwrap_or(0)
    }

    /// What a stack item's target `slot` was announced to receive of a divided total (CR 601.2d).
    pub fn share(&self, item: &StackItem, slot: usize) -> u32 {
        self.shares(item).get(slot).copied().unwrap_or(0)
    }

    /// Where in a stack item's targets the targets chosen for the spec at `spec` are (CR 601.2c): one run, in the order chosen.
    /// Empty when that slot chose none, or it is past the targets announced (an unchosen "up to one" at the end of the list).
    pub fn spec_targets(&self, item: &StackItem, spec: usize) -> std::ops::Range<usize> {
        let of = self.spec_of(item);
        let start = of.iter().position(|&s| s as usize >= spec).unwrap_or(of.len());
        let end = start + of[start..].iter().take_while(|&&s| s as usize == spec).count();
        start..end
    }

    /// Sets a stack item's targets, the spec each was chosen for and each one's share of a divided total, all parallel: missing
    /// specs and shares are 0. Past `INLINE_TARGETS` targets the lists are put in the target arena, journaled, and the item
    /// names them (the first ones are in the item too).
    pub fn set_item_targets(&mut self, item: &mut StackItem, targets: &[Target], spec_of: &[u8], shares: &[u32]) {
        // Counted in a u32: every target is a different object or player (CR 115.3), which the event cap bounds (RX-1).
        item.target_count = crate::count_u32(targets.len());
        let n = item.target_count as usize;
        item.inline_targets = [Target::None; INLINE_TARGETS];
        item.inline_spec_of = [0; INLINE_TARGETS];
        let inline = n.min(INLINE_TARGETS);
        item.inline_targets[..inline].copy_from_slice(&targets[..inline]);
        for i in 0..inline {
            item.inline_spec_of[i] = spec_of.get(i).copied().unwrap_or(0);
        }
        item.spill = 0;
        if n > INLINE_TARGETS {
            let start = self.spill_targets.len();
            self.spill_targets.extend_from_slice(&targets[..n]);
            self.spill_specs.extend((0..n).map(|i| spec_of.get(i).copied().unwrap_or(0)));
            self.log(Undo::TargetsSpilled(start));
            item.spill = start;
        }
        // The shares, when any is above 0, in their own arena, named by a u32: as many as each divided item's targets, every
        // one chosen by hand in a game the event cap bounds (RX-1), far inside it. Past it none would be kept.
        item.shares = StackItem::NO_SHARES;
        if shares.iter().take(n).any(|&s| s > 0) {
            let start = self.share_arena.len();
            if let Some(at) = u32::try_from(start).ok().filter(|&at| at != StackItem::NO_SHARES) {
                self.share_arena.extend((0..n).map(|i| shares.get(i).copied().unwrap_or(0)));
                self.log(Undo::SharesPushed(start));
                item.shares = at;
            }
        }
    }

    /// Changes a stack item's target `slot` to `to`, keeping the rest.
    pub fn set_item_target(&mut self, item: &mut StackItem, slot: usize, to: Target) {
        if slot >= item.target_count as usize {
            return;
        }
        if item.holds_its_targets() {
            item.inline_targets[slot] = to;
            return;
        }
        let mut targets = self.targets(item).to_vec();
        targets[slot] = to;
        let (of, shares) = (self.spec_of(item).to_vec(), self.shares(item).to_vec());
        self.set_item_targets(item, &targets, &of, &shares);
    }

    /// Sets the spec each of a stack item's targets was chosen for, keeping the targets and shares.
    pub fn set_item_spec_of(&mut self, item: &mut StackItem, spec_of: &[u8]) {
        let (targets, shares) = (self.targets(item).to_vec(), self.shares(item).to_vec());
        self.set_item_targets(item, &targets, spec_of, &shares);
    }

    /// Sets each of a stack item's targets' share of a divided total, keeping the targets and specs.
    pub fn set_item_shares(&mut self, item: &mut StackItem, shares: &[u32]) {
        let (targets, of) = (self.targets(item).to_vec(), self.spec_of(item).to_vec());
        self.set_item_targets(item, &targets, &of, shares);
    }

    /// The stack item a spell last had on the stack, if it has left it (CR 608.2h).
    pub fn left_stack(&self, spell: ObjectId) -> Option<&StackItem> {
        self.left_stack.iter().rev().find(|i| i.kind == StackKind::Spell && i.object == spell)
    }

    /// The stack item an activated ability last had on the stack, named by its serial (`Target::Ability`), if it has left
    /// it (CR 608.2h, 707.10).
    pub fn left_ability(&self, serial: u32) -> Option<&StackItem> {
        self.left_stack.iter().rev().find(|i| i.kind == StackKind::Activated && i.serial == serial)
    }

    /// The characteristics an object last had where it was, once it has ceased to be that object - left any zone, or
    /// ceased to exist (CR 400.7, 608.2h, 113.7a).
    pub fn last_known(&self, id: ObjectId) -> Option<&LastKnown> {
        self.objects[id.index()].last_known.map(|row| &self.last_known[row as usize])
    }

    /// Expected-zone information for a source currently phased out (CR 608.2h, 702.26b). Its projected current
    /// characteristics are computed separately; phase-in clears this link and a later phase-out captures a new one.
    pub fn phased_last_known(&self, id: ObjectId) -> Option<&LastKnown> {
        let obj = self.obj(id);
        if obj.became.is_some() || !obj.zone.is(ZoneKind::PhasedOut) { return None; }
        obj.marks.phased.map(|p| &self.last_known[p.last_known as usize])
    }

    pub fn set_attackers(&mut self, attackers: Vec<Attacker>) {
        let old = std::mem::replace(&mut self.attackers, attackers);
        self.log(Undo::AttackersSet(old));
    }

    pub fn set_blocks(&mut self, blocks: Vec<Block>) {
        let old = std::mem::replace(&mut self.blocks, blocks);
        self.log(Undo::BlocksSet(old));
    }

    /// The timestamp keyword counters of `kind` on `object` have (CR 613.7c): that of the latest one put on it, or the
    /// object's own for those it entered with.
    pub fn counter_stamp(&self, object: ObjectId, kind: CounterKind) -> u32 {
        self.counter_stamps
            .iter()
            .find(|s| s.object == object && s.kind == kind)
            .map_or(self.obj(object).timestamp, |s| s.timestamp)
    }

    /// Gives the keyword counters of `kind` on `object` a new timestamp, as one more is put on it (CR 613.7c). The rows of
    /// objects that have since left their zone (CR 400.7) go as it is written.
    pub fn stamp_counters(&mut self, object: ObjectId, kind: CounterKind) {
        let timestamp = self.timestamp();
        let old = self.counter_stamps.clone();
        let objects = &self.objects;
        self.counter_stamps.retain(|s| objects[s.object.index()].became.is_none() && !(s.object == object && s.kind == kind));
        self.counter_stamps.push(CounterStamp { object, kind, timestamp });
        self.log(Undo::CounterStampsSet(old));
    }

    pub fn see_statics(&mut self) {
        if !self.statics_seen {
            self.statics_seen = true;
            self.log(Undo::StaticsSeen(false));
        }
    }

    pub fn set_loop_watch(&mut self, watch: LoopWatch) {
        if self.loop_watch != watch {
            let old = std::mem::replace(&mut self.loop_watch, watch);
            self.log(Undo::LoopWatchSet(old));
        }
    }

    /// The entry of the position `key` in the run of passes, made if there is none: its index. How many positions a run
    /// sees is bounded by the events the game records (`docs/rules-exceptions.md`, RX-1), far inside the `u32` that names
    /// an entry.
    pub fn loop_key_entry(&mut self, key: u64) -> usize {
        if let Some(&at) = self.loop_index.get(&key) {
            return at as usize;
        }
        self.loop_keys.push(LoopKey { key, rounds: 0, words: None });
        let at = self.loop_keys.len() - 1;
        self.loop_index.insert(key, crate::count_u32(at));
        self.log(Undo::LoopKeyPushed);
        at
    }

    /// Sets the rounds the loop has come round the position at `at`, journalling that alone.
    pub fn set_loop_rounds(&mut self, at: usize, rounds: u32) {
        let old = std::mem::replace(&mut self.loop_keys[at].rounds, rounds);
        if old != rounds {
            self.log(Undo::LoopKeyRounds(at as u32, old));
        }
    }

    /// Keeps the canonical words of the position at `at` (see `LoopKey`), journalling that alone.
    pub fn set_loop_words(&mut self, at: usize, words: Vec<u64>) {
        let old = self.loop_keys[at].words.replace(words);
        self.log(Undo::LoopKeyWords(at as u32, old));
    }

    /// Forgets every position of the run of passes.
    pub fn loop_keys_clear(&mut self) {
        if !self.loop_keys.is_empty() {
            let old = std::mem::take(&mut self.loop_keys);
            self.loop_index.clear();
            self.log(Undo::LoopKeysSet(old));
        }
    }

    pub fn set_extra_turns(&mut self, turns: Vec<ExtraTurn>) {
        let old = std::mem::replace(&mut self.extra_turns, turns);
        self.log(Undo::ExtraTurnsSet(old));
    }

    pub fn frame_push(&mut self, frame: Frame) {
        self.agenda.push(frame);
        self.log(Undo::FramePushed);
    }

    pub fn frame_pop(&mut self) -> Frame {
        let frame = self.agenda.pop().expect("a frame");
        self.log(Undo::FramePopped(frame.clone()));
        frame
    }

    /// Removes the frame at `at`, below any its run pushed.
    pub fn frame_remove(&mut self, at: usize) -> Frame {
        let frame = self.agenda.remove(at);
        self.log(Undo::FrameRemoved(at as u32, frame.clone()));
        frame
    }

    /// Changes the frame at `at`.
    pub fn update_frame(&mut self, at: usize, f: impl FnOnce(&mut Frame)) {
        let old = self.agenda[at].clone();
        f(&mut self.agenda[at]);
        if self.agenda[at] != old {
            self.log(Undo::FrameSet(at as u32, old));
        }
    }

    /// Whether the generator was drawn on since `mark` (journal entries the journal still holds): for a search's fork,
    /// whose generator is another, to know whether re-running a run begun at `mark` would draw otherwise than the game
    /// did (`cantrip_ai::search`). Reads only.
    pub fn random_since(&self, mark: Mark) -> bool {
        let from = mark.0.saturating_sub(self.journal.base) as usize;
        self.journal.entries.get(from..).is_some_and(|tail| tail.iter().any(|u| matches!(u, Undo::Rng(_))))
    }

    /// A random number below `bound`, from the state's own generator.
    pub fn random(&mut self, bound: u32) -> u32 {
        let old = self.rng;
        let r = self.rng.below(bound);
        self.log(Undo::Rng(old));
        r
    }

    /// Rolls a die with this many sides (CR 706.2), from the state's own generator so that a game replays exactly.
    pub fn roll_die(&mut self, sides: u32) -> u32 {
        let old = self.rng;
        let r = self.rng.roll(sides);
        self.log(Undo::Rng(old));
        r
    }

    /// Fixes the faces the next dice show (1 to 20, at most ten), for a scenario that must say what a die rolls.
    pub fn force_rolls(&mut self, faces: &[u8]) {
        let mut packed = 0u64;
        for (i, &face) in faces.iter().take(10).enumerate() {
            packed |= (face as u64 & 0b11111) << (5 * i);
        }
        self.rng.forced = packed;
        self.rng.forced_len = faces.len().min(10) as u8;
    }

    pub fn timestamp(&mut self) -> u32 {
        self.log(Undo::Timestamp(self.next_timestamp));
        self.next_timestamp += 1;
        self.next_timestamp - 1
    }

    /// A request id: never journalled, so never handed out twice.
    pub fn request_id(&mut self) -> u32 {
        self.next_request += 1;
        self.next_request - 1
    }

    pub fn set_outcome(&mut self, outcome: Outcome) {
        let old = self.outcome;
        self.outcome = outcome;
        self.log(Undo::Outcome(old));
    }

    pub fn set_unsettled(&mut self) {
        if !self.unsettled {
            self.unsettled = true;
            self.log(Undo::Unsettled(false));
        }
    }

    pub fn clear_unsettled(&mut self) {
        if self.unsettled {
            self.unsettled = false;
            self.log(Undo::Unsettled(true));
        }
    }

    // --- Moving through the journal ---------------------------------------------------------------------------

    /// Undoes every write since `mark`.
    /// Moves the state back to `mark` for the game-log tree (docs/engine-design.md, "The game log is the search
    /// tree"): pops and undoes every entry after it, and hands back the entries in the order they were written with,
    /// in the same order, the entries that redo them. Nothing is copied but the redo entries, made from the state as
    /// each is undone.
    /// A copy of the state and its whole journal, for one that must still be able to roll back.
    pub fn clone_with_journal(&self) -> GameState {
        let mut copy = self.clone();
        copy.journal = self.journal.deep_clone();
        copy
    }

    /// A copy of the state and the journal from `mark` on: one that can roll back as far as `mark`, and no further.
    pub fn clone_with_journal_from(&self, mark: Mark) -> GameState {
        assert!(mark.0 >= self.journal.base && mark.0 <= self.mark().0, "copying a journal from a mark it does not hold");
        let mut copy = self.clone();
        let from = (mark.0 - self.journal.base) as usize;
        copy.journal = Journal { entries: self.journal.entries[from..].to_vec(), base: mark.0, holds: self.journal.holds };
        copy
    }

    /// Replaces the generator with one from `seed`, keeping any die rolls a script has fixed, for a copy that must not
    /// draw what the game it was copied from will (a search's fork). The journal's generator entries are rewritten too,
    /// so that rolling back past a draw does not bring back the old generator.
    pub fn reseed(&mut self, seed: u64) {
        let rng = Rng { forced: self.rng.forced, forced_len: self.rng.forced_len, ..Rng::new(seed) };
        self.rng = rng;
        for entry in &mut self.journal.entries {
            if let Undo::Rng(old) = entry {
                *old = rng;
            }
        }
    }

    /// Gives the hidden objects of `chain` (one object and the rows it became from while it stayed hidden) another card,
    /// for a search's world (`cantrip_ai::determinize`): never on a real game. Not journalled, so that a rollback inside
    /// the fork keeps the world: the journal's stored copies of those rows are rewritten too, as `reseed` rewrites the
    /// generator's. `abilities` maps the cards their abilities name (the card, and its back face) to the new ones, for
    /// the stack items, triggers and delayed triggers of those objects (a face-down spell's). `statics` says whether
    /// the new card has static abilities, which the layers' guard must then not skip. The caches are made stale.
    pub fn rebind_hidden(&mut self, chain: &[ObjectId], new: CardId, abilities: &[(CardId, CardId)], statics: bool) {
        let patch = |ability: &mut AbilityRef| {
            if let Some(&(_, to)) = abilities.iter().find(|(from, _)| *from == ability.card) {
                ability.card = to;
            }
        };
        let old = self.objects[chain[0].index()].card;
        for &id in chain {
            debug_assert_eq!(self.objects[id.index()].card, old, "a chain of rows of one card");
            self.objects[id.index()].card = new;
        }
        let rebound = |id: ObjectId| chain.contains(&id);
        for item in self.stack.iter_mut().chain(self.left_stack.iter_mut()) {
            if rebound(item.object) {
                patch(&mut item.ability);
            }
        }
        for trigger in &mut self.triggers {
            if rebound(trigger.source) {
                patch(&mut trigger.ability);
            }
        }
        for delayed in &mut self.delayed {
            if rebound(delayed.source) {
                patch(&mut delayed.ability);
            }
        }
        // The stored copies, walking back from the present: an object pushed later is not there earlier, and a popped
        // one stood at the end.
        let mut len = self.objects.len();
        for entry in self.journal.entries.iter_mut().rev() {
            match entry {
                Undo::ObjectPushed => len -= 1,
                Undo::ObjectPopped(obj) => {
                    if rebound(ObjectId(len as u32)) && obj.card == old {
                        obj.card = new;
                    }
                    len += 1;
                }
                Undo::Object(id, obj) if rebound(*id) && obj.card == old => obj.card = new,
                Undo::StackPopped(item) | Undo::StackSet(_, item) | Undo::LeftStackPopped(item) if rebound(item.object) => {
                    patch(&mut item.ability)
                }
                Undo::TriggerRemoved(_, trigger) if rebound(trigger.source) => patch(&mut trigger.ability),
                Undo::DelayedRemoved(_, delayed) if rebound(delayed.source) => patch(&mut delayed.ability),
                Undo::StaticsSeen(seen) if statics => *seen = true,
                _ => {}
            }
        }
        if statics {
            self.statics_seen = true;
        }
        self.epoch += 1;
        self.chars_epoch += 1;
    }

    /// The last-known rows, kept and in the journal, of the objects of `chain` (hidden ones `rebind_hidden` deals anew) for
    /// which `shows` holds: those that show the card they were dealt from.
    fn hidden_rows_mut<'a>(&'a mut self, chain: &'a [ObjectId]) -> impl Iterator<Item = &'a mut LastKnown> + 'a {
        let popped = self.journal.entries.iter_mut().filter_map(|e| match e {
            Undo::LastKnownPopped(last) => Some(&mut **last),
            _ => None,
        });
        self.last_known.iter_mut().chain(popped).filter(move |l| chain.contains(&l.id))
    }

    /// The objects of `chain` with a last-known row, kept or in the journal, that `shows` says shows the card they are
    /// dealt anew from: what `rebind_hidden_rows` needs a row of the new card for.
    pub fn hidden_rows_showing(&self, chain: &[ObjectId], shows: impl Fn(&LastKnown) -> bool) -> Vec<ObjectId> {
        let popped = self.journal.entries.iter().filter_map(|e| match e {
            Undo::LastKnownPopped(last) => Some(&**last),
            _ => None,
        });
        let mut ids: Vec<ObjectId> = Vec::new();
        for l in self.last_known.iter().chain(popped) {
            if chain.contains(&l.id) && shows(l) && !ids.contains(&l.id) {
                ids.push(l.id);
            }
        }
        ids
    }

    /// The last-known rows of the hidden objects of `chain` once `rebind_hidden` has given them another card, for a
    /// search's world: never on a real game, and not journalled, the journal's stored copies rewritten too. A row that
    /// showed the old card (`shows`: one an object left as it moved while hidden, a library card drawn or a hand card
    /// cast, CR 400.7) becomes its object's row in `rows`, as the new card. One that showed nothing of it - a face-down
    /// object's (a face-down spell as it was cast, a face-down creature as it attacked, `Event::Attacks::known`, a card
    /// cloaked from the library as it left) - holds the world's values as much as the game's, and stays.
    pub fn rebind_hidden_rows(&mut self, chain: &[ObjectId], rows: &[(ObjectId, LastKnown)], shows: impl Fn(&LastKnown) -> bool) {
        for row in self.hidden_rows_mut(chain) {
            if shows(row) {
                match rows.iter().find(|(id, _)| *id == row.id) {
                    Some((_, l)) => *row = *l,
                    None => debug_assert!(false, "a row that shows a hidden card has a row of the card it is dealt"),
                }
            }
        }
        self.chars_epoch += 1;
    }

    /// Forgets the companion `player` chose at deck building and will reveal once the starting player is known
    /// (`Player::companion_to_reveal`), for a search's world dealt to another seat: never on a real game. Until it is
    /// revealed (CR 103.2b) nobody else knows it, so a world neither reveals it nor keeps it. Not journalled, as
    /// `restamp_hidden` is not, and the journal's stored copies of the player are rewritten too.
    pub fn forget_companion_to_reveal(&mut self, player: PlayerId) {
        self.players[player.index()].companion_to_reveal = None;
        for entry in self.journal.entries.iter_mut() {
            if let Undo::Player(p, stored) = entry
                && *p == player
            {
                stored.companion_to_reveal = None;
            }
        }
        self.epoch += 1;
    }

    /// Gives a hidden object another timestamp, for a search's world: never on a real game. Not journalled, as
    /// `rebind_hidden` is not, and the journal's stored copies of the row are rewritten too. A hidden object's timestamp
    /// is when it went where it is; one whose arrival no event records (a card of an opening hand) would otherwise tell
    /// a world in what order its zone was dealt.
    pub fn restamp_hidden(&mut self, id: ObjectId, timestamp: u32) {
        let old = self.objects[id.index()].timestamp;
        self.objects[id.index()].timestamp = timestamp;
        let mut len = self.objects.len();
        for entry in self.journal.entries.iter_mut().rev() {
            match entry {
                Undo::ObjectPushed => len -= 1,
                Undo::ObjectPopped(obj) => {
                    if len == id.index() && obj.timestamp == old {
                        obj.timestamp = timestamp;
                    }
                    len += 1;
                }
                Undo::Object(at, obj) if *at == id && obj.timestamp == old => obj.timestamp = timestamp,
                _ => {}
            }
        }
        self.epoch += 1;
        self.chars_epoch += 1;
    }

    /// Puts a library's objects in another order, for a search's world: never on a real game. Not journalled, as
    /// `rebind_hidden` is not. The journal's tail keeps up: a removal it records names the object it puts back, and that
    /// name is changed as the object's place is, so that a rollback inside the fork lays the library out in the new order
    /// at every point. An object made in the tail (`zone_order_fixed`) keeps its place: a rollback takes away by place.
    pub fn reorder_library(&mut self, slot: usize, order: Vec<ObjectId>) {
        let fixed = self.zone_order_fixed(slot);
        debug_assert!({
            let mut a = order.clone();
            let mut b = self.zones[slot].clone();
            a.sort();
            b.sort();
            a == b && self.zones[slot].iter().zip(&order).all(|(was, now)| was == now || !(fixed.contains(was) || fixed.contains(now)))
        });
        // The object now at each place stands, through the tail, for the one that was there.
        let moved: Vec<(ObjectId, ObjectId)> = self.zones[slot].iter().zip(&order).filter(|(was, now)| was != now).map(|(&was, &now)| (was, now)).collect();
        if !moved.is_empty() {
            for entry in &mut self.journal.entries {
                if let Undo::ZoneRemoved { slot: s, id, .. } = entry
                    && *s as usize == slot
                    && let Some(&(_, now)) = moved.iter().find(|(was, _)| was == id)
                {
                    *id = now;
                }
            }
        }
        self.zones[slot] = order;
        self.epoch += 1;
        self.chars_epoch += 1;
    }

    /// The objects of a zone slot that the journal's tail made: a rollback inside the tail takes them away by their place,
    /// so a reorder keeps them where they are (`reorder_library`).
    pub fn zone_order_fixed(&self, slot: usize) -> Vec<ObjectId> {
        let mut len = self.objects.len();
        for entry in self.journal.entries.iter().rev() {
            match entry {
                Undo::ObjectPushed => len -= 1,
                Undo::ObjectPopped(_) => len += 1,
                _ => {}
            }
        }
        self.zones[slot].iter().copied().filter(|id| id.index() >= len).collect()
    }

    pub fn step_back(&mut self, mark: Mark) -> (Vec<Undo>, Vec<Undo>) {
        assert!(mark.0 >= self.journal.base, "stepping back past what the journal still holds");
        let mut undos = Vec::new();
        let mut redos = Vec::new();
        while self.journal.base + self.journal.entries.len() as u32 > mark.0 {
            let undo = self.journal.entries.pop().expect("entry");
            redos.push(self.inverse(&undo));
            undos.push(undo.clone());
            self.undo(undo);
        }
        undos.reverse();
        redos.reverse();
        (undos, redos)
    }

    /// The reverse of `step_back`: reapplies the redo entries in order and puts the original entries back on the
    /// journal, so the state and its journal are as they were before stepping back.
    pub fn step_forward(&mut self, undos: Vec<Undo>, redos: Vec<Undo>) {
        for redo in redos {
            self.undo(redo);
        }
        self.journal.entries.extend(undos);
    }

    pub fn rollback(&mut self, mark: Mark) {
        assert!(mark.0 >= self.journal.base, "rolling back past what the journal still holds");
        while self.journal.base + self.journal.entries.len() as u32 > mark.0 {
            let undo = self.journal.entries.pop().expect("entry");
            self.undo(undo);
        }
    }

    fn undo(&mut self, undo: Undo) {
        self.epoch += 1;
        if undo.layered() {
            self.chars_epoch += 1;
        }
        match undo {
            Undo::Object(id, old) => self.objects[id.index()] = *old,
            Undo::ObjectPushed => {
                self.objects.pop();
            }
            Undo::ObjectPopped(obj) => self.objects.push(*obj),
            Undo::Player(p, old) => self.players[p.index()] = old,
            Undo::Turn(old) => self.turn = old,
            Undo::RestrictedManaSet(old) => self.restricted_mana = old,
            Undo::ManaFromPushed => {
                self.mana_from.pop();
            }
            Undo::ManaFromSet(at, old) => self.mana_from[at as usize] = old,
            Undo::ManaFromRemoved(at, old) => self.mana_from.insert(at as usize, old),
            Undo::ManaFromInserted(at) => {
                self.mana_from.remove(at as usize);
            }
            Undo::ManaFromAll(old) => self.mana_from = old,
            Undo::CounterStampsSet(old) => self.counter_stamps = old,
            Undo::ZoneInserted { slot, at } => {
                self.zones[slot as usize].remove(at as usize);
            }
            Undo::ZoneRemoved { slot, at, id } => self.zones[slot as usize].insert(at as usize, id),
            Undo::StackPushed => {
                self.stack.pop();
            }
            Undo::StackPopped(item) => self.stack.push(*item),
            Undo::StackSet(at, old) => self.stack[at as usize] = *old,
            Undo::EffectPushed => {
                self.effects.pop();
            }
            Undo::EffectRemoved(at, effect) => self.effects.insert(at as usize, *effect),
            Undo::EffectInserted(at) => {
                self.effects.remove(at as usize);
            }
            Undo::EffectSet(at, old) => self.effects[at as usize] = *old,
            Undo::PermissionPushed => {
                self.permissions.pop();
            }
            Undo::PermissionRemoved(at, p) => self.permissions.insert(at as usize, p),
            Undo::PermissionInserted(at) => {
                self.permissions.remove(at as usize);
            }
            Undo::TriggerPushed => {
                self.triggers.pop();
            }
            Undo::TriggerRemoved(at, trigger) => self.triggers.insert(at as usize, trigger),
            Undo::TriggerInserted(at) => {
                self.triggers.remove(at as usize);
            }
            Undo::DelayedPushed => {
                self.delayed.pop();
            }
            Undo::DelayedRemoved(at, d) => self.delayed.insert(at as usize, d),
            Undo::DelayedInserted(at) => {
                self.delayed.remove(at as usize);
            }
            Undo::LastKnownPushed => {
                self.last_known.pop();
            }
            Undo::LastKnownPopped(row) => self.last_known.push(*row),
            Undo::LeftStackPushed => {
                self.left_stack.pop();
            }
            Undo::LeftStackPopped(item) => self.left_stack.push(*item),
            Undo::MembersPushed(len) => self.members.truncate(len as usize),
            Undo::MembersPopped(objects) => self.members.extend(objects),
            Undo::TargetsSpilled(len) => {
                self.spill_targets.truncate(len);
                self.spill_specs.truncate(len);
            }
            Undo::SharesPushed(len) => self.share_arena.truncate(len),
            Undo::SharesPopped(shares) => self.share_arena.extend(shares),
            Undo::TargetsUnspilled(targets, specs) => {
                self.spill_targets.extend(targets);
                self.spill_specs.extend(specs);
            }
            Undo::AttackersSet(old) => self.attackers = old,
            Undo::BlocksSet(old) => self.blocks = old,
            Undo::ExtraTurnsSet(old) => self.extra_turns = old,
            Undo::LoopWatchSet(old) => self.loop_watch = old,
            Undo::LoopKeyPushed => {
                if let Some(key) = self.loop_keys.pop() {
                    self.loop_index.remove(&key.key);
                }
            }
            Undo::LoopKeyPopped(key) => {
                self.loop_index.insert(key.key, self.loop_keys.len() as u32);
                self.loop_keys.push(key);
            }
            Undo::LoopKeyRounds(at, old) => self.loop_keys[at as usize].rounds = old,
            Undo::LoopKeyWords(at, old) => self.loop_keys[at as usize].words = old,
            Undo::LoopKeysSet(old) => {
                self.loop_index = old.iter().enumerate().map(|(at, k)| (k.key, at as u32)).collect();
                self.loop_keys = old;
            }
            Undo::StaticsSeen(old) => self.statics_seen = old,
            Undo::FramePushed => {
                self.agenda.pop();
            }
            Undo::FramePopped(frame) => self.agenda.push(frame),
            Undo::FrameRemoved(at, frame) => self.agenda.insert(at as usize, frame),
            Undo::FrameInserted(at) => {
                self.agenda.remove(at as usize);
            }
            Undo::FrameSet(at, old) => self.agenda[at as usize] = old,
            Undo::Rng(old) => self.rng = old,
            Undo::Timestamp(old) => self.next_timestamp = old,
            Undo::Outcome(old) => self.outcome = old,
            Undo::Unsettled(old) => self.unsettled = old,
        }
    }

    /// Undoes every write since `mark` as new writes, each journalled with its own inverse, so that the history before
    /// is kept: a later rollback to a point before `mark` puts back what this undid. This is what re-running a frame
    /// uses, since a search may hold a checkpoint inside the run being undone.
    pub fn revert(&mut self, mark: Mark) {
        assert!(mark.0 >= self.journal.base, "reverting past what the journal still holds");
        let mut at = (mark.0 - self.journal.base) as usize;
        let end = self.journal.entries.len();
        let undos: Vec<Undo> = self.journal.entries[at..end].iter().rev().cloned().collect();
        for undo in undos {
            let inverse = self.inverse(&undo);
            self.undo(undo);
            self.journal.entries.push(inverse);
            at += 1;
        }
        let _ = at;
    }

    /// The entry that undoes applying `undo` to the state as it is now.
    fn inverse(&self, undo: &Undo) -> Undo {
        match undo {
            Undo::Object(id, _) => Undo::Object(*id, Box::new(self.objects[id.index()])),
            Undo::ObjectPushed => Undo::ObjectPopped(Box::new(*self.objects.last().expect("an object"))),
            Undo::ObjectPopped(_) => Undo::ObjectPushed,
            Undo::Player(p, _) => Undo::Player(*p, self.players[p.index()]),
            Undo::Turn(_) => Undo::Turn(self.turn),
            Undo::RestrictedManaSet(_) => Undo::RestrictedManaSet(self.restricted_mana.clone()),
            Undo::ManaFromPushed => {
                Undo::ManaFromRemoved(self.mana_from.len() as u32 - 1, *self.mana_from.last().expect("mana"))
            }
            Undo::ManaFromSet(at, _) => Undo::ManaFromSet(*at, self.mana_from[*at as usize]),
            Undo::ManaFromRemoved(at, _) => Undo::ManaFromInserted(*at),
            Undo::ManaFromInserted(at) => Undo::ManaFromRemoved(*at, self.mana_from[*at as usize]),
            Undo::ManaFromAll(_) => Undo::ManaFromAll(self.mana_from.clone()),
            Undo::CounterStampsSet(_) => Undo::CounterStampsSet(self.counter_stamps.clone()),
            Undo::ZoneInserted { slot, at } => {
                Undo::ZoneRemoved { slot: *slot, at: *at, id: self.zones[*slot as usize][*at as usize] }
            }
            Undo::ZoneRemoved { slot, at, .. } => Undo::ZoneInserted { slot: *slot, at: *at },
            Undo::StackPushed => Undo::StackPopped(Box::new(*self.stack.last().expect("an item"))),
            Undo::StackPopped(_) => Undo::StackPushed,
            Undo::StackSet(at, _) => Undo::StackSet(*at, Box::new(self.stack[*at as usize])),
            Undo::EffectPushed => Undo::EffectRemoved(self.effects.len() as u32 - 1, Box::new(*self.effects.last().expect("an effect"))),
            Undo::EffectRemoved(at, _) => Undo::EffectInserted(*at),
            Undo::EffectInserted(at) => Undo::EffectRemoved(*at, Box::new(self.effects[*at as usize])),
            Undo::EffectSet(at, _) => Undo::EffectSet(*at, Box::new(self.effects[*at as usize])),
            Undo::PermissionPushed => {
                Undo::PermissionRemoved(self.permissions.len() as u32 - 1, *self.permissions.last().expect("a permission"))
            }
            Undo::PermissionRemoved(at, _) => Undo::PermissionInserted(*at),
            Undo::PermissionInserted(at) => Undo::PermissionRemoved(*at, self.permissions[*at as usize]),
            Undo::TriggerPushed => Undo::TriggerRemoved(self.triggers.len() as u32 - 1, *self.triggers.last().expect("a trigger")),
            Undo::TriggerRemoved(at, _) => Undo::TriggerInserted(*at),
            Undo::TriggerInserted(at) => Undo::TriggerRemoved(*at, self.triggers[*at as usize]),
            Undo::DelayedPushed => Undo::DelayedRemoved(self.delayed.len() as u32 - 1, *self.delayed.last().expect("delayed")),
            Undo::DelayedRemoved(at, _) => Undo::DelayedInserted(*at),
            Undo::DelayedInserted(at) => Undo::DelayedRemoved(*at, self.delayed[*at as usize]),
            Undo::LastKnownPushed => Undo::LastKnownPopped(Box::new(*self.last_known.last().expect("a row"))),
            Undo::LastKnownPopped(_) => Undo::LastKnownPushed,
            Undo::LeftStackPushed => Undo::LeftStackPopped(Box::new(*self.left_stack.last().expect("an item"))),
            Undo::LeftStackPopped(_) => Undo::LeftStackPushed,
            Undo::MembersPushed(len) => Undo::MembersPopped(self.members[*len as usize..].to_vec()),
            Undo::MembersPopped(_) => Undo::MembersPushed(self.members.len() as u32),
            Undo::TargetsSpilled(len) => Undo::TargetsUnspilled(
                self.spill_targets[*len..].to_vec(),
                self.spill_specs[*len..].to_vec(),
            ),
            Undo::SharesPushed(len) => Undo::SharesPopped(self.share_arena[*len..].to_vec()),
            Undo::SharesPopped(_) => Undo::SharesPushed(self.share_arena.len()),
            Undo::TargetsUnspilled(..) => Undo::TargetsSpilled(self.spill_targets.len()),
            Undo::AttackersSet(_) => Undo::AttackersSet(self.attackers.clone()),
            Undo::BlocksSet(_) => Undo::BlocksSet(self.blocks.clone()),
            Undo::ExtraTurnsSet(_) => Undo::ExtraTurnsSet(self.extra_turns.clone()),
            Undo::LoopWatchSet(_) => Undo::LoopWatchSet(self.loop_watch.clone()),
            Undo::LoopKeyPushed => Undo::LoopKeyPopped(self.loop_keys.last().expect("a key").clone()),
            Undo::LoopKeyPopped(_) => Undo::LoopKeyPushed,
            Undo::LoopKeyRounds(at, _) => Undo::LoopKeyRounds(*at, self.loop_keys[*at as usize].rounds),
            Undo::LoopKeyWords(at, _) => Undo::LoopKeyWords(*at, self.loop_keys[*at as usize].words.clone()),
            Undo::LoopKeysSet(_) => Undo::LoopKeysSet(self.loop_keys.clone()),
            Undo::StaticsSeen(_) => Undo::StaticsSeen(self.statics_seen),
            Undo::FramePushed => Undo::FramePopped(self.agenda.last().expect("a frame").clone()),
            Undo::FramePopped(_) => Undo::FramePushed,
            Undo::FrameRemoved(at, _) => Undo::FrameInserted(*at),
            Undo::FrameInserted(at) => Undo::FrameRemoved(*at, self.agenda[*at as usize].clone()),
            Undo::FrameSet(at, _) => Undo::FrameSet(*at, self.agenda[*at as usize].clone()),
            Undo::Rng(_) => Undo::Rng(self.rng),
            Undo::Timestamp(_) => Undo::Timestamp(self.next_timestamp),
            Undo::Outcome(_) => Undo::Outcome(self.outcome),
            Undo::Unsettled(_) => Undo::Unsettled(self.unsettled),
        }
    }

    /// Forgets the journal before `mark`, when nothing can roll back past it.
    pub fn forget_before(&mut self, mark: Mark) {
        let drop = mark.0.saturating_sub(self.journal.base).min(self.journal.entries.len() as u32);
        self.journal.entries.drain(..drop as usize);
        self.journal.base += drop;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> StackItem {
        StackItem {
            kind: StackKind::Spell,
            object: ObjectId(1),
            ability: AbilityRef { card: CardId(0), index: 0 },
            controller: PlayerId(0),
            inline_targets: [Target::None; INLINE_TARGETS],
            target_count: 0,
            inline_spec_of: [0; INLINE_TARGETS],
            x: 0,
            kicked: 0,
            counted: 0,
            paid: 0,
            way: 0,
            modes: 0,
            choice: 0,
            subject: Target::None,
            data: [0; 2],
            members: Span::default(),
            serial: 0,
            gift: None,
            shares: StackItem::NO_SHARES,
            spill: 0,
            exile_instead: ExileInstead::Never,
            flashback: false,
            attacking: false,
            since: 0,
            copied: false,
            cast_known: u32::MAX,
            acquired: Acquired::OWN,
        }
    }

    /// A library reordered for a search's world, after the journal's tail took cards from it and put them back: rolling the
    /// tail back lays the library out in the new order, every object once, and an object the tail made keeps its place.
    #[test]
    fn a_reordered_library_rolls_back_in_its_new_order() {
        let mut s = GameState::new(2, 1);
        let library = Zone::library(PlayerId(0));
        let ids: Vec<ObjectId> = (0..6).map(|_| s.push_obj(Obj::new(CardId(0), PlayerId(0), library))).collect();
        for &id in &ids {
            s.zone_push(library, id);
        }
        let start = s.mark();
        // The tail: the top two taken out and put back (a run that stopped to ask), one taken out for good, and a new
        // object put on the bottom.
        s.zone_remove(library, ids[0]);
        s.zone_remove(library, ids[1]);
        s.zone_insert(library, 0, ids[1]);
        s.zone_insert(library, 0, ids[0]);
        s.zone_remove(library, ids[5]);
        let made = s.push_obj(Obj::new(CardId(0), PlayerId(0), library));
        s.zone_push(library, made);
        // A fork's journal begins where the tail does.
        let mut fork = s.clone_with_journal_from(start);
        let slot = library.slot(2);
        assert_eq!(fork.zone_order_fixed(slot), vec![made]);
        // Now: 0 1 2 3 4 made. Reordered: 3 0 4 1 2 made.
        let order = vec![ids[3], ids[0], ids[4], ids[1], ids[2], made];
        fork.reorder_library(slot, order);
        fork.rollback(start);
        // At the start the real order was 0 1 2 3 4 5; each object stands where the one it replaced stood.
        assert_eq!(fork.zone(library), &[ids[3], ids[0], ids[4], ids[1], ids[2], ids[5]][..]);
    }

    /// Past `INLINE_TARGETS` a stack item's targets and specs are in the game's arena, and its shares in theirs, read back whole; changing one
    /// keeps the rest; a short list again holds nothing past it; and rolling the journal back frees what was spilled.
    #[test]
    fn a_stack_item_holds_more_targets_than_it_holds_in_itself_in_the_games_arena() {
        let mut s = GameState::new(2, 1);
        let before = s.mark();
        let n = INLINE_TARGETS + 4;
        let targets: Vec<Target> = (0..n as u32).map(|i| Target::Object(ObjectId(i + 10))).collect();
        let of: Vec<u8> = (0..n).map(|i| (i / 10) as u8).collect();
        let shares: Vec<u32> = (0..n).map(|i| 300 + i as u32).collect();
        let mut a = item();
        s.set_item_targets(&mut a, &targets, &of, &shares);
        assert_eq!(s.targets(&a), &targets[..]);
        assert_eq!(s.spec_of(&a), &of[..]);
        assert_eq!(s.share(&a, n - 1), 300 + (n - 1) as u32);
        assert_eq!(s.spec_targets(&a, 1), 10..n);
        assert_eq!(a.target(0), targets[0], "the first targets are in the item too");
        s.set_item_target(&mut a, n - 1, Target::Player(PlayerId(1)));
        assert_eq!(s.target(&a, n - 1), Target::Player(PlayerId(1)));
        assert_eq!(s.target(&a, 0), targets[0]);
        assert_eq!(s.spec(&a, n - 1), of[n - 1]);
        let mut b = item();
        s.set_item_targets(&mut b, &targets[..2], &[0, 0], &[]);
        assert!(b.holds_its_targets());
        assert_eq!(s.targets(&b), &targets[..2]);
        assert_eq!(s.target(&b, 5), Target::None);
        assert!(!s.spill_targets.is_empty());
        s.rollback(before);
        assert!(s.spill_targets.is_empty() && s.spill_specs.is_empty() && s.share_arena.is_empty(), "the arena outlived its journal");
    }

    /// Lists longer than 2^16 - a zone of 70,000 tokens, a batch of 70,000 objects - are reachable under the event cap
    /// (RX-1): a batch keeps its whole length, and a write at a position past 65,535 is rolled back at that position.
    #[test]
    fn a_list_past_65535_entries_keeps_its_length_and_rolls_back_exactly() {
        const N: u32 = 70_000;
        let mut s = GameState::new(2, 1);
        let ids: Vec<ObjectId> = (0..N).map(ObjectId).collect();
        let span = s.members_push(&ids);
        assert_eq!(s.members(span).len(), N as usize);
        let field = Zone::BATTLEFIELD;
        for &id in &ids {
            s.zone_push(field, id);
        }
        let before = s.clone_with_journal();
        let mark = s.mark();
        assert!(s.zone_remove(field, ObjectId(N - 2)));
        s.zone_insert(field, 66_000, ObjectId(N - 2));
        assert_eq!(s.zone(field)[66_000], ObjectId(N - 2));
        s.rollback(mark);
        assert!(s == before, "a write past 65,535 was not undone where it was made");
    }

    /// A pool has no limit (CR 106.4): 300 black mana added one at a time, from 300 sources and from one, is all there,
    /// the sources' shares adding up to it, and what is spent comes out of both.
    #[test]
    fn a_pool_holds_more_than_255_of_a_kind() {
        let mut s = GameState::new(2, 1);
        let p = PlayerId(0);
        for i in 0..300u32 {
            s.add_mana(p, Mana::B, 1, ObjectId(i), None, ManaLasts::Step);
        }
        for _ in 0..300u32 {
            s.add_mana(p, Mana::B, 1, ObjectId(1000), None, ManaLasts::Step);
        }
        assert_eq!(s.player(p).pool.get(Mana::B), 600);
        let shares: u32 = s.mana_from_of(p).map(|e| e.pool.get(Mana::B)).sum();
        assert_eq!(shares, 600);
        let mut taken = Vec::new();
        assert_eq!(s.spend_mana(p, None, Mana::B, 200, ManaPick::Any, &mut taken), 200);
        assert_eq!(s.player(p).pool.get(Mana::B), 400);
        assert_eq!(s.mana_from_of(p).map(|e| e.pool.get(Mana::B)).sum::<u32>(), 400);
    }

    /// Mana that lasts (CR 500.5a) stays through the endings it outlasts and no further: step mana empties at any ending, combat
    /// mana at the end of combat and of the turn, turn mana only at the end of the turn; and mana is spent step first, then
    /// combat, then turn, so the longest-lasting mana is the last used.
    #[test]
    fn mana_that_lasts_empties_at_the_ending_it_does_not_outlast_and_is_spent_last() {
        let mut s = GameState::new(2, 1);
        let p = PlayerId(0);
        let src = ObjectId(1);
        let pool = |s: &GameState| (s.player(p).pool.get(Mana::R), s.player(p).pool.get(Mana::G), s.player(p).pool.get(Mana::B));
        let add = |s: &mut GameState| {
            s.add_mana(p, Mana::R, 1, src, None, ManaLasts::Step);
            s.add_mana(p, Mana::G, 2, src, None, ManaLasts::Combat);
            s.add_mana(p, Mana::B, 3, src, None, ManaLasts::Turn);
        };
        add(&mut s);
        assert_eq!(pool(&s), (1, 2, 3));
        s.empty_mana(p, ManaLasts::Step);
        assert_eq!(pool(&s), (0, 2, 3));
        s.empty_mana(p, ManaLasts::Combat);
        assert_eq!(pool(&s), (0, 0, 3));
        s.empty_mana(p, ManaLasts::Turn);
        assert_eq!(pool(&s), (0, 0, 0));
        assert_eq!(s.mana_from_of(p).count(), 0);

        // One kind of mana of each duration: spending two takes the step's, then the combat's, leaving the turn's.
        s.add_mana(p, Mana::R, 1, src, None, ManaLasts::Turn);
        s.add_mana(p, Mana::R, 1, src, None, ManaLasts::Combat);
        s.add_mana(p, Mana::R, 1, src, None, ManaLasts::Step);
        let mut taken = Vec::new();
        assert_eq!(s.spend_mana(p, None, Mana::R, 2, ManaPick::Any, &mut taken), 2);
        s.empty_mana(p, ManaLasts::Combat);
        assert_eq!(s.player(p).pool.get(Mana::R), 1);
        s.empty_mana(p, ManaLasts::Turn);
        assert_eq!(s.player(p).pool.get(Mana::R), 0);
    }
}
