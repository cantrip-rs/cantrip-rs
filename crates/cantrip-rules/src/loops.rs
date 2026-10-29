//! Loops (CR 104.4b, 732; `docs/rules-exceptions.md`, RX-4). The engine finds one kind of loop only: the game comes back to
//! the same exact position at priority with every player having only passed priority since, and nothing having asked the
//! game's history in between. Such a loop is let go round - a player holding an answer outside it (a Disenchant in hand)
//! may still choose to end it, and no player can be made to (CR 732.5) - until the same position has come round
//! `LOOP_ROUNDS` more times after the first time it was seen again, each compared in full; the game is then a draw. Every
//! other loop is played by hand for as long as its players choose: no player is made to choose otherwise (CR 732.3 is not
//! applied, RX-4), the engine supports no shortcut (CR 732.2), and nothing here cuts it off. A person is held to the match
//! clock (the draw offers RX-4 describes come with the server's support for them), a seat nobody drives stops by itself
//! (the C# client's `LoopGuard`), and a game that reaches `MAX_GAME_EVENTS` events is a draw (`Game::run`; RX-1). A loop of
//! mandatory actions in which nobody is ever given priority is drawn by that cap.
//!
//! **A run of passes.** Any choice ends the run: an action at priority (`acted`), or an answer to any other question with
//! more than one legal answer (`asked`; a "may" declined is a choice, RX-2). A question with one legal answer is none: a
//! lone target, the only host an Aura can enter attached to. Every priority given with something on the stack in a run of
//! passes but its first `SKIPPED` is fingerprinted (`at_priority`), and a fingerprint seen again is the game come round a
//! loop of passes once more - once the position, written out in full (`words`), is the same as when it was first seen
//! again: no round rests on a hash. A priority with an empty stack is not fingerprinted - passing there ends the step (CR
//! 117.4), so the game comes back to the step only through something put on the stack, whose priorities are - and is no
//! choice either.
//!
//! **What a position is.** Whatever the game's future depends on, up to the identity of objects (CR 400.7: an exiled card
//! that returns is a new object, so object ids and timestamps grow round a loop although nothing else changes): each object
//! is numbered by where it is, a reference to an object that has left is followed one move to what it became, as far as
//! any rule follows it (`Obj::became`), with what its row and last-known information still say - past that, by those alone
//! - and timestamps are replaced by their order. Every amount is in it - life totals,
//! poison, counters on objects, damage, mana in pools, the cards in each library - so a loop that changes one each time
//! round never comes back to the same position: it goes on until the amount runs out (a player at 0 life, CR 704.5a; a
//! draw from an empty library, CR 704.5b) or reaches what a card waits for. The event history is not in the fingerprint, as
//! no two rounds have the same one; instead the history counts every time it is read (`cantrip_api::history::note_read`),
//! and a read clears the positions seen so far: a position comes round again only if nothing asked the past in between, so
//! that "the third time this turn", "only once each turn" and "if you've cast two spells this turn" can never be what tells
//! two rounds apart. A question about the past whose answer no later event of the turn can change ("it has triggered once
//! this turn already") is asked through `settled_read`, which counts it only while it could still change. What working out
//! a priority's options reads is not counted (`turn::priority`): while every player passes, what they could have done
//! changes nothing.
//!
//! **What the watch keeps.** Each position of the run once (`GameState::loop_keys`, found by its fingerprint in an index,
//! journalled an entry at a time) with the rounds the game has come round it, and, once it has come round, its words: a
//! run of any length costs a fingerprint and a lookup per priority, and a position seen again its words besides. Nothing
//! bounds what it keeps but the game's own maximum on events (RX-1). A normal game pays a fingerprint per priority with
//! something on the stack but the first two of each run, and a count per read of the history.

use std::hash::{Hash, Hasher};

use cantrip_api::history::{note_read, take_reads};
use cantrip_api::*;
use cantrip_state::{ChoiceRecord, GameState, LastKnown, LoopWatch};

use crate::Cx;

/// How many times a loop of passes comes round to the same position before the game is a draw (RX-4): full rounds in which
/// every player had priority and only passed, each a chance for one of them to end it with an action outside the loop.
pub const LOOP_ROUNDS: u32 = 3;

