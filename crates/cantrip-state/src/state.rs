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
    pub attackers: Vec<Attacker>,
    pub blocks: Vec<Block>,
    /// Extra turns waiting to be taken (CR 500.7): the most recently created last, and taken first.
    pub extra_turns: Vec<ExtraTurn>,
    /// What the rules watch to find a loop of mandatory actions (CR 104.4b, 732.4): see `LoopWatch`.
    pub loop_watch: LoopWatch,
    /// The basic land cards a Limited event supplies (Magic Tournament Rules: a Limited sideboard holds any number of basic
    /// lands), of which each player always has one more outside the game (`Zone::sideboard`): one brought into the game is
    /// replaced there. Empty for a constructed game. Fixed at the deal, so no part of the journal.
    pub outside_supply: Vec<CardId>,
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
    /// Counts every write and every undo, never going back: two reads of a state at one `epoch` see the same position.
    /// Derived data below is valid for the epoch it was made at, so it is journal-safe by construction. No part of the
    /// position.
    pub epoch: u64,
    /// The permanents whose static abilities could apply to anything, found by the layers (a function of the position).
    pub static_sources: std::cell::RefCell<StaticSources>,
    /// The permanents that have lost all their abilities to a static ability (CR 613.1f), found by the layers for one
    /// position and history (a function of both: such an ability may read the past).
    pub lost_to_statics: std::cell::RefCell<LostToStatics>,
    /// The lands that have lost the abilities of their rules text to an effect that sets their subtype (CR 305.7), found by
    /// the layers for one position and history, as `lost_to_statics` is (`applying` unused: a land's own such ability keeps
    /// applying and is not counted).
    pub land_text_lost: std::cell::RefCell<LostToStatics>,
}

/// What the rules watch to find a loop (CR 104.4b, 732): `cantrip_rules::loops` is what they do with it. Empty, but for the
/// step it is in, in nearly every position.
///
/// A run of mandatory actions: `forced` counts the priorities given, with something on the stack, since a player last had a
/// real choice (a question with more than one legal answer); once it is high enough, each such priority's position is
/// fingerprinted into `seen`, and a position seen again with no real choice made and the history not asked in between is a
/// loop no player can stop: the game is a draw. Any real choice clears them, and a question asked of the history clears
/// `seen`. `far` is what the players' life totals, poison and libraries were halfway to the longest such run the rules let
/// go on.
///
/// A step with a great deal in it: from `step_log` (the history's length as the step `step` began) on, each question's
/// position is kept in `keys` with the answers given to it, and `round` follows a position at priority that comes round
/// again: `involved` is the players who chose to act since it last did, `designated` those who must now choose otherwise
/// (CR 732.3). `asking` is the request waiting and its key, `priority_key` the key of the priority about to be asked, and
/// `acted` the players who chose to act at the priority being played out with its answer, not yet among `involved`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LoopWatch {
    pub forced: u32,
    pub seen: Vec<u64>,
    pub far: Vec<i32>,
    pub step: u32,
    pub step_log: u32,
    pub keys: Vec<LoopKey>,
    pub round: Option<LoopRound>,
    pub involved: u8,
    pub designated: u8,
    pub asking: Option<(u32, u64)>,
    pub priority_key: Option<u64>,
    pub acted: u8,
}

/// A question asked in a position (its shape, by `cantrip_rules::loops`): how many times, and the answers given to it there.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LoopKey {
    pub key: u64,
    pub asked: u16,
    pub given: Vec<u64>,
}

/// A position at priority that has come round again: how many times since the players last acted differently (`rounds`),
/// how many of those in a row nobody chose to act (`passes`), the priorities since it last came round (`since`), and the
/// amounts of things as it last did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LoopRound {
    pub key: u64,
    pub rounds: u16,
    pub passes: u16,
    pub since: u16,
    pub amounts: Vec<(u32, i32)>,
}