/// Whether `kind` has exactly one legal answer, so that answering it is no choice (CR 104.4b). Errs towards "a choice":
/// any question not known to have one answer counts as one. Priority is not judged here: passing is no choice, acting is
/// (`acted`).
pub fn forced(kind: &RequestKind) -> bool {
    match kind {
        RequestKind::ChooseTarget { candidates, optional, least, count, any_number, .. } => {
            !optional && !any_number && least == count && *count as usize == candidates.len()
        }
        // Every one of the cards it may take, and nothing else for its rule to judge.
        RequestKind::ChooseCards { cards, rule, .. } => {
            let eligible = rule.among.as_ref().map_or(cards.len(), Vec::len);
            rule.parts().next().is_none() && rule.count == Count::Exactly(count_u32(eligible))
        }
        RequestKind::ChooseModes { eligible, count, at_most, repeat, .. } => !repeat && *at_most == 0 && *count as usize == eligible.len(),
        RequestKind::ChoosePlayer { candidates, .. } => candidates.len() == 1,
        RequestKind::OrderTriggers { abilities, .. } => abilities.len() < 2,
        _ => false,
    }
}

/// A question about the past whose answer, once `settled` says so, no later event of this turn can change (a count that
/// has reached its limit, "the first card drawn this turn" once one has been): asked of the history, and counted as a read
/// of it (`note_read`) only while it could still change. A loop within the turn can't be told apart by a settled answer.
pub fn settled_read<T>(cx: &Cx, ask: impl FnOnce(&History) -> T, settled: impl FnOnce(&T) -> bool) -> T {
    let history = History { records: &cx.g.log, turn: cx.s().turn.number };
    let answer = ask(&history);
    if !settled(&answer) {
        note_read();
    }
    answer
}

/// A question is about to be asked (or asked again, on a re-run): one other than priority with more than one legal answer
/// is a choice, and ends the run of passes. Every question a player is asked comes through here: `Cx::ask_core`,
/// `Cx::ask_then`, and the question a player who left was asked, once it is put to the player chosen to make it
/// (CR 800.4g, `Game::answer`).
pub fn asked(s: &mut GameState, kind: &RequestKind) {
    if !matches!(kind, RequestKind::Priority { .. }) && !forced(kind) {
        clear(s);
    }
}

/// A player took an action at priority: the run of passes is over.
pub fn acted(cx: &mut Cx) {
    clear(cx.st());
}

fn clear(s: &mut GameState) {
    s.loop_keys_clear();
    if s.loop_watch != LoopWatch::default() {
        s.set_loop_watch(LoopWatch::default());
    }
}

/// A frame of the rules has run: if it asked the history anything, the positions seen before can't be compared with any
/// after (see the module).
pub fn after_frame(s: &mut GameState) {
    if take_reads() > 0 {
        clear(s);
    }
}

/// How many priorities with something on the stack a run of passes begins with that are not fingerprinted (see
/// `at_priority`).
const SKIPPED: u8 = 2;

/// Priority is about to be given. True if the game has come round a loop of passes `LOOP_ROUNDS` times, and is now a
/// draw (RX-4): the draw is made, and nobody is asked anything.
pub fn at_priority(cx: &mut Cx) -> bool {
    // The history asked since the last frame: the positions seen before can't be compared with this one.
    if take_reads() > 0 {
        clear(cx.st());
    }
    if cx.s().stack.is_empty() {
        return false;
    }
    // A run's first `SKIPPED` priorities are not fingerprinted, which spares a spell cast and answered by passes any
    // fingerprint: the positions of a loop that come after them are counted from the loop's first round, and the loop is
    // found as it comes round at them. Only a loop all of whose first round is those first priorities (a round of two,
    // starting where the run starts) is found a round later.
    let watch = cx.s().loop_watch.clone();
    if watch.skipped < SKIPPED {
        cx.st().set_loop_watch(LoopWatch { skipped: watch.skipped + 1, last: None });
        return false;
    }
    let s = cx.s();
    let canon = Canon::new(s);
    let here = fingerprint(s, &canon);
    // The same priority asked again (a run redone with its answers) is the same question, not a round.
    if watch.last == Some(here) {
        return false;
    }
    // A position seen before by its fingerprint: written out in full, to be compared, so that no round rests on a hash.
    let seen = s.loop_index.get(&here).map(|&at| (at as usize, words(s, &canon)));
    drop(canon);
    cx.st().set_loop_watch(LoopWatch { skipped: SKIPPED, last: Some(here) });
    let Some((at, words)) = seen else {
        cx.st().loop_key_entry(here);
        return false;
    };
    match &cx.s().loop_keys[at].words {
        // The first time round again, the position is kept in full; each time after, it is a round if it is the same.
        None => {
            cx.st().set_loop_words(at, words);
            return false;
        }
        Some(kept) if *kept != words => return false,
        Some(_) => {}
    }
    let rounds = cx.s().loop_keys[at].rounds.saturating_add(1);
    if rounds < LOOP_ROUNDS {
        cx.st().set_loop_rounds(at, rounds);
        return false;
    }
    clear(cx.st());
    crate::ops::end_game(cx, Outcome::Draw);
    cx.record(Event::GameDrawn { reason: DrawReason::MandatoryLoop });
    true
}

// --- Positions -----------------------------------------------------------------------------------------------------------

/// The hash a position is fingerprinted with: each word written is folded into the state by a folded multiply (the
/// 128-bit product of the state, the word and a constant, its halves xored together), as the foldhash and wyhash families
/// do, and the result is mixed once more when it is read. Two positions that differ collide about as rarely as two random
/// 64-bit values do - what the default hasher gives, at a fraction of its cost per write (a position is written a field
/// at a time, and an object has some sixty). Nothing here is chosen by an adversary.
#[derive(Default)]
struct Mix(u64);

impl Mix {
    fn fold(&mut self, x: u64) {
        let product = u128::from(self.0 ^ x ^ 0x243f_6a88_85a3_08d3).wrapping_mul(0x9e37_79b9_7f4a_7c15_u128 | 1 << 64);
        self.0 = (product as u64) ^ (product >> 64) as u64;
    }
}

impl Hasher for Mix {
    fn finish(&self) -> u64 {
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            self.fold(u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut last = [0u8; 8];
            last[..rest.len()].copy_from_slice(rest);
            // The tail's length goes with it, so that a tail of zero bytes is told from none.
            self.fold(u64::from_le_bytes(last) ^ (rest.len() as u64) << 59);
        }
    }
    fn write_u8(&mut self, n: u8) {
        self.fold(u64::from(n));
    }
    fn write_u16(&mut self, n: u16) {
        self.fold(u64::from(n));
    }
    fn write_u32(&mut self, n: u32) {
        self.fold(u64::from(n));
    }
    fn write_u64(&mut self, n: u64) {
        self.fold(n);
    }
    fn write_usize(&mut self, n: usize) {
        self.fold(n as u64);
    }
}

/// A position's canonical words: every write the fingerprint is made of, kept in full (`LoopKey::words`). Each integer is a
/// word, and a run of bytes is its length and then its bytes eight to a word. The writes come from the rows' own `Hash`,
/// which writes every field in order and the length of everything of variable length before it, so two positions that
/// differ write words that differ: two positions with the same words are the same.
#[derive(Default)]
struct Words(Vec<u64>);

impl Hasher for Words {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.push(bytes.len() as u64);
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.0.push(u64::from_le_bytes(word));
        }
    }
    fn write_u8(&mut self, n: u8) {
        self.0.push(u64::from(n));
    }
    fn write_u16(&mut self, n: u16) {
        self.0.push(u64::from(n));
    }
    fn write_u32(&mut self, n: u32) {
        self.0.push(u64::from(n));
    }
    fn write_u64(&mut self, n: u64) {
        self.0.push(n);
    }
    fn write_usize(&mut self, n: usize) {
        self.0.push(n as u64);
    }
}

thread_local! {
    /// The buffers a fingerprint is worked out in, kept from one priority to the next: `Canon::number` and the timestamps.
    static BUFFERS: std::cell::RefCell<(Vec<u32>, Vec<u32>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

/// The objects of a position numbered by where they are, and its timestamps in order (see the module).
struct Canon<'a> {
    s: &'a GameState,
    number: Vec<u32>,
    stamps: Vec<u32>,
}

impl<'a> Canon<'a> {
    fn new(s: &'a GameState) -> Canon<'a> {
        let (mut number, mut stamps) = BUFFERS.with(|b| std::mem::take(&mut *b.borrow_mut()));
        number.clear();
        number.resize(s.objects.len(), u32::MAX);
        let mut n = 0u32;
        for zone in &s.zones {
            for &id in zone {
                number[id.index()] = n;
                n += 1;
            }
        }
        // Timestamps by their order (CR 613.7).
        stamps.clear();
        stamps.extend(s.zones.iter().flatten().map(|&id| s.obj(id).timestamp));
        stamps.extend(s.effects.iter().map(|e| e.timestamp));
        stamps.extend(s.counter_stamps.iter().map(|c| c.timestamp));
        stamps.sort_unstable();
        stamps.dedup();
        Canon { s, number, stamps }
    }

    fn rank(&self, t: u32) -> usize {
        self.stamps.binary_search(&t).unwrap_or(usize::MAX)
    }

    fn id(&self, id: ObjectId) -> Name<'_> {
        Name(self, id)
    }

    fn target(&self, t: Target) -> TargetName<'_> {
        TargetName(self, t)
    }

    fn members(&self, span: cantrip_state::Span) -> Names<'_> {
        let start = span.start as usize;
        Names(self, self.s.members.get(start..start + span.len as usize).unwrap_or(&[]))
    }
}