/// The layers' list of the permanents a "loses all abilities" static ability took everything from, made for one epoch
/// of the state and one length of the history.
#[derive(Clone, Debug)]
pub struct LostToStatics {
    pub key: (u64, usize),
    pub objects: Vec<ObjectId>,
    /// The sources whose "loses all abilities" abilities apply: one among `objects` that took its own abilities keeps
    /// applying its static abilities, which had begun to apply (CR 613.6).
    pub applying: Vec<ObjectId>,
    /// Being worked out: a question asked from inside the working out is answered without it.
    pub finding: bool,
}

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
    /// Whether a source of `list` has a static ability that continues an effect begun in an earlier layer
    /// (`StaticDef::continues`, CR 613.6): only then do the layers note which effects began to apply to an object.
    pub continuing: bool,
}

impl GameState {
    fn position(
        &self,
    ) -> (
        (&Vec<Player>, &Vec<RestrictedMana>, &Vec<CounterStamp>, &Vec<ManaFrom>),
        &Vec<Obj>,
        &Vec<Vec<ObjectId>>,
        &Vec<StackItem>,
        &Turn,
        &Vec<ContinuousEffect>,
        (&Vec<PendingTrigger>, &Vec<Delayed>, &Vec<LastKnown>, &Vec<PlayPermission>),
        (&Vec<Attacker>, &Vec<Block>, &Vec<ExtraTurn>, &LoopWatch),
        &Vec<Frame>,
        (&Rng, u32, &Outcome, bool, &Vec<ObjectId>, &Vec<StackItem>),
    ) {
        (
            (&self.players, &self.restricted_mana, &self.counter_stamps, &self.mana_from),
            &self.objects,
            &self.zones,
            &self.stack,
            &self.turn,
            &self.effects,
            (&self.triggers, &self.delayed, &self.last_known, &self.permissions),
            (&self.attackers, &self.blocks, &self.extra_turns, &self.loop_watch),
            &self.agenda,
            (&self.rng, self.next_timestamp, &self.outcome, self.unsettled, &self.members, &self.left_stack),
        )
    }
}