impl Drop for Canon<'_> {
    fn drop(&mut self) {
        let buffers = (std::mem::take(&mut self.number), std::mem::take(&mut self.stamps));
        BUFFERS.with(|b| *b.borrow_mut() = buffers);
    }
}

/// An object as the position names it: by where it is; for one that left once, by where the object it became is and what
/// it still says; for one further gone, by what it still says alone.
struct Name<'a>(&'a Canon<'a>, ObjectId);

impl Hash for Name<'_> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let Name(canon, id) = *self;
        if id == ObjectId::NONE || id.index() >= canon.number.len() {
            return 0u8.hash(h);
        }
        let s = canon.s;
        // What an object that left still says (CR 608.2h: an ability of it reads it as it last was): its row and its
        // last-known information, without the ids and timestamps that differ round a loop.
        let departed = |h: &mut H| {
            let mut o = *s.obj(id);
            o.timestamp = 0;
            o.became = None;
            o.last_known = None;
            o.attached_to = None;
            o.entered_with = None;
            o.prepared_from = None;
            o.exiled = None;
            o.marks.choices = o.marks.choices.with_object(None);
            if let Some(phased) = &mut o.marks.phased { phased.last_known = 0; }
            let known = s.last_known(id).map(|k| LastKnown { id: ObjectId::NONE, ..*k });
            (o, known).hash(h);
        };
        match s.obj(id).became {
            None if canon.number[id.index()] != u32::MAX => (1u8, canon.number[id.index()]).hash(h),
            // One move on, the object it became and where that is: what the rules may still follow (`Obj::became`).
            Some(next) if s.obj(next).became.is_none() && canon.number[next.index()] != u32::MAX => {
                (2u8, canon.number[next.index()]).hash(h);
                departed(h);
            }
            // Two moves or more, or gone: no rule follows it that far, so only what it still says counts - and a reference
            // whose object moves every round of a loop names the same thing each round.
            _ => {
                3u8.hash(h);
                departed(h);
            }
        }
    }
}

struct TargetName<'a>(&'a Canon<'a>, Target);

impl Hash for TargetName<'_> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let TargetName(canon, t) = *self;
        match t {
            Target::None => 0u8.hash(h),
            Target::Player(p) => (1u8, p.0).hash(h),
            Target::Object(id) => (2u8, canon.id(id)).hash(h),
            Target::Ability(serial) => (3u8, canon.s.stack.iter().position(|i| i.serial == serial)).hash(h),
        }
    }
}

/// Objects named in order, as a list is written: its length, then each.
struct Names<'a>(&'a Canon<'a>, &'a [ObjectId]);

impl Hash for Names<'_> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.1.len().hash(h);
        for &id in self.1 {
            self.0.id(id).hash(h);
        }
    }
}

/// Targets named in order, as a list is written.
struct TargetNames<'a>(&'a Canon<'a>, &'a [Target]);

impl Hash for TargetNames<'_> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.1.len().hash(h);
        for &t in self.1 {
            self.0.target(t).hash(h);
        }
    }
}

/// The position's fingerprint (see the module).
fn fingerprint(s: &GameState, canon: &Canon) -> u64 {
    let mut h = Mix::default();
    write_position(s, canon, &mut h, true);
    h.finish()
}

/// The position written out in full, as a loop's rounds are compared (see the module): the same words are the same
/// position up to the identity of objects and the values of timestamps. The verifier's check that a settling loop makes
/// progress (`settle`).
pub(crate) fn position(s: &GameState) -> Vec<u64> {
    words(s, &Canon::new(s))
}

/// The position's canonical words (see `Words`).
fn words(s: &GameState, canon: &Canon) -> Vec<u64> {
    let mut w = Words::default();
    write_position(s, canon, &mut w, false);
    w.0
}

/// How many rows `row` keeps: more than a game's objects in its zones at once.
const ROW_SLOTS: usize = 4096;