/// Two states are the same position whatever their journals and request counters hold.
impl PartialEq for GameState {
    fn eq(&self, other: &GameState) -> bool {
        self.position() == other.position()
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

/// How to undo one write.
#[derive(Clone, Debug)]
pub enum Undo {
    Object(ObjectId, Box<Obj>),
    ObjectPushed,
    ObjectPopped(Box<Obj>),
    Player(PlayerId, Player),
    RestrictedManaSet(Vec<RestrictedMana>),
    ManaFromPushed,
    ManaFromSet(u16, ManaFrom),
    ManaFromRemoved(u16, ManaFrom),
    ManaFromInserted(u16),
    ManaFromAll(Vec<ManaFrom>),
    CounterStampsSet(Vec<CounterStamp>),
    Turn(Turn),
    ZoneInserted { slot: u16, at: u16 },
    ZoneRemoved { slot: u16, at: u16, id: ObjectId },
    StackPushed,
    StackPopped(Box<StackItem>),
    StackSet(u16, Box<StackItem>),
    EffectPushed,
    EffectRemoved(u16, Box<ContinuousEffect>),
    EffectInserted(u16),
    EffectSet(u16, Box<ContinuousEffect>),
    PermissionPushed,
    PermissionRemoved(u16, PlayPermission),
    PermissionInserted(u16),
    TriggerPushed,
    TriggerRemoved(u16, PendingTrigger),
    TriggerInserted(u16),
    DelayedPushed,
    DelayedRemoved(u16, Delayed),
    DelayedInserted(u16),
    LastKnownPushed,
    LastKnownPopped(Box<LastKnown>),
    LeftStackPushed,
    LeftStackPopped(Box<StackItem>),
    /// Members appended since the list was this long.
    MembersPushed(u32),
    MembersPopped(Vec<ObjectId>),
    AttackersSet(Vec<Attacker>),
    BlocksSet(Vec<Block>),
    ExtraTurnsSet(Vec<ExtraTurn>),
    LoopWatchSet(LoopWatch),
    StaticsSeen(bool),
    FramePushed,
    FramePopped(Frame),
    FrameRemoved(u16, Frame),
    FrameInserted(u16),
    FrameSet(u16, Frame),
    Rng(Rng),
    Timestamp(u32),
    Outcome(Outcome),
    Unsettled(bool),
}

/// A point in the journal to come back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark(pub u32);

impl GameState {
    pub fn new(players: usize, seed: u64) -> GameState {
        GameState {
            players: vec![Player { life: 20, starting_life: 20, ..Player::default() }; players],
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
            attackers: Vec::new(),
            blocks: Vec::new(),
            extra_turns: Vec::new(),
            loop_watch: LoopWatch::default(),
            outside_supply: Vec::new(),
            statics_seen: false,
            agenda: Vec::new(),
            rng: Rng::new(seed),
            next_timestamp: 1,
            next_request: 1,
            outcome: Outcome::InProgress,
            unsettled: true,
            journal: Journal::default(),
            epoch: 0,
            static_sources: std::cell::RefCell::new(StaticSources { epoch: u64::MAX, list: Vec::new(), strippers: Vec::new(), setters: Vec::new(), controllers: Vec::new(), honed: Vec::new(), granted: Vec::new(), continuing: false }),
            lost_to_statics: std::cell::RefCell::new(LostToStatics { key: (u64::MAX, 0), objects: Vec::new(), applying: Vec::new(), finding: false }),
            land_text_lost: std::cell::RefCell::new(LostToStatics { key: (u64::MAX, 0), objects: Vec::new(), applying: Vec::new(), finding: false }),
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
        let id = ObjectId(self.objects.len() as u32);
        self.objects.push(obj);
        self.log(Undo::ObjectPushed);
        id
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
    /// if it has one (CR 106.6), kept apart; mana that lasts until end of combat if `lasting` (CR 500.5a), which no
    /// restricted mana does.
    pub fn add_mana(&mut self, player: PlayerId, mana: Mana, n: u8, source: ObjectId, restriction: Option<ManaRestriction>, lasting: bool) {
        let m = mana as usize;
        // A pool holds at most `u8::MAX` of each kind (of each restriction): what more there would be is not added, so that
        // the count never wraps round and the sources' entries still add up to it (`docs/known-issues.md`, KI-62).
        let held = match restriction {
            Some(r) => self.restricted_mana.iter().find(|e| e.player == player && e.restriction == r).map_or(0, |e| e.pool.0[m]),
            None => self.player(player).pool.0[m],
        };
        let n = n.min(u8::MAX - held);
        if n == 0 {
            return;
        }
        match restriction {
            Some(r) => self.add_restricted_mana(player, r, mana, n),
            None => self.update_player(player, |p| {
                if lasting {
                    p.lasting.0[m] = p.lasting_mana().0[m] + n;
                }
                p.pool.0[m] += n;
            }),
        }
        let lasting = lasting && restriction.is_none();
        match self.mana_from.iter().position(|e| e.player == player && e.restriction == restriction && e.source == source && e.lasting == lasting) {
            Some(at) => {
                let old = self.mana_from[at];
                self.mana_from[at].pool.0[m] += n;
                self.log(Undo::ManaFromSet(at as u16, old));
            }
            None => {
                let mut pool = ManaPool::default();
                pool.0[m] = n;
                self.mana_from.push(ManaFrom { player, restriction, source, lasting, pool });
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
        for lasting in [false, true] {
            let mut at = 0;
            while left > 0 && at < self.mana_from.len() {
                let e = self.mana_from[at];
                if e.player == player && e.restriction == restriction && e.lasting == lasting && e.pool.0[m] > 0 && pick.takes(e.source) {
                    let take = e.pool.0[m].min(left);
                    left -= take;
                    taken.push((e.source, mana, take));
                    let mut after = e;
                    after.pool.0[m] -= take;
                    if after.pool.is_empty() {
                        self.mana_from.remove(at);
                        self.log(Undo::ManaFromRemoved(at as u16, e));
                        continue;
                    }
                    self.mana_from[at] = after;
                    self.log(Undo::ManaFromSet(at as u16, e));
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
                let lasting: u8 =
                    self.mana_from.iter().filter(|e| e.player == player && e.restriction.is_none() && e.lasting).map(|e| e.pool.0[m]).sum();
                self.update_player(player, |p| {
                    p.pool.0[m] -= spent;
                    p.lasting.0[m] = lasting;
                });
            }
        }
        spent
    }

    /// Empties `player`'s pool as a step or phase ends (CR 106.4, 500.4): all of it, or, `keep_lasting`, all but the mana
    /// that lasts until end of combat (CR 500.5a).
    pub fn empty_mana(&mut self, player: PlayerId, keep_lasting: bool) {
        if !self.players[player.index()].pool.is_empty() || !self.players[player.index()].lasting.is_empty() {
            self.update_player(player, |pl| {
                pl.pool = if keep_lasting { pl.lasting_mana() } else { ManaPool::default() };
                pl.lasting = pl.pool;
            });
        }
        self.empty_restricted_mana(player);
        if self.mana_from.iter().any(|e| e.player == player && !(keep_lasting && e.lasting)) {
            let old = self.mana_from.clone();
            self.mana_from.retain(|e| e.player != player || (keep_lasting && e.lasting));
            self.log(Undo::ManaFromAll(old));
        }
    }

    /// Adds `n` mana of one kind under `restriction` to `player`'s pool: the pool alone, without its source (`add_mana`).
    fn add_restricted_mana(&mut self, player: PlayerId, restriction: ManaRestriction, mana: Mana, n: u8) {
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
        let take = self.restricted_mana[at].pool.get(mana).min(n);
        if take == 0 {
            return 0;
        }
        let old = self.restricted_mana.clone();
        self.restricted_mana[at].pool.0[mana as usize] -= take;
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
        self.log(Undo::ZoneInserted { slot: slot as u16, at: at as u16 });
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
        self.log(Undo::ZoneRemoved { slot: slot as u16, at: at as u16, id });
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
        self.log(Undo::StackSet(at as u16, Box::new(old)));
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
        self.log(Undo::EffectSet(at as u16, Box::new(old)));
        self.set_unsettled();
    }

    /// Removes every effect `remove` says to, keeping the order of the rest.
    /// Keeps the effects at the indices `keep` marks true (one flag per effect, in order); the rest go.
    pub fn effects_keep(&mut self, keep: &[bool]) {
        debug_assert_eq!(keep.len(), self.effects.len(), "one flag per effect");
        for i in (0..self.effects.len()).rev() {
            if !keep[i] {
                let effect = self.effects.remove(i);
                self.log(Undo::EffectRemoved(i as u16, Box::new(effect)));
                self.set_unsettled();
            }
        }
    }

    pub fn effects_retain(&mut self, mut keep: impl FnMut(&ContinuousEffect) -> bool) {
        let mut i = self.effects.len();
        while i > 0 {
            i -= 1;
            if !keep(&self.effects[i]) {
                let effect = self.effects.remove(i);
                self.log(Undo::EffectRemoved(i as u16, Box::new(effect)));
                self.set_unsettled();
            }
        }
    }

    pub fn permission_push(&mut self, permission: PlayPermission) {
        self.permissions.push(permission);
        self.log(Undo::PermissionPushed);
    }

    /// Removes every permission `keep` says not to keep, keeping the order of the rest.
    pub fn permissions_retain(&mut self, mut keep: impl FnMut(&PlayPermission) -> bool) {
        let mut i = self.permissions.len();
        while i > 0 {
            i -= 1;
            if !keep(&self.permissions[i]) {
                let p = self.permissions.remove(i);
                self.log(Undo::PermissionRemoved(i as u16, p));
            }
        }
    }

    pub fn trigger_push(&mut self, trigger: PendingTrigger) {
        self.triggers.push(trigger);
        self.log(Undo::TriggerPushed);
    }

    pub fn trigger_remove(&mut self, at: usize) -> PendingTrigger {
        let trigger = self.triggers.remove(at);
        self.log(Undo::TriggerRemoved(at as u16, trigger));
        trigger
    }

    pub fn delayed_push(&mut self, delayed: Delayed) {
        self.delayed.push(delayed);
        self.log(Undo::DelayedPushed);
    }

    pub fn delayed_remove(&mut self, at: usize) -> Delayed {
        let d = self.delayed.remove(at);
        self.log(Undo::DelayedRemoved(at as u16, d));
        d
    }

    pub fn last_known_push(&mut self, row: LastKnown) {
        self.last_known.push(row);
        self.log(Undo::LastKnownPushed);
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
        Span { start: start as u32, len: objects.len() as u16 }
    }

    /// The objects a span names.
    pub fn members(&self, span: Span) -> &[ObjectId] {
        &self.members[span.start as usize..span.start as usize + span.len as usize]
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

    /// The characteristics an object last had on the battlefield, if it left it.
    pub fn last_known(&self, id: ObjectId) -> Option<&LastKnown> {
        self.last_known.iter().rev().find(|l| l.id == id)
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
        self.log(Undo::FrameRemoved(at as u16, frame.clone()));
        frame
    }

    /// Changes the frame at `at`.
    pub fn update_frame(&mut self, at: usize, f: impl FnOnce(&mut Frame)) {
        let old = self.agenda[at].clone();
        f(&mut self.agenda[at]);
        if self.agenda[at] != old {
            self.log(Undo::FrameSet(at as u16, old));
        }
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
            Undo::AttackersSet(old) => self.attackers = old,
            Undo::BlocksSet(old) => self.blocks = old,
            Undo::ExtraTurnsSet(old) => self.extra_turns = old,
            Undo::LoopWatchSet(old) => self.loop_watch = old,
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
                Undo::ManaFromRemoved(self.mana_from.len() as u16 - 1, *self.mana_from.last().expect("mana"))
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
            Undo::EffectPushed => Undo::EffectRemoved(self.effects.len() as u16 - 1, Box::new(*self.effects.last().expect("an effect"))),
            Undo::EffectRemoved(at, _) => Undo::EffectInserted(*at),
            Undo::EffectInserted(at) => Undo::EffectRemoved(*at, Box::new(self.effects[*at as usize])),
            Undo::EffectSet(at, _) => Undo::EffectSet(*at, Box::new(self.effects[*at as usize])),
            Undo::PermissionPushed => {
                Undo::PermissionRemoved(self.permissions.len() as u16 - 1, *self.permissions.last().expect("a permission"))
            }
            Undo::PermissionRemoved(at, _) => Undo::PermissionInserted(*at),
            Undo::PermissionInserted(at) => Undo::PermissionRemoved(*at, self.permissions[*at as usize]),
            Undo::TriggerPushed => Undo::TriggerRemoved(self.triggers.len() as u16 - 1, *self.triggers.last().expect("a trigger")),
            Undo::TriggerRemoved(at, _) => Undo::TriggerInserted(*at),
            Undo::TriggerInserted(at) => Undo::TriggerRemoved(*at, self.triggers[*at as usize]),
            Undo::DelayedPushed => Undo::DelayedRemoved(self.delayed.len() as u16 - 1, *self.delayed.last().expect("delayed")),
            Undo::DelayedRemoved(at, _) => Undo::DelayedInserted(*at),
            Undo::DelayedInserted(at) => Undo::DelayedRemoved(*at, self.delayed[*at as usize]),
            Undo::LastKnownPushed => Undo::LastKnownPopped(Box::new(*self.last_known.last().expect("a row"))),
            Undo::LastKnownPopped(_) => Undo::LastKnownPushed,
            Undo::LeftStackPushed => Undo::LeftStackPopped(Box::new(*self.left_stack.last().expect("an item"))),
            Undo::LeftStackPopped(_) => Undo::LeftStackPushed,
            Undo::MembersPushed(len) => Undo::MembersPopped(self.members[*len as usize..].to_vec()),
            Undo::MembersPopped(_) => Undo::MembersPushed(self.members.len() as u32),
            Undo::AttackersSet(_) => Undo::AttackersSet(self.attackers.clone()),
            Undo::BlocksSet(_) => Undo::BlocksSet(self.blocks.clone()),
            Undo::ExtraTurnsSet(_) => Undo::ExtraTurnsSet(self.extra_turns.clone()),
            Undo::LoopWatchSet(_) => Undo::LoopWatchSet(self.loop_watch.clone()),
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

    /// A pool holds at most 255 of a kind (KI-62): mana past that is not added, and the sources' shares still add up to it.
    #[test]
    fn mana_past_what_a_pool_holds_is_not_added() {
        let mut s = GameState::new(2, 1);
        let p = PlayerId(0);
        for i in 0..300u32 {
            s.add_mana(p, Mana::B, 1, ObjectId(i), None, false);
        }
        assert_eq!(s.player(p).pool.get(Mana::B), u8::MAX);
        let shares: u32 = s.mana_from_of(p).map(|e| e.pool.get(Mana::B) as u32).sum();
        assert_eq!(shares, u8::MAX as u32);
    }
}