thread_local! {
    /// The last row hashed in each slot, and its hash (`row`).
    static ROWS: std::cell::RefCell<Vec<Option<(Obj, u64)>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The hash of an object's row, as `fingerprint` writes it. Most rows are as they were at the last priority fingerprinted,
/// so the last hash of each object is kept with the row it was of, in a slot of `ROW_SLOTS` its index picks, and given again
/// for an equal row. The hash is a function of the row alone, so a row kept from another object, another game or an
/// earlier position that is equal gives the same answer, and one that is not is hashed afresh: the slots bound only what
/// is kept, never what is found.
fn row(id: ObjectId, o: &Obj) -> u64 {
    ROWS.with(|rows| {
        let mut rows = rows.borrow_mut();
        let at = id.index() % ROW_SLOTS;
        if let Some(Some((kept, hash))) = rows.get(at)
            && kept == o
        {
            return *hash;
        }
        let mut h = Mix::default();
        o.hash(&mut h);
        let hash = h.finish();
        if rows.len() <= at {
            rows.resize(at + 1, None);
        }
        rows[at] = Some((*o, hash));
        hash
    })
}

/// Writes the position up to the identity of objects and the order of timestamps (see the module): for the fingerprint
/// (`hashed`, each object's row by its kept hash, `row`) or in full (each row itself).
/// Every list is written with its length before its elements, so that no two positions write the same words.
fn write_position<H: Hasher>(s: &GameState, canon: &Canon, h: &mut H, hashed: bool) {
    s.turn.hash(h);
    s.players.hash(h);
    s.restricted_mana.hash(h);
    s.mana_from.len().hash(h);
    for m in &s.mana_from {
        (m.player, m.restriction, canon.id(m.source), m.lasts, m.pool).hash(h);
    }
    s.counter_stamps.len().hash(h);
    for c in &s.counter_stamps {
        (canon.id(c.object), c.kind, canon.rank(c.timestamp)).hash(h);
    }
    s.zones.len().hash(h);
    for zone in &s.zones {
        zone.len().hash(h);
        for &id in zone {
            let mut o = *s.obj(id);
            let stamp = canon.rank(o.timestamp);
            (
                o.attached_to.map(|id| canon.id(id)),
                o.entered_with.map(|id| canon.id(id)),
                o.prepared_from.map(|id| canon.id(id)),
                o.exiled.map(|e| (canon.id(e.by), e.text, e.from, e.until_leaves, e.encoded)),
                o.marks.choices.entered_raw().1.object.map(|c| canon.id(c)),
                stamp,
                s.phased_last_known(id).map(|last| LastKnown { id: ObjectId::NONE, ..*last }),
            )
                .hash(h);
            o.timestamp = 0;
            o.became = None;
            o.attached_to = None;
            o.entered_with = None;
            o.prepared_from = None;
            o.exiled = None;
            o.marks.choices = o.marks.choices.with_object(None);
            if let Some(phased) = &mut o.marks.phased { phased.last_known = 0; }
            if hashed {
                row(id, &o).hash(h);
            } else {
                o.hash(h);
            }
        }
    }
    s.stack.len().hash(h);
    for item in &s.stack {
        let mut i = *item;
        (
            canon.id(i.object),
            TargetNames(canon, s.targets(item)),
            s.spec_of(item),
            s.shares(item),
            canon.target(i.subject),
            canon.members(i.members),
        )
            .hash(h);
        i.object = ObjectId::NONE;
        // The targets, their slots and shares are written above, read through the arena: not where the arena keeps them.
        i.inline_targets = [Target::None; cantrip_state::INLINE_TARGETS];
        i.inline_spec_of = [0; cantrip_state::INLINE_TARGETS];
        i.shares = 0;
        i.spill = 0;
        i.subject = Target::None;
        i.members = cantrip_state::Span::default();
        i.serial = 0;
        i.since = 0;
        i.hash(h);
    }
    s.effects.len().hash(h);
    for e in &s.effects {
        (canon.id(e.source), e.controller, canon.rank(e.timestamp), canon.id(e.affected), e.kind, e.duration).hash(h);
    }
    s.permissions.len().hash(h);
    for p in &s.permissions {
        // The permanent a "for as long as you control" permission is about, by where it is, as every object is.
        let source = p.grant.expiry.controlled().map(|id| canon.id(id));
        let grant = PlayGrant { expiry: if source.is_some() { Expiry::WhileControls(ObjectId::NONE) } else { p.grant.expiry }, ..p.grant };
        (canon.id(p.card), p.player, grant, source, p.turn).hash(h);
    }
    s.triggers.len().hash(h);
    for t in &s.triggers {
        (canon.id(t.source), t.ability, t.controller, canon.target(t.subject), t.data, canon.members(t.members), t.count).hash(h);
    }
    s.delayed.len().hash(h);
    for d in &s.delayed {
        (d.when, canon.id(d.source), d.controller, d.ability, canon.target(d.subject), canon.members(d.members)).hash(h);
    }
    s.attackers.len().hash(h);
    for a in &s.attackers {
        (canon.id(a.id), canon.target(a.target), a.defending, a.attacked_as, a.blocked, a.first_strike, a.band).hash(h);
    }
    s.blocks.len().hash(h);
    for b in &s.blocks {
        (canon.id(b.blocker), canon.id(b.attacker)).hash(h);
    }
    s.extra_turns.hash(h);
    s.rng.hash(h);
    s.agenda.len().hash(h);
    for f in &s.agenda {
        match f.kind {
            // The item resolving by its place on the stack, not its serial, a timestamp that differs each time round.
            cantrip_state::FrameKind::Resolve { serial } => (u8::MAX, s.stack.iter().rposition(|i| i.serial == serial)).hash(h),
            kind => kind.hash(h),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Game;

    fn never(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
        None
    }

    fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
        Ok(())
    }

    static CARD: CardDef = CardDef {
        name: "Loop Test",
        types: TypeSet::of(&[CardType::Enchantment]),
        abilities: &[triggered(when(EventMask::ZONE_CHANGE, never), nothing, "")],
        ..CardDef::BLANK
    };

    /// A game with one Loop Test on the battlefield and its ability on the stack.
    fn game() -> Game {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&CARD])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let card = table.id_of(&CARD);
        let id = game.state.push_obj(Obj::new(card, PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        let item = crate::actions::stack_item(StackKind::Triggered, id, AbilityRef { card, index: 0 }, PlayerId(0), &[]);
        game.state.stack_push(item);
        game
    }

    fn cx(game: &mut Game) -> Cx<'_> {
        Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
    }

    #[test]
    fn phased_source_snapshot_contents_not_arena_index_decide_loop_identity() {
        let position = |s: &GameState, canon: &Canon| (fingerprint(s, canon), words(s, canon));
        let mut game = game();
        let source = game.state.battlefield()[0];
        crate::phasing::phase_out(&mut cx(&mut game), &[source], None, false);
        let first = position(&game.state, &Canon::new(&game.state));
        // An equivalent row at another arena index must not make an otherwise identical position different.
        let row = *game.state.phased_last_known(source).unwrap();
        let same = game.state.last_known_push(row);
        game.state.update_obj(source, |o| o.marks.phased.as_mut().unwrap().last_known = same);
        assert_eq!(position(&game.state, &Canon::new(&game.state)), first);
        let changed = game.state.last_known_push(LastKnown { keywords: row.keywords.with(Keyword::Lifelink), ..row });
        game.state.update_obj(source, |o| o.marks.phased.as_mut().unwrap().last_known = changed);
        assert_ne!(position(&game.state, &Canon::new(&game.state)), first, "different source information was ignored");
    }

    /// The permanent leaves and comes back: a new object (CR 400.7), at a later timestamp, and its ability triggers again,
    /// the ability's source the object that left.
    fn flicker(game: &mut Game) {
        let old = game.state.battlefield()[0];
        let item = game.state.stack_pop().expect("the ability");
        game.state.zone_remove(Zone::BATTLEFIELD, old);
        let t = game.state.timestamp();
        let mut obj = Obj::new(game.state.obj(old).card, PlayerId(0), Zone::BATTLEFIELD);
        obj.timestamp = t;
        let new = game.state.push_obj(obj);
        game.state.update_obj(old, |o| o.became = Some(new));
        game.state.zone_push(Zone::BATTLEFIELD, new);
        game.state.stack_push(StackItem { object: old, serial: 0, ..item });
    }

    /// CR 104.4b, 400.7: a position that has come round again is the same whatever its objects are numbered and whatever
    /// their timestamps, but anything else that differs - a life total - makes it another.
    #[test]
    fn a_position_reached_again_with_new_objects_has_the_same_fingerprint() {
        let mut game = game();
        flicker(&mut game);
        let print = |game: &Game| (fingerprint(&game.state, &Canon::new(&game.state)), words(&game.state, &Canon::new(&game.state)));
        let before = print(&game);
        flicker(&mut game);
        assert_eq!(print(&game), before, "a new object of the same card where the old one was");
        game.state.update_player(PlayerId(1), |p| p.life -= 1);
        let after = print(&game);
        assert!(after.0 != before.0 && after.1 != before.1, "a life total changed");
    }

    /// CR 104.4b: a question with one legal answer is no choice; one with more is.
    #[test]
    fn only_a_question_with_one_legal_answer_is_forced() {
        let choose = |candidates: Vec<Target>, optional: bool| RequestKind::ChooseTarget {
            source: ObjectId(0),
            purpose: Purpose::Targets,
            candidates,
            optional,
            least: 1,
            count: 1,
            any_number: false,
            label: "",
            slot_player: None,
            together: None,
        };
        assert!(forced(&choose(vec![Target::Object(ObjectId(1))], false)));
        assert!(!forced(&choose(vec![Target::Object(ObjectId(1))], true)), "choosing none is another answer");
        assert!(!forced(&choose(vec![Target::Object(ObjectId(1)), Target::Object(ObjectId(2))], false)));
        // "One or two targets" with two candidates: choosing one of them is another answer (CR 601.2c).
        let one_or_two = RequestKind::ChooseTarget {
            source: ObjectId(0),
            purpose: Purpose::Targets,
            candidates: vec![Target::Object(ObjectId(1)), Target::Object(ObjectId(2))],
            optional: false,
            least: 1,
            count: 2,
            any_number: false,
            label: "",
            slot_player: None,
            together: None,
        };
        assert!(!forced(&one_or_two));
        assert!(!forced(&RequestKind::May { source: ObjectId(0), purpose: Purpose::Optional }));
    }

    /// Both players pass once round the loop, `first` run before the round's first priority; whether the game was drawn on
    /// the way.
    fn round(game: &mut Game, first: impl Fn(&mut Game)) -> bool {
        flicker(game);
        first(game);
        let mut drawn = false;
        for p in [PlayerId(0), PlayerId(1)] {
            crate::loops::after_frame(&mut game.state);
            game.state.update_turn(|t| t.priority = p);
            drawn |= at_priority(&mut cx(game));
        }
        drawn
    }

    fn passes(_: &mut Game) {}

    /// The round of a run of passes, counted from 1 at its start, in which the test's two-priority loop is drawn: round 1 is
    /// the run's first `SKIPPED` priorities, round 2 sees the positions, round 3 comes round to them again and keeps them in
    /// full, and each round after is a confirmed one, `LOOP_ROUNDS` of them.
    const DRAWN_IN: u32 = 3 + LOOP_ROUNDS;

    /// RX-4: a loop every player only passes in is let go round, and is a draw once its position has come round
    /// `LOOP_ROUNDS` times confirmed, not before.
    #[test]
    fn a_loop_of_passes_is_a_draw_after_its_rounds_and_not_before() {
        take_reads();
        let mut game = game();
        for _ in 1..DRAWN_IN {
            assert!(!round(&mut game, passes));
        }
        assert_eq!(game.state.outcome, Outcome::InProgress);
        assert!(round(&mut game, passes), "come round `LOOP_ROUNDS` times confirmed");
        assert_eq!(game.state.outcome, Outcome::Draw);
        assert!(game.log.iter().any(|r| r.event == Event::GameDrawn { reason: DrawReason::MandatoryLoop }));
    }

    /// RX-4, RX-2: a choice in a round - an action at priority, or a "may" declined - starts a new run, however many rounds
    /// there were before it; a question with one legal answer is no choice.
    #[test]
    fn a_choice_in_a_round_starts_the_count_over() {
        take_reads();
        let mut game = game();
        for _ in 1..DRAWN_IN {
            assert!(!round(&mut game, passes));
        }
        // The round a choice is made in is the new run's first.
        assert!(!round(&mut game, |g| acted(&mut cx(g))), "an action at priority");
        for _ in 2..DRAWN_IN {
            assert!(!round(&mut game, passes));
        }
        let source = game.state.battlefield()[0];
        let may = RequestKind::May { source, purpose: Purpose::Optional };
        assert!(!round(&mut game, |g| asked(&mut g.state, &may)), "a may");
        let lone = RequestKind::ChoosePlayer { source, candidates: vec![PlayerId(1)], verb: "", amount: 0, subject: None };
        for _ in 2..DRAWN_IN {
            assert!(!round(&mut game, |g| asked(&mut g.state, &lone)), "a lone choice is none");
        }
        assert_eq!(game.state.outcome, Outcome::InProgress);
        assert!(round(&mut game, |g| asked(&mut g.state, &lone)));
        assert_eq!(game.state.outcome, Outcome::Draw);
    }

    /// A position whose fingerprint comes round but whose words are not those kept is no round: no draw rests on a hash.
    #[test]
    fn a_fingerprint_seen_again_with_other_words_is_no_round() {
        take_reads();
        let mut game = game();
        for _ in 0..3 {
            assert!(!round(&mut game, passes));
        }
        // Each position has come round once and is kept in full; another position's words stand in for a collision.
        assert!(game.state.loop_keys.iter().all(|k| k.words.is_some()));
        for at in 0..game.state.loop_keys.len() {
            game.state.set_loop_words(at, vec![7]);
        }
        for _ in 0..DRAWN_IN * 2 {
            assert!(!round(&mut game, passes));
        }
        assert!(game.state.loop_keys.iter().all(|k| k.rounds == 0));
        assert_eq!(game.state.outcome, Outcome::InProgress);
    }

    /// The same priority asked again (a run redone with its answers) is no round.
    #[test]
    fn a_priority_asked_again_is_no_round() {
        take_reads();
        let mut game = game();
        for _ in 0..LOOP_ROUNDS * 4 {
            assert!(!at_priority(&mut cx(&mut game)));
        }
        assert_eq!(game.state.loop_keys.len(), 1);
        assert_eq!(game.state.loop_keys[0].rounds, 0);
        assert_eq!(game.state.loop_keys[0].words, None);
    }

    /// Nothing on the stack: passing in succession ends the step (CR 117.4), so no position is kept.
    #[test]
    fn an_empty_stack_is_never_a_loop() {
        take_reads();
        let mut game = game();
        game.state.stack_pop();
        for _ in 0..LOOP_ROUNDS * 4 {
            assert!(!at_priority(&mut cx(&mut game)));
        }
        assert!(game.state.loop_keys.is_empty());
    }

    /// CR 603.2h: a loop whose rounds differ only in their history is the same position round after round, but a question
    /// the history is asked between them ("is this the sixth time this turn?") may answer otherwise on a later round, so the
    /// rounds can't be found the same. A read of the history clears the positions seen; without one, the same rounds are a
    /// draw.
    #[test]
    fn a_loop_whose_history_is_read_between_rounds_is_not_found_by_its_positions() {
        take_reads();
        let mut game = game();
        for _ in 0..DRAWN_IN * 2 {
            let read = |_: &mut Game| {
                let records: [Record; 0] = [];
                let _ = History::new(&records, 1).this_turn().len();
            };
            assert!(!round(&mut game, read), "the history was asked: the rounds could differ");
        }
        assert_eq!(game.state.outcome, Outcome::InProgress);
        let drawn = (0..DRAWN_IN).any(|_| round(&mut game, passes));
        assert!(drawn && game.state.outcome == Outcome::Draw, "the same rounds with nothing asking the past are a draw");
    }

    /// A question about the past settled for the rest of the turn is no read that tells rounds apart; one that is not yet
    /// settled is.
    #[test]
    fn a_settled_question_about_the_past_is_not_counted_as_a_read() {
        let mut game = game();
        take_reads();
        let c = cx(&mut game);
        assert!(settled_read(&c, |_| true, |&done| done));
        assert_eq!(take_reads(), 0);
        assert!(!settled_read(&c, |_| false, |&done| done));
        assert_eq!(take_reads(), 1);
    }

    /// A reference is named by the object it became one move on, as the rules follow it; past that only by what it still
    /// says, so two positions that differ only in how far past two moves a reference's object has gone are the same - a
    /// loop that moves the object every round comes round again.
    #[test]
    fn a_reference_two_moves_or_more_on_is_named_alike_however_far() {
        let game = game();
        let card = game.state.obj(game.state.battlefield()[0]).card;
        let chain = |game: &mut Game, moves: usize| {
            let first = game.state.push_obj(Obj::new(card, PlayerId(0), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, first);
            let mut now = first;
            for _ in 0..moves {
                let next = game.state.push_obj(Obj::new(card, PlayerId(0), Zone::BATTLEFIELD));
                game.state.zone_remove(Zone::BATTLEFIELD, now);
                game.state.update_obj(now, |o| o.became = Some(next));
                game.state.zone_push(Zone::BATTLEFIELD, next);
                now = next;
            }
            first
        };
        let names = |moves: usize| {
            let mut g = game.clone();
            let first = chain(&mut g, moves);
            let canon = Canon::new(&g.state);
            let mut w = Words::default();
            canon.id(first).hash(&mut w);
            w.0
        };
        assert_eq!(names(2), names(3));
        assert_eq!(names(2), names(20));
        assert_ne!(names(1), names(2), "one move on, the object it became is named");
        assert_ne!(names(0), names(1));
    }
}
