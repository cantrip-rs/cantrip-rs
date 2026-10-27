//! Characteristics (CR 613), computed when asked: the copiable values (layer 1, CR 707), then the continuous
//! effects that apply to the object in timestamp order or as they depend on each other (CR 613.7, 613.8), then counters
//! (layer 7c).
//!
//! Characteristics are not cached per object: a game has a few dozen objects and a handful of effects. What is shared
//! is what does not depend on the object: a card's own values (`CardTable::printed`) and the list of static-ability
//! sources, found once per position (`GameState::epoch`, which every write and undo moves).
//!
//! Every entry point takes the event history beside the state (`log`): a static ability's condition may ask about the
//! past ("as long as you've lost life this turn", `LayerQuery::history`), and its effect is not locked in (CR 611.3a), so it
//! is read afresh with everything else. Recording an event does not move the epoch, and nothing cached here depends on the
//! history: which permanents have static abilities is a question of the state alone.

use cantrip_api::*;
use cantrip_state::ContinuousEffect;

pub fn chars(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId) -> Chars {
    chars_before(cards, state, log, id, Layer::End)
}

/// Expected-zone source information (CR 113.7a, 608.2h, 702.15c): a phased-out permanent is read as it most recently
/// existed phased in. `chars` remains the live projected calculation; phase-in drops this snapshot link.
pub fn source_chars(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId) -> Chars {
    match state.phased_last_known(id) {
        Some(last) => initial(cards, last.card, None).as_known(last),
        None => chars(cards, state, log, id),
    }
}

/// How many threads are working out an object's abilities' givers now (`ability_givers`): read first, so the layers pay one
/// relaxed load per entry for the watching the rest of the time.
static WATCHING: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

thread_local! {
    /// The object whose abilities' givers are being worked out (`ability_givers`), `ObjectId::NONE` the rest of the time.
    static WATCHED: std::cell::Cell<ObjectId> = const { std::cell::Cell::new(ObjectId::NONE) };
    /// For each of the watched object's ability slots, the object whose effect put the ability there.
    static GIVERS: std::cell::Cell<[Giver; cantrip_state::MAX_ABILITIES]> = const { std::cell::Cell::new([Giver::NONE; cantrip_state::MAX_ABILITIES]) };
}

/// What gave an object one of its abilities (`ability_givers`): the other object whose effect did, and the effect, by how
/// the object acquired the ability (CR 607.5). `Giver::NONE` for one of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Giver {
    pub object: ObjectId,
    pub acquired: Acquired,
}

impl Giver {
    pub const NONE: Giver = Giver { object: ObjectId::NONE, acquired: Acquired::OWN };

    /// The giver of a resolved effect that gave an object an ability (`EffectKind::AddAbility`): its source, as the effect's
    /// entry gives it (`GEntry::giver`).
    fn of_effect(e: &ContinuousEffect) -> Giver {
        Giver { object: e.source, acquired: Acquired { by: AcquiredBy::Grant, stamp: e.timestamp, part: 0 } }
    }
}

/// Whether `ability` is of the text an object whose characteristics are `chars` has (CR 607.1, 607.1a-b): the card's it is
/// now, its face's or a copy effect's, read as a whole card (`CardTable::linked_card`).
fn own_text(cards: &CardTable, chars: &Chars, ability: AbilityRef) -> bool {
    cards.linked_card(ability.card) == cards.linked_card(chars.card)
}

/// `ability`, an ability `giver` gave `holder`, as linked abilities know it (CR 607.1, 607.5): of its card's text, acquired
/// by that effect - or, given by none, or by an effect of `holder`'s own and printed within its text (CR 607.1a), of the
/// text `holder` has, as `ability_givers` counts it (`acquired`).
fn given_ability(cards: &CardTable, state: &GameState, holder: ObjectId, ability: AbilityRef, giver: Giver) -> Linked {
    let text = cards.linked_card(ability.card);
    let own = giver.object == ObjectId::NONE || (giver.object == holder && text == cards.linked_to(state, holder).text);
    let acquired = if own { state.copy_acquired(holder) } else { giver.acquired };
    Linked { object: holder, text, acquired }
}

/// For each of `id`'s abilities (`Chars::abilities`, slot for slot), the other object whose effect gave it the ability (CR
/// 113.2c, 613.1f): the static ability's source (Chromatic Lantern, an Equipment), or the source of the resolved spell or
/// ability that did; `ObjectId::NONE` for one of its own (printed, copied, its own static's, a basic land type's). Its
/// characteristics are worked out again, watching each entry of layer 6 that applies to it: asked only of an object that has
/// an ability another card's, which is rare, so nothing is paid for it the rest of the time.
pub fn ability_givers(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId) -> [Giver; cantrip_state::MAX_ABILITIES] {
    /// Ends the watch however `chars` returns, a panic included, so the layers never go on paying for it.
    struct Watch(ObjectId);
    impl Drop for Watch {
        fn drop(&mut self) {
            WATCHING.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            WATCHED.with(|w| w.set(self.0));
        }
    }
    GIVERS.with(|g| g.set([Giver::NONE; cantrip_state::MAX_ABILITIES]));
    let watch = Watch(WATCHED.with(|w| w.replace(id)));
    WATCHING.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // Abilities are settled by layer 6: the layers after it give and take none.
    let _ = chars_before(cards, state, log, id, Layer::PtDefine);
    drop(watch);
    GIVERS.with(|g| g.get())
}

/// Whether an object could now have an ability by anything but its own text (CR 607.5): a copy effect applied to a permanent
/// after it entered (`EffectKind::RestoreCopy`), a resolved effect that gives an ability, or a static ability that gives
/// other objects abilities. With none, every ability is of its object's own text, and nothing is worked out.
pub fn acquisition_possible(cards: &CardTable, state: &GameState) -> bool {
    state.effects.iter().any(|e| matches!(e.kind, EffectKind::RestoreCopy(_) | EffectKind::AddAbility(_))) || ability_granting_present(cards, state)
}

/// Whether the ability in `slot` of `chars`, an object's characteristics, is surely one of the text it has (CR 607.5), with
/// nothing worked out: of the face that is up (`Chars::card`) or a door of the Room it is, or the rules' own, with no other
/// ability of the object the same and none of its abilities lost, so no effect can have given it (an ability given an
/// object that has the same one printed is a second one, CR 113.2c). An ability of the other face of a transforming card,
/// though that face is the same text (CR 607.1b), can only have been given (Agatha's Soul Cauldron lending Jacob Hauken's
/// to an animated Hauken's Insight), and is worked out.
fn surely_its_text(cards: &CardTable, chars: &Chars, slot: usize) -> bool {
    let Some(ability) = chars.abilities.get(slot).copied().flatten() else { return true };
    // One its card only lists to give (`CardDef::delayed`) it has only because an effect gave it: whose, is worked out.
    if ability.is_delayed() {
        return false;
    }
    let of_face_up = ability.card == chars.card
        || cards.halves.get(chars.card.0 as usize).copied().flatten().is_some_and(|(left, right)| ability.card == left || ability.card == right);
    ability.card == cards.intrinsic
        || (of_face_up && !chars.lost_all && !chars.text_lost && chars.abilities.iter().filter(|a| **a == Some(ability)).count() == 1)
}

/// How `id` acquired the ability in `slot` of `chars`, its characteristics now (CR 607.5): by the text it has (its own, or the
/// latest copy effect's, `GameState::copy_acquired`), or by the effect that granted it (`ability_givers`). Worked out only
/// where an effect could have given it the ability; what a stack item, a pending trigger and an exile link remember.
pub fn acquired(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId, chars: &Chars, slot: usize) -> Acquired {
    if !acquisition_possible(cards, state) {
        return Acquired::OWN;
    }
    let text = state.copy_acquired(id);
    if surely_its_text(cards, chars, slot) {
        return text;
    }
    let giver = ability_givers(cards, state, log, id)[slot];
    if giver.object == ObjectId::NONE { text } else { giver.acquired }
}

/// `ability`, an ability `id` has, acquired `acquired` (`acquired`), as linked abilities know it (CR 607.1, 607.5), where that
/// differs from an ability of the text it has (`CardTable::linked_to`): one an effect gave it. None for one of its text.
pub fn given_link(cards: &CardTable, state: &GameState, id: ObjectId, ability: AbilityRef, acquired: Acquired) -> Option<Linked> {
    let by = Linked { object: id, text: cards.linked_card(ability.card), acquired };
    (by != cards.linked_to(state, id)).then_some(by)
}

/// `acquired` for each of `chars`'s slots, of an object as it is now: none worked out (`None`) where every one is surely
/// of its text, which is nearly always.
pub fn acquisitions(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId, chars: &Chars) -> Option<Box<[Acquired; cantrip_state::MAX_ABILITIES]>> {
    if (0..cantrip_state::MAX_ABILITIES).all(|slot| surely_its_text(cards, chars, slot)) {
        return None;
    }
    let text = state.copy_acquired(id);
    let givers = ability_givers(cards, state, log, id);
    Some(Box::new(givers.map(|g| if g.object == ObjectId::NONE { text } else { g.acquired })))
}

/// `ability_givers` within a pass (`chars_in`).
fn givers_in(cx: &Ctx, id: ObjectId) -> [Giver; cantrip_state::MAX_ABILITIES] {
    struct Watch(ObjectId);
    impl Drop for Watch {
        fn drop(&mut self) {
            WATCHING.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            WATCHED.with(|w| w.set(self.0));
        }
    }
    let outer = GIVERS.with(|g| g.replace([Giver::NONE; cantrip_state::MAX_ABILITIES]));
    let watch = Watch(WATCHED.with(|w| w.replace(id)));
    WATCHING.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _ = chars_in(cx, id, Layer::PtDefine);
    drop(watch);
    GIVERS.with(|g| g.replace(outer))
}

/// The givers noted so far, where `id`'s are being worked out.
fn watched_givers(id: ObjectId) -> Option<[Giver; cantrip_state::MAX_ABILITIES]> {
    if WATCHING.load(std::sync::atomic::Ordering::Relaxed) == 0 {
        return None;
    }
    (WATCHED.with(|w| w.get()) == id).then(|| GIVERS.with(|g| g.get()))
}

/// Applies `f`, an entry of the layers, to `id`'s characteristics, and while its abilities' givers are being worked out
/// (`ability_givers`), notes `giver` beside each ability the entry put there (an ability the entry only moved keeps its
/// giver). One `id`'s own effect gave it is of its own text only if it is printed within that text (CR 607.1a: "{1}: This
/// creature gains '...'"); one of another card's text its own effect gave it is acquired by that effect (CR 607.5:
/// Quicksilver Elemental gaining Sisters of Stone Death's abilities).
fn watching_givers(cards: &CardTable, id: ObjectId, giver: Giver, chars: &mut Chars, f: impl FnOnce(&mut Chars)) {
    let Some(givers) = watched_givers(id) else { return f(chars) };
    let before = chars.abilities;
    f(chars);
    let mut after = givers;
    // An ability that left a slot and is now in another was moved (the abilities kept by "loses all abilities except mana
    // abilities", packed together): it keeps its giver. Any other new one is the entry's.
    let mut moved = [false; cantrip_state::MAX_ABILITIES];
    for (slot, ability) in chars.abilities.iter().enumerate() {
        if *ability == before[slot] {
            continue;
        }
        let from = (0..before.len()).find(|&from| !moved[from] && before[from].is_some() && before[from] == *ability && chars.abilities[from] != before[from]);
        after[slot] = match from {
            Some(from) => {
                moved[from] = true;
                givers[from]
            }
            None if giver.object == id && ability.is_some_and(|a| own_text(cards, chars, a)) => Giver::NONE,
            None => giver,
        };
    }
    GIVERS.with(|g| g.set(after));
}

/// An object's characteristics as if it were the face `face` of its card (CR 715.4, 709.3, 712.8a, 720.3): a card in hand
/// judged as the Adventure, Omen, half or back face it would be cast as - a spell with that face's values, worked out through
/// the layers with every effect that applies to the card where it is (Painter's Servant's color among them) and none that
/// applies only to the card (CR 400.7). Where the stack itself must be read, `ops::as_cast` puts it there.
pub fn chars_as_face(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId, face: CardId) -> Chars {
    let was = state.judged_as.replace(Some((id, face)));
    let chars = chars(cards, state, log, id);
    state.judged_as.set(was);
    chars
}

/// Token characteristics at creation: copiable values and intrinsic type abilities, without battlefield or
/// stack effects (CR 614.12, 707.2). A token-creation replacement reads this, the token as it is created, never as it
/// would enter: Draconic Visitor's ruling (2026-08-21) has it ignore a nonartifact token Mycosynth Lattice would make an
/// artifact as it enters (scenario draconic-visitor-ignores-a-token-mycosynth-lattice-makes-an-artifact).
pub fn token_chars(cards: &CardTable, card: CardId, copy: Option<Copiable>) -> Chars {
    let def = cards.get(card);
    let mut chars = initial(cards, card, copy.as_ref());
    if chars.is(CardType::Land) {
        intrinsic_mana(cards, def, &mut chars);
    }
    if chars.is(CardType::Battle) {
        intrinsic_siege(cards, &mut chars);
    }
    intrinsic_keyword_abilities(cards, &mut chars);
    chars
}

/// While the value lives, the face-down card `card` outside the battlefield and the stack (CR 406.3a) is judged as the card
/// it is: what playing it would be, asked of the resulting spell or land (CR 406.3a-b, 601.3, 305.1), for a player who may
/// play it. The card is turned face up only as it is played (`ops::turn_up_to_play`); until then it has no characteristics
/// for every other purpose. Nothing for any other object.
///
/// The layers' pass is kept apart while a card is judged (`PassKey::judged`); the static-ability sources (`static_sources`)
/// are not keyed on it: a face-down card in exile is never one, judged or not. A face-down permanent judged as it would be
/// face up (`Query::chars_if_face_up`) is not one either - its face-up static abilities are not applied to that judgement,
/// which no printed card with morph or disguise reaches (none has a static ability that takes its own abilities).
///
/// It does not ask whether the player may look at the card (`seen_by`): every permission that lets a face-down card be played
/// today comes from the effect that exiled it and let its player look. A permission to cast "spells with certain qualities
/// from among face-down cards in exile" must check that first (CR 406.3b, 601.3f).
pub struct JudgedFaceUp<'a> {
    state: &'a GameState,
    was: Option<Option<ObjectId>>,
}

pub fn judged_face_up(state: &GameState, card: ObjectId) -> JudgedFaceUp<'_> {
    let was = state.obj(card).face_down_card().then(|| state.judged_face_up.replace(Some(card)));
    JudgedFaceUp { state, was }
}

impl Drop for JudgedFaceUp<'_> {
    fn drop(&mut self) {
        if let Some(was) = self.was {
            self.state.judged_face_up.set(was);
        }
    }
}

/// What a static ability's functions read: other objects through the layers before the one being applied, or, while a
/// layer is being applied (`stage`), as that layer's entries applied so far leave them (CR 613.8: the order of a layer's
/// effects is judged on the whole game as it stands at each step). A static ability applied at step `k` of a layer reads
/// every other object at step `k`, so the reads go to strictly earlier steps and end.
#[derive(Clone, Copy)]
struct View<'a> {
    cards: &'a CardTable,
    state: &'a GameState,
    /// The event history (`cantrip_api::history`), which a static ability's condition may ask about the past.
    log: &'a [Record],
    stop: Layer,
    stage: Option<Stage<'a>>,
    /// The objects read through `chars` while a dependency is judged (CR 613.8a: applying one effect changes what another
    /// does when it changes an object the other reads).
    reads: Option<&'a std::cell::RefCell<Vec<ObjectId>>>,
    /// The pass a `stage` is of, if the one asking has it at hand.
    pass: Option<&'a Pass>,
    /// In layer 2, who controls each object as the control-changing effects applied so far leave it (`control_order`).
    controllers: Option<&'a std::collections::HashMap<ObjectId, PlayerId, Quick>>,
    /// The static ability whose functions are being asked, where an effect gave it its object (`GKind::Given`,
    /// `GKind::Granted`): as linked abilities know it (CR 607.5), what `linked_to` of that object answers. Its own and a copy
    /// effect's are the text the object has (`CardTable::linked_to`).
    asking: Option<Linked>,
}

impl<'a> View<'a> {
    fn new(cards: &'a CardTable, state: &'a GameState, log: &'a [Record], stop: Layer) -> View<'a> {
        View { cards, state, log, stop, stage: None, reads: None, pass: None, controllers: None, asking: None }
    }

    /// An object with the layers before `stop` done, in the pass at hand or the position's.
    fn before(&self, id: ObjectId, stop: Layer) -> Chars {
        match self.pass {
            Some(pass) => chars_in(&Ctx { cards: self.cards, state: self.state, log: self.log, pass }, id, stop),
            None => chars_before(self.cards, self.state, self.log, id, stop),
        }
    }

    /// An object at `stage`, in the pass at hand or the position's.
    fn at(&self, id: ObjectId, stage: &Stage) -> Chars {
        let owned;
        let pass = match self.pass {
            Some(pass) => pass,
            None => {
                owned = pass(self.cards, self.state, self.log);
                &owned
            }
        };
        chars_at(&Ctx { cards: self.cards, state: self.state, log: self.log, pass }, id, stage, false).chars
    }
}

impl LayerQuery for View<'_> {
    fn state(&self) -> &GameState {
        self.state
    }

    /// The static ability being asked, where an effect gave it `source` (CR 607.5: linked only to the abilities that effect
    /// gave it); else an ability of the text `source` has.
    fn linked_to(&self, source: ObjectId) -> Linked {
        match self.asking {
            Some(by) if by.object == source => by,
            _ => self.cards.linked_to(self.state, source),
        }
    }

    fn cards(&self) -> &CardTable {
        self.cards
    }

    fn history(&self) -> History<'_> {
        History::new(self.log, self.state.turn.number)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        if let Some(reads) = self.reads {
            reads.borrow_mut().push(id);
        }
        match &self.stage {
            Some(stage) => self.at(id, stage),
            None => self.before(id, self.stop),
        }
    }

    /// Another object with its own characteristic-defining abilities of power and toughness applied (layer 7a): read at the
    /// stage where the layer being applied is 7a or earlier - what 7a makes of an object is not yet made in an earlier layer,
    /// and in 7a an object's own such abilities are its layer's start - and as 7a left it after.
    fn chars_with_defined_pt(&self, id: ObjectId) -> Chars {
        if let Some(reads) = self.reads {
            reads.borrow_mut().push(id);
        }
        let seventh = LAYERS.iter().position(|&l| l == Layer::PtDefine).unwrap_or(0);
        // An object whose own characteristic-defining abilities of 7a are being applied, asked again from within them (two
        // such abilities each reading the other's object - no printed card does): it is answered as it stands before them,
        // so the question ends.
        if DEFINING.with(|d| d.borrow().contains(&id)) {
            return self.before(id, Layer::PtDefine);
        }
        match (self.stop.cmp(&Layer::PtDefine), &self.stage) {
            (std::cmp::Ordering::Greater, _) => self.before(id, Layer::PtSet),
            (_, Some(stage)) => self.at(id, stage),
            (std::cmp::Ordering::Equal, None) => {
                let start = Stage { li: seventh, entries: &[], order: &[], len: 0, extra: None, keep: true };
                self.at(id, &start)
            }
            (_, None) => self.before(id, self.stop),
        }
    }

    fn controller(&self, id: ObjectId) -> PlayerId {
        match self.controllers.and_then(|c| c.get(&id)) {
            Some(&who) => who,
            None => self.state.obj(id).controller,
        }
    }

    /// The words of an object's text (CR 612), a read of the object like any other (recorded as `chars` records). The words come
    /// from the text-changing effects of the state in their own order (`cantrip_api::words`), which no entry of a layer
    /// after 3 changes, so they are the same at every stage that reads them.
    fn words(&self, id: ObjectId) -> Words {
        if let Some(reads) = self.reads {
            reads.borrow_mut().push(id);
        }
        cantrip_api::words(self.state, id)
    }
}

const LAYERS: [Layer; 8] =
    [Layer::Text, Layer::Type, Layer::Color, Layer::Ability, Layer::PtDefine, Layer::PtSet, Layer::PtModify, Layer::PtSwitch];

/// The characteristics with every layer before `stop` applied (CR 613.1): the copiable values, then each layer's
/// static abilities and other continuous effects in turn, in the order the layer's pass gives them (CR 613.7, 613.8).
pub fn chars_before(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId, stop: Layer) -> Chars {
    let layers = LAYERS.iter().take_while(|&&l| l < stop).count();
    match object(cards, state, id) {
        Object::Fixed(chars) => *chars,
        Object::Bare(card, spell) => bare(cards, card, spell, stop),
        Object::Full(o) => {
            if layers == 0 {
                return o.start(cards);
            }
            let pass = pass(cards, state, log);
            let cx = Ctx { cards, state, log, pass: &pass };
            // While its abilities' givers are being worked out (`ability_givers`), the object is worked out afresh, watched.
            let watch = watched_givers(id).is_some();
            // Nothing in any layer: what the rules give the object alone, nothing kept.
            if layers_with_entries(&cx) == 0 {
                let mut snap = Snap { chars: o.start(cards), parts: Vec::new() };
                for li in 0..layers {
                    through(&cx, &o, li, &mut snap, watch);
                }
                return snap.chars;
            }
            let reads = cantrip_api::history::reads_total();
            let chars = before(&cx, &o, layers, watch).chars;
            // A static ability asked about the past: what the pass holds is for this history.
            if cantrip_api::history::reads_total() != reads && pass.log.get().is_none() {
                pass.log.set(Some(log.len()));
            }
            if verifying() && !watch {
                cross_check(cards, state, log, &o, layers, &chars, pass.built_at != state.epoch);
            }
            chars
        }
    }
}

/// `chars_before` within a pass: what every read inside the layers uses, so that a pass of its own (`cross_check`) is fresh
/// all through.
fn chars_in(cx: &Ctx, id: ObjectId, stop: Layer) -> Chars {
    let layers = LAYERS.iter().take_while(|&&l| l < stop).count();
    match object(cx.cards, cx.state, id) {
        Object::Fixed(chars) => *chars,
        Object::Bare(card, spell) => bare(cx.cards, card, spell, stop),
        Object::Full(o) if layers == 0 => o.start(cx.cards),
        Object::Full(o) => before(cx, &o, layers, watched_givers(id).is_some()).chars,
    }
}

/// With `CANTRIP_VERIFY`, one answer in `CROSS_CHECK_STRIDE`, and every answer from a pass made at another `epoch`, is worked
/// out again in a pass of its own, from nothing kept: the characteristics an object has are a function of the position,
/// whatever was asked of the pass before.
fn cross_check(cards: &CardTable, state: &GameState, log: &[Record], o: &ObjCtx, layers: usize, kept: &Chars, reused: bool) {
    const CROSS_CHECK_STRIDE: u32 = 16;
    thread_local! {
        static COUNT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        static CHECKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    // Every answer from a pass made at another `epoch` - kept across writes `Undo::layered` calls no change to a
    // characteristic - is checked, so a write misclassified there is found.
    if CHECKING.with(|c| c.get()) || (!reused && !COUNT.with(|c| c.replace(c.get().wrapping_add(1))).is_multiple_of(CROSS_CHECK_STRIDE)) {
        return;
    }
    CHECKING.with(|c| c.set(true));
    let fresh = Pass::new(PassKey { epoch: state.chars_epoch, judged: state.judged_face_up.get(), judged_as: state.judged_as.get() }, state.epoch);
    let again = before(&Ctx { cards, state, log, pass: &fresh }, o, layers, false).chars;
    CHECKING.with(|c| c.set(false));
    assert!(again == *kept, "the layers' pass kept other characteristics for {:?} than a pass of its own works out", o.id);
}

/// An object no continuous effect, counter or static ability could act on: the card's own values, made once (as
/// `chars_before` gives them before `stop`).
fn bare(cards: &CardTable, card: CardId, spell: bool, stop: Layer) -> Chars {
    let mut chars = *printed(cards, card, stop > Layer::Ability);
    chars.spell = spell;
    chars
}

/// What an object is to the layers.
enum Object<'a> {
    /// Characteristics nothing changes: a card exiled face down (none, CR 406.3a), or an object that has ceased to be
    /// itself (as it last was, CR 400.7, 608.2h, 603.10).
    Fixed(Box<Chars>),
    /// Nothing acts on it: its card's own values (`bare`).
    Bare(CardId, bool),
    Full(ObjCtx<'a>),
}

/// The object being worked out, and what about it the layers read again and again.
struct ObjCtx<'a> {
    id: ObjectId,
    obj: &'a Obj,
    card: CardId,
    def: &'static CardDef,
    copy: Option<&'a Copiable>,
    room: bool,
    spell: bool,
    on_battlefield: bool,
    has_counters: bool,
    has_hone: bool,
}

impl ObjCtx<'_> {
    /// The copiable values (CR 707.2), with the Room's doors (CR 709.5).
    fn start(&self, cards: &CardTable) -> Chars {
        let mut chars = match self.copy {
            Some(c) => initial(cards, self.card, Some(c)),
            None => *printed(cards, self.card, false),
        };
        chars.spell = self.spell;
        if self.room {
            doors(cards, self.card, self.obj.marks.doors, self.copy, &mut chars);
        }
        chars
    }
}

fn object<'a>(cards: &CardTable, state: &'a GameState, id: ObjectId) -> Object<'a> {
    let obj = state.obj(id);
    // CR 406.3a: a card exiled face down has no characteristics, unless a player who may play it is being asked whether
    // they could (`judged_face_up`).
    if obj.face_down_card() && state.judged_face_up.get() != Some(id) {
        return Object::Fixed(Box::new(*printed(cards, cards.no_characteristics, false)));
    }
    // A card judged as the face it would be cast as (`chars_as_face`): a spell with that face's values.
    let judged_as = state.judged_as.get().filter(|&(card, _)| card == id).map(|(_, face)| face);
    let spell = obj.zone.is(ZoneKind::Stack) || judged_as.is_some();
    // The copiable values (CR 707.2): a copy effect's (layer 1a) under a face-down permanent's face-down values (layer 1b,
    // CR 613.2b), or the face of the card that is up (CR 711.2).
    // The face-down values are a card's own printed values (no exceptions), so a face-down permanent starts from that card
    // as an object with no copy effect does. A face-down permanent judged as it would be face up (`Query::chars_if_face_up`) has
    // its normal copiable values, under the effects that apply to it (CR 708.8).
    let face_down_as = obj.marks.face_down_as.filter(|_| state.judged_face_up.get() != Some(id));
    let copy: Option<&Copiable> = if face_down_as.is_some() || judged_as.is_some() { None } else { obj.copy.as_ref() };
    // A double-faced token's own copy values (CR 707.8a) are its card's faces, the one up, under its exceptions.
    let card = match (judged_as, face_down_as, copy) {
        (Some(face), _, _) => face,
        (None, Some(values), _) => values,
        (None, None, Some(c)) if !obj.own_faces(c) => c.card,
        (None, None, _) => cards.face(obj.card, obj.transformed),
    };
    let def = cards.get(card);
    // A Room on the battlefield (CR 709.5): the name, mana cost and abilities of its unlocked doors alone, which no
    // card-only cache can hold, as they are the object's. In every other zone the card has both halves (CR 709.4).
    let room = def.is_room() && obj.zone.is(ZoneKind::Battlefield);
    let o = ObjCtx { id, obj, card, def, copy, room, spell, on_battlefield: false, has_counters: false, has_hone: false };
    // An object that has ceased to be itself - changed zones or ceased to exist - is as it last was (CR 400.7, 608.2h,
    // 603.10).
    if let Some(last) = state.last_known(id) {
        return Object::Fixed(Box::new(o.start(cards).as_known(last)));
    }
    // A phased-out permanent keeps its counters and the effects on it (CR 702.26d, 702.26f), so it is worked out as it is on
    // the battlefield, though no rule asks while it is phased out: what it shows, and what it is again as it phases in.
    let on_battlefield = (obj.zone.is(ZoneKind::Battlefield) || obj.zone.is(ZoneKind::PhasedOut)) && obj.became.is_none();
    // What could apply at all, found once: most objects have no effects, no statics touch them, and no counters.
    // A spell's text can be changed too (CR 612.1: "target spell or permanent").
    // The effects on a card do not follow it to the stack (CR 400.7): a card judged as a spell has none.
    let has_effects = (on_battlefield || spell) && judged_as.is_none() && state.effects.iter().any(|e| e.affected == id);
    let has_counters = on_battlefield && obj.has_counters();
    let has_own = !def.statics.is_empty();
    // `statics_seen` is set by hone counters as well (CR 122.1j), which need no card with a static ability.
    let (any_sources, has_hone) = if state.statics_seen {
        let found = static_sources(cards, state);
        let has_hone = on_battlefield && found.honed.iter().any(|&e| state.obj(e).attached_to == Some(id));
        (!found.list.is_empty() || !found.granted.is_empty(), has_hone)
    } else {
        (false, false)
    };
    if !has_effects && !has_counters && !has_own && !any_sources && !has_hone {
        if copy.is_none() && !spell && !room {
            return Object::Bare(card, false);
        }
        if copy.is_none() && !room {
            return Object::Bare(card, true);
        }
    }
    Object::Full(ObjCtx { on_battlefield, has_counters, has_hone, ..o })
}

// --- The layer pass (CR 613.7, 613.8) ------------------------------------------------------------------------------------

/// The layers' work for one position and history: each layer's application order, fixed once over every object (CR 613.8:
/// "an effect is said to depend on another if ... applying the other would change ... what it applies to, or what it does
/// to any of the things it applies to"; that is a question about the game, not about one object), and the characteristics
/// worked out with it. Made when first needed, kept on the state (`GameState::layer_pass`) for as long as the position and
/// history are the same. A copy of the state starts with none (`LayerCache`'s `Clone`): no pass is shared between
/// games.
struct Pass {
    key: PassKey,
    /// The state's `epoch` the pass was made at: a pass used at another is checked under `CANTRIP_VERIFY` (`chars_before`).
    built_at: u64,
    /// The length of the history it is for, once anything worked out in it has asked the past: until then it is for any.
    log: std::cell::Cell<Option<usize>>,
    plans: std::cell::RefCell<[Option<std::rc::Rc<Plan>>; LAYERS.len()]>,
    /// The layers whose plans are being made, by index.
    building: std::cell::Cell<u16>,
    /// The layers some entry could be in, by index, once found (`layers_with_entries`).
    occupied: std::cell::Cell<Option<u16>>,
    /// Whether anything could take an object's abilities, once found (`strips`).
    strips: std::cell::Cell<Option<bool>>,
    /// Characteristics worked out: (object, layer index, step, entry applied beyond the step) to what they are there, and
    /// (object, LAYERS.len() + layers done, 0, 0) to what they are once that many layers are done.
    memo: std::cell::RefCell<Memo>,
    /// For each object, what it is once each number of layers is done.
    done: std::cell::RefCell<std::collections::HashMap<ObjectId, [Option<std::rc::Rc<Snap>>; LAYERS.len() + 1], Quick>>,
    /// For each object and layer index, the furthest step kept in `memo`.
    furthest: std::cell::RefCell<std::collections::HashMap<(ObjectId, u8), u32, Quick>>,
}

/// A quick hash for the pass's small keys (the default one guards against inputs chosen to collide, which these are not).
#[derive(Clone, Copy, Default)]
struct Quick;

impl std::hash::BuildHasher for Quick {
    type Hasher = QuickHasher;
    fn build_hasher(&self) -> QuickHasher {
        QuickHasher(0)
    }
}

struct QuickHasher(u64);

impl std::hash::Hasher for QuickHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u8(&mut self, n: u8) {
        self.write_u64(n as u64);
    }
    fn write_u16(&mut self, n: u16) {
        self.write_u64(n as u64);
    }
    fn write_u32(&mut self, n: u32) {
        self.write_u64(n as u64);
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

impl Pass {
    fn new(key: PassKey, built_at: u64) -> Pass {
        Pass {
            key,
            built_at,
            log: std::cell::Cell::new(None),
            plans: std::cell::RefCell::new(Default::default()),
            building: std::cell::Cell::new(0),
            occupied: std::cell::Cell::new(None),
            strips: std::cell::Cell::new(None),
            memo: std::cell::RefCell::new(std::collections::HashMap::default()),
            furthest: std::cell::RefCell::new(std::collections::HashMap::default()),
            done: std::cell::RefCell::new(std::collections::HashMap::default()),
        }
    }
}

/// The counts in these keys - a layer's entries, its steps - are `u32`: no layer of a game the engine plays to its end
/// (`MAX_GAME_EVENTS`, 2^21 events, RX-1) can have as many entries as that holds, so they never wrap.
/// The pass's characteristics worked out, by (object, layer, step, entry beyond the step).
type Memo = std::collections::HashMap<(ObjectId, u8, u32, u32), std::rc::Rc<Snap>, Quick>;

/// What a pass is made for: the position as the layers read it (`GameState::chars_epoch`, which every write and undo that
/// could change a characteristic moves), and the face-down card being judged as the card it is, if one is
/// (`judged_face_up`): what is worked out while it is judged is kept apart. The history is the pass's `log`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct PassKey {
    epoch: u64,
    judged: Option<ObjectId>,
    judged_as: Option<(ObjectId, CardId)>,
}

/// An object's characteristics at a point of the layers, with the parts of effects (CR 613.6) that began to apply to it.
#[derive(Clone, PartialEq)]
struct Snap {
    chars: Chars,
    parts: Vec<(ObjectId, *const StaticDef)>,
}

/// One layer's entries, in timestamp order (CR 613.7), and the order the layer applies them in (CR 613.8).
struct Plan {
    entries: Vec<GEntry>,
    order: Vec<u32>,
}

/// The work under way: the state, the history, and the pass for them.
struct Ctx<'a> {
    cards: &'a CardTable,
    state: &'a GameState,
    log: &'a [Record],
    pass: &'a Pass,
}

/// A point of a layer: `len` entries of `order` applied, and then `extra` (an entry a dependency is judged with) if any.
#[derive(Clone, Copy)]
struct Stage<'a> {
    li: usize,
    entries: &'a [GEntry],
    order: &'a [u32],
    len: usize,
    extra: Option<u32>,
    /// Whether what is worked out here may be kept: not in a plan made only for a question asked while the layer's own plan
    /// was being made.
    keep: bool,
}

/// Whether `CANTRIP_VERIFY` is set: the layers check themselves.
pub(crate) fn verifying() -> bool {
    static VERIFY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *VERIFY.get_or_init(|| std::env::var_os("CANTRIP_VERIFY").is_some())
}

/// The pass for this position and history, made if there is none.
fn pass(cards: &CardTable, state: &GameState, log: &[Record]) -> std::rc::Rc<Pass> {
    let _ = cards;
    let key = PassKey { epoch: state.chars_epoch, judged: state.judged_face_up.get(), judged_as: state.judged_as.get() };
    {
        let cache = state.layer_pass.0.borrow();
        for kept in cache.iter().rev() {
            if let Ok(found) = std::rc::Rc::downcast::<Pass>(kept.clone())
                && found.key == key
                && found.log.get().is_none_or(|len| len == log.len())
            {
                // What the pass holds was worked out asking the past: using it again asks the past again, though nothing
                // reads the history this time - which the rules that find a loop of passes count (`note_read`, `loops`).
                if found.log.get().is_some() {
                    cantrip_api::history::note_read();
                }
                return found;
            }
        }
    }
    let made = std::rc::Rc::new(Pass::new(key, state.epoch));
    let mut cache = state.layer_pass.0.borrow_mut();
    // Passes for other positions are of no more use; those for other lengths of the history are kept beside this one.
    cache.retain(|kept| std::rc::Rc::downcast::<Pass>(kept.clone()).is_ok_and(|p| p.key.epoch == key.epoch));
    // A handful at most: one per history length the layers were asked about at this position.
    if cache.len() >= 8 {
        cache.remove(0);
    }
    cache.push(made.clone());
    made
}

/// The object with its first `layers` layers done (`chars_before` of the layer after them): from the furthest layer kept,
/// each layer in turn, and only the last kept.
fn before(cx: &Ctx, o: &ObjCtx, layers: usize, watch: bool) -> std::rc::Rc<Snap> {
    let mut from = 0;
    let mut snap = None;
    if !watch && let Some(kept) = cx.pass.done.borrow().get(&o.id) {
        for k in (1..=layers).rev() {
            if let Some(found) = &kept[k] {
                from = k;
                snap = Some(found.clone());
                break;
            }
        }
    }
    if from == layers
        && let Some(snap) = snap
    {
        return snap;
    }
    let mut snap = snap.map_or_else(|| Snap { chars: o.start(cx.cards), parts: Vec::new() }, |s| (*s).clone());
    for li in from..layers {
        through(cx, o, li, &mut snap, watch);
    }
    let snap = std::rc::Rc::new(snap);
    if !watch && layers > 0 {
        cx.pass.done.borrow_mut().entry(o.id).or_insert_with(Default::default)[layers] = Some(snap.clone());
    }
    snap
}

/// The object through layer `li`, from `snap`, what the layers before it left it.
fn through(cx: &Ctx, o: &ObjCtx, li: usize, snap: &mut Snap, watch: bool) {
    starting(cx, o, li, snap);
    let plan = plan(cx, li);
    if !plan.order.is_empty() {
        let stage = Stage { li, entries: &plan.entries, order: &plan.order, len: plan.order.len(), extra: None, keep: true };
        apply_steps(cx, o, &stage, 0, snap, watch);
    }
    // CR 702.147a, 116.2b, 709.5e: what the rules give an object for what layer 6 left it.
    if LAYERS[li] == Layer::Ability {
        intrinsic_keyword_abilities(cx.cards, &mut snap.chars);
        if o.obj.face_down && !o.obj.face_down_card() && cx.state.judged_face_up.get() != Some(o.id) {
            turn_face_up_actions(cx.cards, o.obj, &mut snap.chars);
        }
        if o.room {
            unlock_actions(cx.cards, o.obj.marks.doors, &mut snap.chars);
        }
    }
}

/// Any object at `stage` of its layer (what `View::chars` reads while the layer is applied or judged).
fn chars_at(cx: &Ctx, id: ObjectId, stage: &Stage, watch: bool) -> std::rc::Rc<Snap> {
    match object(cx.cards, cx.state, id) {
        Object::Fixed(chars) => std::rc::Rc::new(Snap { chars: *chars, parts: Vec::new() }),
        Object::Bare(card, spell) => std::rc::Rc::new(Snap { chars: bare(cx.cards, card, spell, LAYERS[stage.li]), parts: Vec::new() }),
        Object::Full(o) => at(cx, &o, stage, watch),
    }
}

/// The object at `stage`: as the layers before left it, with what the rules give it at the layer's start and its own
/// characteristic-defining abilities (CR 613.3), and then the entries of the stage that apply to it, each as it stands.
fn at(cx: &Ctx, o: &ObjCtx, stage: &Stage, watch: bool) -> std::rc::Rc<Snap> {
    let keep = stage.keep && !watch;
    let key = |len: usize, extra: Option<u32>| (o.id, stage.li as u8, len as u32, extra.map_or(0, |e| e + 1));
    if keep && let Some(found) = cx.pass.memo.borrow().get(&key(stage.len, stage.extra)) {
        return found.clone();
    }
    // From the furthest step kept no further than the stage, or the layer's start.
    let mut from = 0;
    let mut snap = None;
    if keep {
        let furthest = cx.pass.furthest.borrow().get(&(o.id, stage.li as u8)).copied();
        if let Some(len) = furthest {
            let len = (len as usize).min(stage.len);
            if let Some(found) = cx.pass.memo.borrow().get(&key(len, None)) {
                from = len;
                snap = Some((**found).clone());
            }
        }
    }
    let mut snap = match snap {
        Some(snap) => snap,
        None => {
            let mut start = (*before(cx, o, stage.li, watch)).clone();
            starting(cx, o, stage.li, &mut start);
            if keep {
                cx.pass.memo.borrow_mut().insert(key(0, None), std::rc::Rc::new(start.clone()));
                cx.pass.furthest.borrow_mut().entry((o.id, stage.li as u8)).or_insert(0);
            }
            start
        }
    };
    // Every step on the way is kept, so a later stage starts where this one ends.
    if keep && stage.extra.is_none() {
        for j in from..stage.len {
            let here = Stage { len: j + 1, ..*stage };
            apply_steps(cx, o, &Stage { len: j + 1, ..*stage }, j, &mut snap, watch);
            let _ = here;
            cx.pass.memo.borrow_mut().insert(key(j + 1, None), std::rc::Rc::new(snap.clone()));
        }
        let mut furthest = cx.pass.furthest.borrow_mut();
        let kept = furthest.entry((o.id, stage.li as u8)).or_insert(0);
        *kept = (*kept).max(stage.len as u32);
        drop(furthest);
        return cx.pass.memo.borrow().get(&key(stage.len, None)).cloned().unwrap_or_else(|| std::rc::Rc::new(snap));
    }
    apply_steps(cx, o, stage, from, &mut snap, watch);
    let snap = std::rc::Rc::new(snap);
    if keep {
        cx.pass.memo.borrow_mut().insert(key(stage.len, stage.extra), snap.clone());
    }
    snap
}
/// Applies the entries of `stage` from step `from` on, and its extra entry, to the object.
fn apply_steps(cx: &Ctx, o: &ObjCtx, stage: &Stage, from: usize, snap: &mut Snap, watch: bool) {
    // A card outside the game is affected by nothing but its own characteristic-defining abilities (CR 400.11c).
    if o.obj.zone.is(ZoneKind::Sideboard) {
        return;
    }
    let noting = cx.state.statics_seen && static_sources(cx.cards, cx.state).continuing;
    for j in from..stage.len {
        let entry = &stage.entries[stage.order[j] as usize];
        let here = Stage { len: j, extra: None, ..*stage };
        apply_entry(cx, o, entry, &here, snap, noting, watch);
    }
    if let Some(extra) = stage.extra {
        let here = Stage { extra: None, ..*stage };
        apply_entry(cx, o, &stage.entries[extra as usize], &here, snap, noting, false);
    }
}

/// The object as the layers before `li` leave it (`snap`), with what the rules give it as layer `li` begins (a basic land type's
/// mana ability and a Siege's defeat ability in layer 6, counters in 7c) and its own characteristic-defining abilities of the
/// layer, which apply first (CR 613.3).
fn starting(cx: &Ctx, o: &ObjCtx, li: usize, snap: &mut Snap) {
    let layer = LAYERS[li];
    let chars = &mut snap.chars;
    // CR 305.6: the abilities a basic land type carries follow from the types layer 4 settled, as the object's
    // own from the start of layer 6.
    if layer == Layer::Ability && chars.is(CardType::Land) {
        intrinsic_mana(cx.cards, o.def, chars);
    }
    if layer == Layer::Ability && chars.is(CardType::Battle) {
        intrinsic_siege(cx.cards, chars);
    }
    if o.has_counters && layer == Layer::PtModify {
        counters(o.obj, chars);
    }
    if o.has_hone && layer == Layer::PtModify {
        let honed = static_sources(cx.cards, cx.state).honed.clone();
        hone(cx, o.id, &honed, chars);
    }
    own_statics(cx, o, layer, chars);
}

thread_local! {
    /// The objects whose own characteristic-defining abilities of layer 7a are being applied (`View::chars_with_defined_pt`).
    static DEFINING: std::cell::RefCell<Vec<ObjectId>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The object's own characteristic-defining abilities of `layer` (CR 604.3, 613.3): they apply first, and depend only on
/// one another (CR 613.8a(c)).
fn own_statics(cx: &Ctx, o: &ObjCtx, layer: Layer, chars: &mut Chars) {
    let mut own = o.def.statics.iter().filter(|s| s.own && s.layer == layer).peekable();
    if own.peek().is_none() {
        return;
    }
    // CR 707.9d: a copy that states its colors or its power and toughness does not copy a characteristic-defining
    // ability that would define them.
    let colors = o.copy.and_then(|c| c.colors).is_some();
    let pt = o.copy.and_then(|c| c.pt()).is_some();
    if (layer == Layer::Color && colors) || (layer == Layer::PtDefine && pt) {
        return;
    }
    let view = View { pass: Some(cx.pass), ..View::new(cx.cards, cx.state, cx.log, layer) };
    // CR 604.3, 613.1f: its own characteristic-defining abilities are static abilities, so layer 6 takes them away, and the layers
    // after it (a power and toughness one, 613.4a) find them gone. Those of layers 4 and 5 were applied before, and stay (CR 613.6).
    // CR 305.7: a land whose subtype an effect sets has lost them in layer 4, after its own characteristic-defining
    // abilities of layer 4 applied first (CR 613.3).
    let lost = (layer >= Layer::Ability && chars.lost_all) || (layer > Layer::Type && chars.text_lost);
    let defining = layer == Layer::PtDefine;
    if defining {
        DEFINING.with(|d| d.borrow_mut().push(o.id));
    }
    for st in own {
        if lost {
            if let Some((power, toughness)) = st.lost_pt {
                chars.power = Some(power);
                chars.toughness = Some(toughness);
            }
        } else {
            (st.apply)(&view, o.id, chars);
        }
    }
    if defining {
        DEFINING.with(|d| d.borrow_mut().pop());
    }
}


/// Each kind of keyword counter on the object (CR 122.1b) as the layer-6 effect it is, "gains [keyword]", at the timestamp
/// of its counters (CR 613.7c), put among the effects in timestamp order.
fn keyword_counters(state: &GameState, id: ObjectId, obj: &Obj, fx: &mut Vec<ContinuousEffect>) {
    for kind in CounterKind::KEYWORDS {
        if obj.counters(kind) > 0
            && let Some(k) = kind.keyword()
        {
            let timestamp = state.counter_stamp(id, kind);
            let at = fx.partition_point(|e| e.timestamp <= timestamp);
            fx.insert(
                at,
                ContinuousEffect {
                    source: id,
                    controller: obj.controller,
                    timestamp,
                    affected: id,
                    kind: EffectKind::AddKeywords(Keywords::of(&[k])),
                    duration: Duration::Indefinite,
                },
            );
        }
    }
}

/// The abilities the rules give an object for a keyword it has once layer 6 is done (CR 702.147a): decayed's triggered
/// ability, whether the keyword is printed, granted or a counter's. Its static half ("this creature can't block") is read
/// off the keyword by the blocking rule.
fn intrinsic_keyword_abilities(cards: &CardTable, chars: &mut Chars) {
    if chars.keywords.has(Keyword::Decayed)
        && let Some(slot) = chars.abilities.iter().position(|a| a.is_none())
    {
        chars.abilities[slot] = Some(AbilityRef { card: cards.intrinsic, index: cantrip_api::intrinsic::DECAYED });
    }
}

/// Turning a face-down permanent face up is a special action the rules give it (CR 116.2b, 701.40b, 701.58b, 702.37e,
/// 702.168d), not an ability it has, so an effect that takes its abilities ("creatures lose all abilities": Humility, Dress
/// Down) does not take it (Whisperwood Elemental's ruling). Its face-down values carry each one it may be turned up by
/// (`CardTable::turn_up`), which layer 6 may have removed: they are given back once layer 6 is done.
fn turn_face_up_actions(cards: &CardTable, obj: &Obj, chars: &mut Chars) {
    let Some(values) = obj.marks.face_down_as else { return };
    for index in 0..cards.get(values).abilities.len() as u8 {
        let action = AbilityRef { card: values, index };
        if !cards.turns_face_up(action) || chars.abilities.contains(&Some(action)) {
            continue;
        }
        if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
            chars.abilities[slot] = Some(action);
        }
    }
}

/// The static abilities effects have given the object (CR 613.1f) that it has once layer 6 is done, oldest first, each with the
/// timestamp its effect has (CR 613.7a): the later of the object's own and that of the effect that granted the ability.
/// "Loses all abilities" (CR 613.1f) at a later timestamp takes the ones granted before it, and a static ability that took
/// abilities away has taken them from `chars` already: only what the object still has is kept. A grant of the same ability
/// twice is two effects, each with its own timestamp. Guarded on the object's own effects, so an object nothing granted
/// anything to costs nothing here.
fn granted_statics(cards: &CardTable, object_timestamp: u32, fx: &[ContinuousEffect], chars: &Chars) -> Vec<(u32, &'static StaticDef, AbilityRef, Giver)> {
    let mut granted: Vec<(u32, AbilityRef, Giver)> = Vec::new();
    for e in fx {
        match e.kind {
            // Its own: one that affects other objects applies from it as a source (`granted_to_others`).
            EffectKind::AddAbility(ability) if matches!(cards.ability(ability), AbilityDef::Static(s) if s.own) => {
                granted.push((e.timestamp.max(object_timestamp), ability, Giver::of_effect(e)));
            }
            EffectKind::LoseAllAbilities => granted.clear(),
            _ => {}
        }
    }
    granted
        .into_iter()
        .filter(|(_, ability, _)| chars.abilities.contains(&Some(*ability)))
        .filter_map(|(timestamp, ability, giver)| match cards.ability(ability) {
            AbilityDef::Static(s) => Some((timestamp, s, ability, giver)),
            _ => None,
        })
        .collect()
}

/// The permanents (and graveyard cards) whose static abilities could apply to anything, for the state's current epoch:
/// found once per position, whatever the number of questions asked of it.
/// Whether a permanent (or other source) whose static abilities function now gives other objects abilities (layer 6):
/// a source that has not lost its abilities to a resolved effect, whose card - or the card it copies, or its face - has such
/// a static ability.
pub(crate) fn ability_grants_present(cards: &CardTable, state: &GameState) -> bool {
    static_sources(cards, state).list.iter().any(|&(id, _, lost)| !lost && cards.ability_grants[cards.current(state.obj(id)).0 as usize])
}

fn static_sources<'a>(cards: &CardTable, state: &'a GameState) -> std::cell::Ref<'a, cantrip_state::StaticSources> {
    {
        let have = state.static_sources.borrow();
        if have.epoch == state.epoch {
            return have;
        }
    }
    {
        let mut made = state.static_sources.borrow_mut();
        made.list.clear();
        made.strippers.clear();
        made.setters.clear();
        made.controllers.clear();
        made.honed.clear();
        made.targeting.clear();
        // A permanent that has lost all its abilities to an effect keeps the effects its static abilities had applied in
        // layers 1 to 5 by the time layer 6 took them (CR 613.6, Humility with Magus of the Moon); from layer 6 on it has
        // none (CR 613.1f, 613.8a).
        let note = |list: &mut Vec<(ObjectId, ZoneKind, bool)>, source: ObjectId, from: ZoneKind| {
            let sobj = state.obj(source);
            let scard = cards.current(sobj);
            if cards.statics[scard.0 as usize] && live(state, source) {
                list.push((source, from, state.lost_all_abilities_to_an_effect(source)));
            }
        };
        for &source in state.battlefield() {
            note(&mut made.list, source, ZoneKind::Battlefield);
            // CR 122.1j: hone counters act on what their permanent is attached to, abilities or none.
            if state.obj(source).counters(CounterKind::Hone) > 0 && live(state, source) {
                made.honed.push(source);
            }
            if cards.targeting_rules[cards.current(state.obj(source)).0 as usize] && live(state, source) {
                made.targeting.push(source);
            }
        }
        // An emblem's static abilities function in the command zone (CR 114.4).
        for &source in state.zone(Zone::COMMAND) {
            note(&mut made.list, source, ZoneKind::Battlefield);
        }
        if cards.graveyard_statics {
            for p in state.seats() {
                for &source in state.zone(Zone::graveyard(p)) {
                    note(&mut made.list, source, ZoneKind::Graveyard);
                }
            }
        }
        // A card encoded on a creature functions in exile (CR 702.99a).
        if cards.exile_statics {
            for p in state.seats() {
                // A card exiled face down has no abilities (CR 406.3a).
                for &source in state.zone(Zone::exile(p)).iter().filter(|&&source| !state.obj(source).face_down) {
                    note(&mut made.list, source, ZoneKind::Exile);
                }
            }
        }
        // CR 613.7: static abilities apply in the timestamp order of the objects that have them.
        made.list.sort_by_key(|&(source, _, _)| state.obj(source).timestamp);
        let (mut strippers, mut setters, mut controllers) = (Vec::new(), Vec::new(), Vec::new());
        for &(source, from, lost) in &made.list {
            let statics = cards.get(cards.current(state.obj(source))).statics;
            // CR 613.1b: a control-changing ability applies in layer 2, before layer 6 could take it.
            if from == ZoneKind::Battlefield && state.obj(source).zone.is(ZoneKind::Battlefield) && statics.iter().any(|s| s.layer == Layer::Control) {
                controllers.push(source);
            }
            if !lost && statics.iter().any(|s| s.loses_all && s.from == from) {
                strippers.push((source, from));
            }
            // CR 305.7, 613.6: an ability that sets land subtypes applies in layer 4, whatever its source loses in layer 6.
            if statics.iter().any(|s| s.sets_land_subtypes && s.from == from) {
                setters.push((source, from));
            }
        }
        made.strippers = strippers;
        made.continuing = made.list.iter().any(|&(source, _, _)| cards.get(cards.current(state.obj(source))).statics.iter().any(|s| s.continues));
        made.setters = setters;
        made.controllers = controllers;
        made.granted.clear();
        for e in &state.effects {
            if let EffectKind::AddAbility(ability) = e.kind
                && matches!(cards.ability(ability), AbilityDef::Static(s) if !s.own)
                && live(state, e.affected)
                && state.obj(e.affected).zone.is(ZoneKind::Battlefield)
                && !made.granted.contains(&e.affected)
            {
                made.granted.push(e.affected);
            }
        }
        made.epoch = state.epoch;
    }
    state.static_sources.borrow()
}

/// The copiable values of `card` as an object has them before any layer applies: the card's own, or a copy effect's
/// (`copy`), with the abilities the card prints.
fn initial(cards: &CardTable, card: CardId, copy: Option<&Copiable>) -> Chars {
    let def = cards.get(card);
    let (extra_types, extra_subtypes, extra_keywords, colors, pt, no_cost, copy_name, words) = match copy {
        Some(c) => (c.extra_types, c.extra_subtypes, c.extra_keywords, c.colors, c.pt(), c.has_no_cost(), c.name, c.words),
        None => (TypeSet::NONE, SubtypeSet::NONE, Keywords::NONE, None, None, false, None, Words::NONE),
    };
    // "Except it's legendary" or "except it isn't legendary" (CR 707.9b).
    let legendary = copy.and_then(|c| c.legendary_stated()).unwrap_or(def.supertypes.legendary);
    // "Except it's a 5/5 black Demon" (CR 707.9b, 205.1a): the types and subtypes the exception states, instead of the
    // card's; an instant or sorcery stays one, with its spell types. Changeling, which defines the subtypes it replaced,
    // is not copied (CR 707.9d), though an exception may give it again.
    let stated = copy.is_some_and(|c| c.types_stated());
    let (types, subtypes, printed_keywords) = if stated {
        let spell = TypeSet(def.types.0 & TypeSet::of(&[CardType::Instant, CardType::Sorcery]).0);
        let spell_types = if spell.0 != 0 {
            def.subtypes.iter().filter(|s| !s.is_creature_type()).fold(SubtypeSet::NONE, SubtypeSet::with)
        } else {
            SubtypeSet::NONE
        };
        (TypeSet(spell.0 | extra_types.0), spell_types.union(extra_subtypes), Keywords(def.printed_keywords().0 & !Keywords::of(&[Keyword::Changeling]).0))
    } else {
        (TypeSet(def.types.0 | extra_types.0), words.subtypes(def.subtypes).union(extra_subtypes), def.printed_keywords())
    };
    let mut abilities = [None; MAX_ABILITIES];
    let mut n = 0;
    // A dungeon or the rules' own card has more abilities than any object does; nothing reads them as an object's.
    for index in 0..def.abilities.len().min(MAX_ABILITIES) {
        abilities[n] = Some(AbilityRef { card, index: index as u8 });
        n += 1;
    }
    for extra in copy.iter().flat_map(|c| c.extra_instances()).take(MAX_ABILITIES - n) {
        abilities[n] = Some(extra);
        n += 1;
    }
    // An Aura's printed enchant ability (CR 702.5a), after the abilities its text lists so their positions stay.
    if def.enchant.is_some() && n < MAX_ABILITIES {
        abilities[n] = Some(AbilityRef::enchant(card));
    }
    let mut chars = Chars {
        card,
        // A token's name is its definition's, or the one the changed text that made it gave it (`CardDef::token_name_under`).
        name: copy_name.unwrap_or(def.name),
        types,
        supertypes: Supertypes { legendary, ..def.supertypes },
        // CR 612.4: a token's subtypes are what the text that made it named.
        subtypes,
        // CR 707.9b: a copy that says what it is gets that, not what the card it copies is (707.9d). A token's color
        // words are its text's (CR 612.4).
        colors: colors.unwrap_or_else(|| words.colors(def.colors())),
        power: pt.map(|(p, _)| p).or(def.power),
        toughness: pt.map(|(_, t)| t).or(def.toughness),
        keywords: Keywords(printed_keywords.0 | extra_keywords.0),
        // CR 612.4: a color word of a token's text is the text's that made it.
        protection: words.colors(def.protection),
        protection_colorless: false,
        protection_players: 0,
        protection_types: TypeSet::NONE,
        spell: false,
        protection_colored_spells: false,
        protection_everything: false,
        hexproof_from: worded_hexproof(def.hexproof_from, words),
        must_attack: 0,
        // Printed toxic N (CR 702.164a): the card's, or the one it copies.
        toxic: def.toxic,
        abilities,
        no_cost,
        name_set: false,
        lost_all: false,
        text_lost: false,
    };
    changeling(&mut chars);
    chars
}

/// Printed "hexproof from [quality]" with the color words of its text as `words` has them (CR 612.2).
fn worded_hexproof(printed: HexproofFrom, words: Words) -> HexproofFrom {
    if printed.colors().is_empty() || words.is_printed() {
        return printed;
    }
    printed.with_colors(words.colors(printed.colors()))
}

/// Changeling (CR 702.73a): "This object is every creature type", a characteristic-defining ability that works in every
/// zone (CR 604.3) and applies first in layer 4 (CR 613.3). It gives only what the object can have: creature types
/// belong to creatures and kindreds (CR 205.3d, 205.3m). Only the printed ability is a characteristic-defining one
/// (CR 604.3a), so one granted in layer 6 gives nothing. A non-creature, non-Kindred object with Changeling that later
/// becomes a creature is not given the types: no printed card can reach that case (Firdoch Core is a Kindred).
fn changeling(chars: &mut Chars) {
    if chars.keywords.has(Keyword::Changeling) && (chars.is(CardType::Creature) || chars.is(CardType::Kindred)) {
        chars.subtypes = chars.subtypes.union(SubtypeSet::CREATURE_TYPES);
    }
}

/// What a Room's unlocked designations leave of it (CR 709.5): "as long as this permanent doesn't have the 'left half
/// unlocked' designation, it doesn't have the name, mana cost, or rules text of this object's left half", and the same of
/// the right. The name is that of the unlocked doors (none, one, or both), unless a copy effect's exception says what it
/// is (CR 707.9b: "except its name is X"), which is no half's name for the rule to take away. Its colors are those of
/// the unlocked doors' mana costs unless a copy effect says otherwise (CR 707.9b), and its abilities the unlocked doors',
/// then any a copy effect added. The unlock special action (CR 709.5e) is no ability of it: the rules give it once
/// layer 6 is done (`unlock_actions`).
fn doors(cards: &CardTable, card: CardId, unlocked: u8, copy: Option<&Copiable>, chars: &mut Chars) {
    let def = cards.get(card);
    let (left, right) = def.split.expect("a Room has two doors");
    let (left_id, right_id) = cards.halves[card.0 as usize].expect("a Room's doors are in the card table");
    if copy.is_none_or(|c| c.name.is_none()) {
        chars.name = match unlocked & (DOOR_LEFT | DOOR_RIGHT) {
            0 => "",
            DOOR_LEFT => left.name,
            DOOR_RIGHT => right.name,
            _ => def.name,
        };
    }
    if copy.is_none_or(|c| c.colors.is_none()) {
        chars.colors = def.room_cost(unlocked).colors();
    }
    let mut abilities = [None; MAX_ABILITIES];
    let mut n = 0;
    let mut push = |ability: AbilityRef| {
        if n < MAX_ABILITIES {
            abilities[n] = Some(ability);
            n += 1;
        }
    };
    for (door, half, id) in [(DOOR_LEFT, left, left_id), (DOOR_RIGHT, right, right_id)] {
        if unlocked & door != 0 {
            for index in 0..half.abilities.len() {
                push(AbilityRef { card: id, index: index as u8 });
            }
        }
    }
    for ability in chars.abilities.into_iter().flatten() {
        push(ability);
    }
    chars.abilities = abilities;
}

/// Unlocking a Room's locked door is a special action the rules give its controller (CR 709.5e, 116.2m), not an ability
/// the permanent has, so an effect that takes its abilities (Humility over an Opalescence-animated Room) does not take
/// it. It is carried as `intrinsic::UNLOCK_LEFT`/`UNLOCK_RIGHT`, one per door still locked, given once layer 6 is done,
/// as turning a face-down permanent face up is (`turn_face_up_actions`).
fn unlock_actions(cards: &CardTable, unlocked: u8, chars: &mut Chars) {
    for (door, index) in [(DOOR_LEFT, cantrip_api::intrinsic::UNLOCK_LEFT), (DOOR_RIGHT, cantrip_api::intrinsic::UNLOCK_RIGHT)] {
        let action = AbilityRef { card: cards.intrinsic, index };
        if unlocked & door != 0 || chars.abilities.contains(&Some(action)) {
            continue;
        }
        if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
            chars.abilities[slot] = Some(action);
        }
    }
}

/// A card's characteristics with no copy effect and nothing acting on it, made once per card (`CardTable::printed`):
/// with `rules`, the abilities the rules give a land by its basic land types and a Siege (layer 6) too. `spell` is not
/// set: it is the object's, not the card's.
fn printed(cards: &CardTable, card: CardId, rules: bool) -> &Chars {
    cards.printed[card.0 as usize][rules as usize].get_or_init(|| {
        let def = cards.get(card);
        let mut chars = initial(cards, card, None);
        if rules {
            if chars.is(CardType::Land) {
                intrinsic_mana(cards, def, &mut chars);
            }
            if chars.is(CardType::Battle) {
                intrinsic_siege(cards, &mut chars);
            }
            intrinsic_keyword_abilities(cards, &mut chars);
        }
        Box::new(chars)
    })
}

/// The parts of effects in parts (`StaticDef::continues`, CR 613.6) that began to apply to an object are kept with it
/// (`Snap::parts`): each static ability's source and the part that applied. "If an effect starts to apply in one layer ...,
/// it will continue to be applied to the same set of objects in each other applicable layer": a later part applies to the
/// object exactly when its first part did, and so to the objects that part's words named then, though they no longer fit
/// those words ("all noncreature artifacts become 2/2 artifact creatures", CR 613.6's third example). Noted only while some
/// static-ability source has an effect in parts (`StaticSources::continuing`).
///
/// A static ability of `source` applied to `id`: its effect, or its effect on that object (`StaticDef::apply_each`).
fn apply_static(st: &StaticDef, view: &View, source: ObjectId, id: ObjectId, chars: &mut Chars) {
    match st.apply_each {
        Some(apply) => apply(view, source, id, chars),
        None => (st.apply)(view, source, chars),
    }
}

/// The first part of the effect `statics[i]` is a part of (CR 613.6): itself, unless it continues the one before it.
fn first_part(statics: &[StaticDef], mut i: usize) -> usize {
    while i > 0 && statics[i].continues {
        i -= 1;
    }
    i
}

/// One continuous effect of a layer, over the whole game.
#[derive(Clone, Copy)]
struct GEntry {
    kind: GKind,
    /// Its timestamp (CR 613.7), and which kind of entry it is at that timestamp: a permanent's static abilities, then
    /// those effects granted others, then resolved effects, then abilities granted an object itself (CR 613.7n: an object's
    /// own static ability is earlier than an effect created with it).
    timestamp: u32,
    seq: u8,
}

#[derive(Clone, Copy)]
enum GKind {
    /// A static ability of `source` (its own, or one an effect granted it) that applies to the objects it says; `first` is
    /// the first part of its effect (CR 613.6), itself unless it `continues` one, and then it applies to the objects that
    /// part applied to (`Snap::parts`), whatever its words say of them by now.
    Static { source: ObjectId, st: &'static StaticDef, first: *const StaticDef, continues: bool },
    /// A static ability `holder` was given that affects other objects (CR 613.1f, 613.7a). `ability` is the ability, where
    /// it was given in this very layer (layer 6): it applies only once `holder` has it.
    Given { holder: ObjectId, st: &'static StaticDef, ability: Option<AbilityRef>, by: Linked },
    /// A resolved spell's or ability's (CR 611.2), which names the object; a keyword counter's (CR 122.1b, 613.7c).
    Effect(ContinuousEffect),
    /// A static ability `holder` was granted that applies to it alone (CR 613.7a), with it as its source; `ability` as for
    /// `Given`.
    Granted { holder: ObjectId, st: &'static StaticDef, ability: Option<AbilityRef>, by: Linked },
}

impl GEntry {
    /// What it says, for a message.
    fn text(&self) -> &'static str {
        match self.kind {
            GKind::Static { st, .. } | GKind::Given { st, .. } | GKind::Granted { st, .. } => st.text,
            GKind::Effect(_) => "a resolved effect",
        }
    }

    /// The object whose effect it is, and the effect, for the abilities' givers (`ability_givers`): its timestamp (CR 613.7),
    /// and for a static ability, which of its source's static abilities it is, so two are two effects (CR 607.5).
    fn giver(&self, cards: &CardTable, state: &GameState) -> Giver {
        let grant = |part: usize| Acquired { by: AcquiredBy::Grant, stamp: self.timestamp, part: u8::try_from(part).unwrap_or(u8::MAX) };
        let part_of = |source: ObjectId, st: &'static StaticDef| {
            cards.get(cards.current(state.obj(source))).statics.iter().position(|s| std::ptr::eq(s, st)).unwrap_or(0)
        };
        match self.kind {
            GKind::Static { source, st, .. } => Giver { object: source, acquired: grant(part_of(source, st)) },
            GKind::Given { holder, st, .. } | GKind::Granted { holder, st, .. } => Giver { object: holder, acquired: grant(part_of(holder, st)) },
            GKind::Effect(e) => Giver { object: e.source, acquired: grant(0) },
        }
    }

    /// Whether another effect could change whether it applies to an object or what it does there (CR 613.8a(b)), which is
    /// what the dependency system judges: a static ability's reach and effect ("other creatures", "each land", "+1/+1 for
    /// each artifact"), and the resolved effects that do something only to an object that has something - a text change,
    /// only where its word is (CR 612.1); "all creature types" and a creature type set, only to a creature or Kindred (CR
    /// 205.3d). Every other resolved effect names the objects it applies to (CR 611.2c) and does the same to each whatever
    /// was done before it. Every static ability is judged - a later part of an effect too, which applies to the set its first
    /// part did (CR 613.6) but may do what other objects decide, and an ability granted an object, to it alone.
    fn dependent(&self) -> bool {
        match self.kind {
            // A later part applies to the set its first part did (CR 613.6), but what it does may still depend on others.
            GKind::Static { .. } | GKind::Given { .. } | GKind::Granted { .. } => true,
            GKind::Effect(e) => matches!(e.kind, EffectKind::ChangeText { .. } | EffectKind::AddAllCreatureTypes | EffectKind::SetCreatureType(_)),
        }
    }
}

/// The plan of layer `li`, made if there is none, once per pass.
fn plan(cx: &Ctx, li: usize) -> std::rc::Rc<Plan> {
    if let Some(plan) = cx.pass.plans.borrow()[li].clone() {
        return plan;
    }
    let bit = 1u16 << li;
    // Unreachable, an invariant of the reads the layers make: while layer `li` is being ordered, every read is of an earlier
    // layer (`chars_in` with a `stop` at or before it: a layer's start, collect_entries' holders through layer 6 only for
    // the layers after it, `hone` through layer 4) or of `li` itself at a stage (`View` with a stage, and
    // `View::chars_with_defined_pt`, which reads 7a at the stage until 7a is done). Nothing asks for the whole of a layer
    // that is being ordered.
    if cx.pass.building.get() & bit != 0 {
        unreachable!("the layers asked for layer {:?} while ordering it", LAYERS[li]);
    }
    if layers_with_entries(cx) & bit == 0 {
        let plan = std::rc::Rc::new(Plan { entries: Vec::new(), order: Vec::new() });
        cx.pass.plans.borrow_mut()[li] = Some(plan.clone());
        return plan;
    }
    cx.pass.building.set(cx.pass.building.get() | bit);
    let mut entries = collect_entries(cx, li);
    let order = order_entries(cx, li, &mut entries);
    cx.pass.building.set(cx.pass.building.get() & !bit);
    let plan = std::rc::Rc::new(Plan { entries, order });
    cx.pass.plans.borrow_mut()[li] = Some(plan.clone());
    plan
}

/// The layers some entry could be in, by index: those of the static abilities of every source, of the resolved effects, of
/// keyword counters (layer 6), and of granted static abilities (layers 7a on, where they are worked out). Found once a pass.
fn layers_with_entries(cx: &Ctx) -> u16 {
    if let Some(mask) = cx.pass.occupied.get() {
        return mask;
    }
    let (cards, state) = (cx.cards, cx.state);
    let bit = |layer: Layer| LAYERS.iter().position(|&l| l == layer).map_or(0, |i| 1u16 << i);
    let mut mask = 0u16;
    let mut granting = false;
    if state.statics_seen {
        let found = static_sources(cards, state);
        for &(source, _, _) in &found.list {
            for st in cards.get(cards.current(state.obj(source))).statics {
                mask |= bit(st.layer);
            }
        }
        granting = !found.granted.is_empty();
    }
    // Static abilities a static ability gives (CR 613.1f) apply from layer 6 on.
    granting |= state.statics_seen && static_grants_present(cards, state);
    if granting {
        mask |= bit(Layer::Ability);
    }
    for e in &state.effects {
        if let Some(layer) = effect_layer(e.kind) {
            mask |= bit(layer);
        }
        granting |= matches!(e.kind, EffectKind::AddAbility(_));
    }
    if granting {
        for layer in [Layer::PtDefine, Layer::PtSet, Layer::PtModify, Layer::PtSwitch] {
            mask |= bit(layer);
        }
    }
    if state.battlefield().iter().chain(state.zone(Zone::PHASED_OUT)).any(|&id| state.obj(id).has_counters()) {
        mask |= bit(Layer::Ability);
    }
    cx.pass.occupied.set(Some(mask));
    mask
}

/// Every continuous effect of layer `li` over the whole game, in timestamp order (CR 613.7): the static abilities of the
/// permanents (and cards that function elsewhere) that have them still, those effects granted permanents to give others,
/// resolved effects and keyword counters, and those effects granted objects themselves.
fn collect_entries(cx: &Ctx, li: usize) -> Vec<GEntry> {
    let layer = LAYERS[li];
    let (cards, state) = (cx.cards, cx.state);
    let mut entries = Vec::new();
    if state.statics_seen {
        let (list, granted_holders) = {
            let found = static_sources(cards, state);
            (found.list.clone(), found.granted.clone())
        };
        // Whether each still exists - its source has not lost it - is judged as the layer applies (`exists`).
        for (source, from, _) in list {
            let statics = cards.get(cards.current(state.obj(source))).statics;
            for (i, st) in statics.iter().enumerate() {
                if st.own || st.layer != layer || st.from != from {
                    continue;
                }
                let first = &statics[first_part(statics, i)];
                let kind = GKind::Static { source, st, first, continues: st.continues };
                entries.push(GEntry { kind, timestamp: state.obj(source).timestamp, seq: 0 });
            }
        }
        // The static abilities effects gave permanents that affect other objects, in their timestamp order (CR 613.7a).
        for holder in granted_holders {
            for (timestamp, holder, st, by) in granted_to_others(cx, holder, layer) {
                entries.push(GEntry { kind: GKind::Given { holder, st, ability: None, by }, timestamp, seq: 1 });
            }
        }
    }
    for e in state.effects.iter().filter(|e| effect_layer(e.kind) == Some(layer)) {
        entries.push(GEntry { kind: GKind::Effect(*e), timestamp: e.timestamp, seq: 2 });
    }
    // CR 122.1b, 613.1f, 613.7c: the ability a keyword counter gives applies in layer 6 at the timestamp of its counters, in
    // timestamp order with the effects there, so "loses all abilities" takes it from counters put on before it and not from
    // those put on after.
    if layer == Layer::Ability {
        for zone in [Zone::BATTLEFIELD, Zone::PHASED_OUT] {
            for &id in state.zone(zone) {
                let obj = state.obj(id);
                if obj.became.is_none() && obj.has_counters() {
                    let mut fx = Vec::new();
                    keyword_counters(state, id, obj, &mut fx);
                    for e in fx {
                        entries.push(GEntry { kind: GKind::Effect(e), timestamp: e.timestamp, seq: 2 });
                    }
                }
            }
        }
    }
    // The static abilities effects have granted objects that they still have once layer 6 is done, each with its timestamp
    // (CR 613.7a), applying to the object alone.
    if layer >= Layer::PtDefine {
        let mut holders: Vec<ObjectId> = Vec::new();
        for e in &state.effects {
            if let EffectKind::AddAbility(ability) = e.kind
                && matches!(cards.ability(ability), AbilityDef::Static(s) if s.own && s.layer == layer)
                && !holders.contains(&e.affected)
            {
                holders.push(e.affected);
            }
        }
        for holder in holders {
            let obj = state.obj(holder);
            let on_battlefield = (obj.zone.is(ZoneKind::Battlefield) || obj.zone.is(ZoneKind::PhasedOut)) && obj.became.is_none();
            if !(on_battlefield || obj.zone.is(ZoneKind::Stack)) || state.last_known(holder).is_some() {
                continue;
            }
            let fx: Vec<ContinuousEffect> = state.effects.iter().filter(|e| e.affected == holder).copied().collect();
            let chars = chars_in(cx, holder, Layer::PtDefine);
            for (timestamp, st, ability, giver) in granted_statics(cards, obj.timestamp, &fx, &chars) {
                if st.layer == layer {
                    let by = given_ability(cards, state, holder, ability, giver);
                    entries.push(GEntry { kind: GKind::Granted { holder, st, ability: None, by }, timestamp, seq: 3 });
                }
            }
        }
        // The static abilities a static ability gave a permanent (CR 613.1f), each at the later of the permanent's
        // timestamp and its giver's (CR 613.7a): found among the abilities each permanent has once layer 6 is done, where
        // some static ability could give abilities.
        if static_grants_present(cards, state) {
            for &holder in state.battlefield() {
                let chars = chars_in(cx, holder, Layer::PtDefine);
                let statics: Vec<(usize, AbilityRef, &'static StaticDef)> = chars
                    .abilities
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, a)| match a.map(|a| (a, cards.ability(a))) {
                        Some((a, AbilityDef::Static(st))) if st.layer == layer => Some((slot, a, st)),
                        _ => None,
                    })
                    .collect();
                if statics.is_empty() {
                    continue;
                }
                let by_effect =
                    |a: AbilityRef| state.effects.iter().any(|e| e.affected == holder && e.kind == EffectKind::AddAbility(a));
                let mut givers = None;
                for (slot, ability, st) in statics {
                    if by_effect(ability) {
                        continue;
                    }
                    let givers = *givers.get_or_insert_with(|| givers_in(cx, holder));
                    if givers[slot].object == ObjectId::NONE {
                        continue;
                    }
                    let timestamp = state.obj(holder).timestamp.max(state.obj(givers[slot].object).timestamp);
                    let by = given_ability(cards, state, holder, ability, givers[slot]);
                    let kind = if st.own { GKind::Granted { holder, st, ability: None, by } } else { GKind::Given { holder, st, ability: None, by } };
                    entries.push(GEntry { kind, timestamp, seq: if st.own { 3 } else { 1 } });
                }
            }
        }
    }
    entries.sort_by_key(|e| (e.timestamp, e.seq));
    entries
}

/// Whether `entry`, at `stage`, applies to the object as `snap` has it.
fn applies(view: &View, o: &ObjCtx, entry: &GEntry, snap: &Snap) -> bool {
    let id = o.id;
    match entry.kind {
        GKind::Static { source, st, first, continues } => {
            (o.on_battlefield || st.everywhere)
                && if continues {
                    snap.parts.iter().any(|&(s, part)| s == source && std::ptr::eq(part, first))
                } else {
                    exists(view, source, id, snap) && (st.affects)(view, source, id, &snap.chars)
                }
        }
        GKind::Given { holder, st, ability, by } => {
            o.on_battlefield
                && ability.is_none_or(|a| view.chars(holder).abilities.contains(&Some(a)))
                && (st.affects)(&View { asking: Some(by), ..*view }, holder, id, &snap.chars)
        }
        GKind::Effect(e) => {
            e.affected == id
                && (o.on_battlefield || o.spell)
                && match e.kind {
                    EffectKind::ChangeText { from: Word::Subtype(from), .. } => {
                        let chars = &snap.chars;
                        (chars.subtypes.has(from) && !(from.is_creature_type() && chars.keywords.has(Keyword::Changeling)))
                            || Keyword::landwalk(from).is_some_and(|walk| chars.keywords.has(walk))
                    }
                    EffectKind::ChangeText { from: Word::Color(from), .. } => {
                        snap.chars.hexproof_from.colors().has(from) || snap.chars.protection.has(from)
                    }
                    EffectKind::AddAllCreatureTypes | EffectKind::SetCreatureType(_) => {
                        snap.chars.is(CardType::Creature) || snap.chars.is(CardType::Kindred)
                    }
                    _ => true,
                }
        }
        GKind::Granted { holder, ability, .. } => holder == id && ability.is_none_or(|a| snap.chars.abilities.contains(&Some(a))),
    }
}

/// Whether a static ability of `source` that has not begun to apply (CR 613.6) exists at this stage: its source has not lost
/// it, to "loses all abilities" in layer 6 (CR 613.1f), or with the rest of its rules text to another's effect setting its
/// land subtypes in layer 4 (CR 305.7, including what a copy effect gave it). Each of those changes the existence of the
/// abilities it takes (CR 613.8a), so it is judged where the layer stands: one that applies first takes them, one that
/// applies after them does not undo what they did. Asked of the source as it stands at the stage (a read, for the
/// dependency system); free where nothing on the battlefield or among the effects takes abilities.
fn exists(view: &View, source: ObjectId, id: ObjectId, snap: &Snap) -> bool {
    if view.stop < Layer::Type || !strips(view) {
        return true;
    }
    let chars = if source == id { snap.chars } else { view.chars(source) };
    !(chars.text_lost || (view.stop >= Layer::Ability && chars.lost_all))
}

/// Whether anything in the game could take an object's abilities: a "loses all abilities" static ability or one that sets
/// land subtypes among the static-ability sources, or a resolved "loses all abilities".
fn strips(view: &View) -> bool {
    match view.pass {
        Some(pass) => {
            if let Some(found) = pass.strips.get() {
                return found;
            }
            let found = strippers_present(view.cards, view.state);
            pass.strips.set(Some(found));
            found
        }
        None => strippers_present(view.cards, view.state),
    }
}

fn strippers_present(cards: &CardTable, state: &GameState) -> bool {
    state.effects.iter().any(|e| e.kind == EffectKind::LoseAllAbilities) || (state.statics_seen && abilities_may_be_lost(cards, state))
}

/// What `entry` does to the object, which it applies to.
fn apply_raw(view: &View, layer: Layer, id: ObjectId, entry: &GEntry, chars: &mut Chars) {
    match entry.kind {
        GKind::Static { source, st, .. } => apply_static(st, view, source, id, chars),
        GKind::Given { holder, st, by, .. } => apply_static(st, &View { asking: Some(by), ..*view }, holder, id, chars),
        GKind::Effect(e) => apply_effect(&e, layer, chars),
        GKind::Granted { st, by, .. } => (st.apply)(&View { asking: Some(by), ..*view }, id, chars),
    }
}

/// Applies `entry` at `stage` to the object, if it applies to it, noting a static ability that began to apply, and its
/// giver while the object's abilities' givers are being worked out (`watch`).
fn apply_entry(cx: &Ctx, o: &ObjCtx, entry: &GEntry, stage: &Stage, snap: &mut Snap, noting: bool, watch: bool) {
    let view = View { stage: Some(*stage), pass: Some(cx.pass), ..View::new(cx.cards, cx.state, cx.log, LAYERS[stage.li]) };
    if !applies(&view, o, entry, snap) {
        return;
    }
    let layer = LAYERS[stage.li];
    let text_lost = snap.chars.text_lost;
    if watch {
        watching_givers(cx.cards, o.id, entry.giver(cx.cards, cx.state), &mut snap.chars, |chars| apply_raw(&view, layer, o.id, entry, chars));
    } else {
        apply_raw(&view, layer, o.id, entry, &mut snap.chars);
    }
    // A land whose own ability sets its own land subtypes ("this land is the chosen type": Multiversal Passage) keeps that
    // ability applying, and loses no rules text to it: it is another's effect that takes a land's text (CR 305.7).
    if let GKind::Static { source, st, .. } = entry.kind
        && st.sets_land_subtypes
        && source == o.id
    {
        snap.chars.text_lost = text_lost;
    }
    if noting && let GKind::Static { source, st, continues: false, .. } = entry.kind {
        snap.parts.push((source, st));
    }
}

/// The objects an entry could apply to: a static ability's, every permanent (a phased-out one is treated as though it
/// doesn't exist, CR 702.26b), and for one that functions everywhere, every object in a zone of the game; a resolved
/// effect's, the object it names; a granted ability's, its holder.
fn candidates(cx: &Ctx, entry: &GEntry) -> Vec<ObjectId> {
    let state = cx.state;
    match entry.kind {
        GKind::Static { st, .. } if st.everywhere => state
            .zones
            .iter()
            .flatten()
            .copied()
            .filter(|&id| {
                let obj = state.obj(id);
                !obj.zone.is(ZoneKind::Sideboard) && !obj.zone.is(ZoneKind::PhasedOut) && obj.became.is_none()
            })
            .collect(),
        GKind::Static { .. } | GKind::Given { .. } => state.battlefield().to_vec(),
        GKind::Effect(e) => vec![e.affected],
        GKind::Granted { holder, .. } => vec![holder],
    }
}

/// The order layer `li` applies `entries` in (CR 613.8): an entry depends on another if applying the other would change
/// whether it applies to an object, or what it does to one it applies to (613.8a(b)), judged on the whole game as the
/// entries applied so far leave it (613.8c); it waits until everything it depends on has applied (613.8b). A dependency
/// loop is broken by timestamp order among its members, and every other dependency is kept (`docs/rules-exceptions.md`,
/// RX-3): an entry is ready once every entry it depends on that has not applied is in a loop with it, and the earliest
/// ready entry applies next. With nothing that could depend on anything, the order is the timestamp order.
///
/// Within layer 6 an entry can come into being: a static ability one entry gives an object (Rune of Flight's "equipped
/// creature has flying", given to an Equipment) exists once that entry has applied (CR 613.1f, 613.7a), and joins the
/// entries still to apply at the later of its holder's timestamp and its giver's.
fn order_entries(cx: &Ctx, li: usize, entries: &mut Vec<GEntry>) -> Vec<u32> {
    let n = entries.len();
    let grants = LAYERS[li] == Layer::Ability && grants_in_layer_six(cx);
    if !grants && (n < 2 || !entries.iter().any(GEntry::dependent)) {
        return (0..n as u32).collect();
    }
    let mut remaining: Vec<u32> = (0..n as u32).collect();
    let mut order: Vec<u32> = Vec::with_capacity(n);
    while !remaining.is_empty() {
        // The judge follows the layer step by step, asking again only of the objects each step changes (613.8c); made anew
        // when entries come into being (`appear`).
        let mut judge = Judge::new(cx, li, &entries[..], std::mem::take(&mut order));
        // What depends on what among `remaining`, once judged: kept while the entries applied since changed nothing.
        let mut judged: Option<Vec<Vec<bool>>> = None;
        let mut given = None;
        while !remaining.is_empty() {
            let m = remaining.len();
            // The earliest entry is ready if nothing could make it wait.
            // The earliest entry is ready if nothing could make it wait, or if it waits for nothing.
            let earliest_free = || (1..m).all(|b| !judge.depends(remaining[0], remaining[b]));
            let next = if !judge.entries[remaining[0] as usize].dependent() || (judged.is_none() && earliest_free()) {
                0
            } else {
                let direct = match judged.take() {
                    Some(direct) => direct,
                    None => {
                        let mut direct = vec![vec![false; m]; m];
                        for a in 0..m {
                            if !judge.entries[remaining[a] as usize].dependent() {
                                continue;
                            }
                            for b in 0..m {
                                if a != b {
                                    direct[a][b] = judge.depends(remaining[a], remaining[b]);
                                }
                            }
                        }
                        direct
                    }
                };
                // Which each depends on through others: a loop is a set of entries that each depend on the others this way.
                let mut through = direct.clone();
                for k in 0..m {
                    let via = through[k].clone();
                    for row in through.iter_mut() {
                        if row[k] {
                            for (cell, &on) in row.iter_mut().zip(&via) {
                                *cell |= on;
                            }
                        }
                    }
                }
                let next = (0..m).find(|&a| (0..m).all(|b| !direct[a][b] || through[b][a])).unwrap_or(0);
                judged = Some(direct);
                next
            };
            let chosen = remaining.remove(next);
            // CR 613.8c: worked out again after each, unless applying it changed nothing.
            let changed = judge.advance(chosen);
            judged = match judged.take() {
                Some(mut direct) if !changed => {
                    direct.remove(next);
                    for row in direct.iter_mut() {
                        row.remove(next);
                    }
                    Some(direct)
                }
                _ => None,
            };
            if grants && changed {
                given = Some(chosen);
                break;
            }
        }
        order = judge.order;
        if let Some(e) = given {
            appear(cx, li, entries, &order[..order.len() - 1], e, &mut remaining);
        }
    }
    order
}
/// Whether a static ability could give an object a static ability (CR 613.1f): a source on the battlefield whose card has a
/// static ability to give (Rune of Flight's "equipped creature has flying", in its `delayed` list).
fn static_grants_present(cards: &CardTable, state: &GameState) -> bool {
    state.statics_seen
        && static_sources(cards, state).list.iter().any(|&(source, _, _)| {
            let card = cards.get(cards.current(state.obj(source)));
            card.statics.iter().any(|s| !s.own && s.layer == Layer::Ability) && card.delayed.iter().any(|a| matches!(a, AbilityDef::Static(_)))
        })
}

/// Whether a static ability could be given in layer 6 an object that applies in layer 6: an effect gives one, or a static
/// ability that gives abilities functions.
fn grants_in_layer_six(cx: &Ctx) -> bool {
    let (cards, state) = (cx.cards, cx.state);
    state.effects.iter().any(|e| matches!(e.kind, EffectKind::AddAbility(a) if matches!(cards.ability(a), AbilityDef::Static(st) if st.layer == Layer::Ability)))
        || static_grants_present(cards, state)
}

/// Entry `e` applies after `order`: the static abilities of layer 6 it gave objects come into being (CR 613.1f), each an
/// entry at the later of its holder's timestamp and `e`'s (CR 613.7a), put among `remaining` in timestamp order. Whether
/// any did.
fn appear(cx: &Ctx, li: usize, entries: &mut Vec<GEntry>, order: &[u32], e: u32, remaining: &mut Vec<u32>) -> bool {
    let found: Vec<GEntry> = {
        let judge = Judge::new(cx, li, entries, order.to_vec());
        let stage = judge.stage();
        let after = Stage { extra: Some(e), ..stage };
        let mut found = Vec::new();
        for &holder in judge.reach(e).iter() {
            let Object::Full(o) = object(cx.cards, cx.state, holder) else { continue };
            let snap = at(cx, &o, &after, false);
            for ability in snap.chars.abilities.into_iter().flatten() {
                let AbilityDef::Static(st) = cx.cards.ability(ability) else { continue };
                let known = |g: &GEntry| match g.kind {
                    GKind::Given { holder: h, ability: a, .. } | GKind::Granted { holder: h, ability: a, .. } => h == holder && a == Some(ability),
                    _ => false,
                };
                if st.layer != Layer::Ability || entries.iter().any(known) || found.iter().any(known) {
                    continue;
                }
                let timestamp = cx.state.obj(holder).timestamp.max(entries[e as usize].timestamp);
                let by = given_ability(cx.cards, cx.state, holder, ability, entries[e as usize].giver(cx.cards, cx.state));
                let (kind, seq) = if st.own {
                    (GKind::Granted { holder, st, ability: Some(ability), by }, 3)
                } else {
                    (GKind::Given { holder, st, ability: Some(ability), by }, 1)
                };
                found.push(GEntry { kind, timestamp, seq });
            }
        }
        found
    };
    let any = !found.is_empty();
    for entry in found {
        let index = entries.len() as u32;
        entries.push(entry);
        let at = remaining.partition_point(|&r| {
            let g = &entries[r as usize];
            (g.timestamp, g.seq) <= (entry.timestamp, entry.seq)
        });
        remaining.insert(at, index);
    }
    any
}

/// What an entry does at a stage to one object it could apply to: whether it applies, what it makes of the object as it
/// is, and which other objects the question read (sorted).
#[derive(Clone)]
struct Eval {
    id: ObjectId,
    here: std::rc::Rc<Snap>,
    applies: bool,
    did: Chars,
    reads: Vec<ObjectId>,
}

/// An entry asked of every object it could apply to, by object, with the objects whose question read each other object.
struct EvalSet {
    /// Sorted by object.
    list: Vec<Eval>,
    readers: std::collections::HashMap<ObjectId, Vec<u32>, Quick>,
}

impl EvalSet {
    fn new(mut list: Vec<Eval>) -> EvalSet {
        list.sort_unstable_by_key(|v| v.id);
        let mut readers: std::collections::HashMap<ObjectId, Vec<u32>, Quick> = Default::default();
        for (i, v) in list.iter().enumerate() {
            for &r in v.reads.iter().filter(|&&r| r != v.id) {
                readers.entry(r).or_default().push(i as u32);
            }
        }
        EvalSet { list, readers }
    }

    fn index(&self, id: ObjectId) -> Option<usize> {
        self.list.binary_search_by_key(&id, |v| v.id).ok()
    }
}

/// The dependency system's questions as a layer is ordered (CR 613.8a): what each entry applies to at the stage reached
/// (`order`, the entries applied so far), and whether applying one would change whether another applies to an object or
/// what it does there. What it has asked is kept from step to step, and asked again only of the objects a step changed and
/// of those whose question read one of them (613.8c).
struct Judge<'a> {
    cx: &'a Ctx<'a>,
    li: usize,
    entries: &'a [GEntry],
    order: Vec<u32>,
    reaches: std::cell::RefCell<std::collections::HashMap<u32, std::rc::Rc<Vec<ObjectId>>>>,
    evals: std::cell::RefCell<std::collections::HashMap<u32, (usize, std::rc::Rc<EvalSet>)>>,
    /// The objects each step changed, in order: what an entry asked at an earlier step must be asked again of.
    steps: Vec<Vec<ObjectId>>,
}

impl<'a> Judge<'a> {
    fn new(cx: &'a Ctx<'a>, li: usize, entries: &'a [GEntry], order: Vec<u32>) -> Judge<'a> {
        Judge { cx, li, entries, order, reaches: Default::default(), evals: Default::default(), steps: Vec::new() }
    }

    fn stage(&self) -> Stage<'_> {
        Stage { li: self.li, entries: self.entries, order: &self.order, len: self.order.len(), extra: None, keep: true }
    }

    fn view<'v>(&'v self, stage: Stage<'v>, reads: Option<&'v std::cell::RefCell<Vec<ObjectId>>>) -> View<'v> {
        View { stage: Some(stage), reads, pass: Some(self.cx.pass), ..View::new(self.cx.cards, self.cx.state, self.cx.log, LAYERS[self.li]) }
    }

    /// Entry `entry` asked of the object `o` at this stage.
    fn eval_one(&self, entry: &GEntry, o: &ObjCtx) -> Eval {
        let stage = self.stage();
        let reads = std::cell::RefCell::new(Vec::new());
        let here = at(self.cx, o, &stage, false);
        let (applies, did) = {
            let view = self.view(stage, Some(&reads));
            let applies = applies(&view, o, entry, &here);
            let mut did = here.chars;
            if applies {
                apply_raw(&view, LAYERS[self.li], o.id, entry, &mut did);
            }
            (applies, did)
        };
        let mut reads = reads.into_inner();
        reads.sort_unstable();
        reads.dedup();
        Eval { id: o.id, here, applies, did, reads }
    }

    /// Entry `e` asked of every object it could apply to at this stage.
    fn eval(&self, e: u32) -> std::rc::Rc<EvalSet> {
        let entry = &self.entries[e as usize];
        let kept = self.evals.borrow().get(&e).cloned();
        if let Some((at_step, found)) = kept {
            if at_step == self.steps.len() {
                return found;
            }
            // Asked again of the objects changed since, and of those whose question read one of them.
            let mut touched: Vec<ObjectId> = self.steps[at_step..].iter().flatten().copied().collect();
            touched.sort_unstable();
            touched.dedup();
            let again: Vec<Eval> = found
                .list
                .iter()
                .map(|v| {
                    let stale = touched.binary_search(&v.id).is_ok() || v.reads.iter().any(|r| touched.binary_search(r).is_ok());
                    match (stale, object(self.cx.cards, self.cx.state, v.id)) {
                        (true, Object::Full(o)) => self.eval_one(entry, &o),
                        _ => v.clone(),
                    }
                })
                .collect();
            let found = std::rc::Rc::new(EvalSet::new(again));
            self.evals.borrow_mut().insert(e, (self.steps.len(), found.clone()));
            return found;
        }
        let found: Vec<Eval> = candidates(self.cx, entry)
            .into_iter()
            .filter_map(|id| match object(self.cx.cards, self.cx.state, id) {
                Object::Full(o) => Some(self.eval_one(entry, &o)),
                _ => None,
            })
            .collect();
        let found = std::rc::Rc::new(EvalSet::new(found));
        self.evals.borrow_mut().insert(e, (self.steps.len(), found.clone()));
        found
    }

    /// The objects entry `e` applies to at this stage, sorted: asked of each object whether it applies, no more.
    fn reach(&self, e: u32) -> std::rc::Rc<Vec<ObjectId>> {
        if let Some(found) = self.reaches.borrow().get(&e) {
            return found.clone();
        }
        let entry = &self.entries[e as usize];
        let stage = self.stage();
        let view = self.view(stage, None);
        let mut found: Vec<ObjectId> = candidates(self.cx, entry)
            .into_iter()
            .filter(|&id| match object(self.cx.cards, self.cx.state, id) {
                Object::Full(o) => applies(&view, &o, entry, &at(self.cx, &o, &stage, false)),
                _ => false,
            })
            .collect();
        found.sort_unstable();
        let found = std::rc::Rc::new(found);
        self.reaches.borrow_mut().insert(e, found.clone());
        found
    }

    /// Entry `e` applies: the stage moves on. What was asked is asked again of the objects `e` changed and of those whose
    /// question read one of them; the rest stands. Whether `e` changed anything.
    fn advance(&mut self, e: u32) -> bool {
        let touched: Vec<ObjectId> = {
            let stage = self.stage();
            let after = Stage { extra: Some(e), ..stage };
            self.reach(e)
                .iter()
                .copied()
                .filter(|&id| match object(self.cx.cards, self.cx.state, id) {
                    Object::Full(o) => {
                        let (was, is) = (at(self.cx, &o, &stage, false), at(self.cx, &o, &after, false));
                        was.chars != is.chars || was.parts != is.parts
                    }
                    _ => false,
                })
                .collect()
        };
        self.order.push(e);
        // An entry applied is asked nothing more; the rest are asked again when next asked (`eval`).
        self.evals.borrow_mut().remove(&e);
        let changed = !touched.is_empty();
        if changed {
            self.reaches.borrow_mut().clear();
            let mut touched = touched;
            touched.sort_unstable();
            self.steps.push(touched);
        }
        changed
    }

    /// Whether entry `a` depends on entry `b` (CR 613.8a): applying `b` would change, for some object, whether `a` applies
    /// to it, or - judged on what `a` would do to the object as it is, so that two effects that each set the same thing do
    /// not depend on each other through it (CR 613.9's flying and "loses flying") - what `a` does there. Only the objects
    /// `b` applies to can be changed by it: `a` is asked of those, and of any object whose question read one of them.
    fn depends(&self, a: u32, b: u32) -> bool {
        let changed = self.reach(b);
        if changed.is_empty() {
            return false;
        }
        let entry = &self.entries[a as usize];
        let stage = self.stage();
        let other = Stage { extra: Some(b), ..stage };
        let layer = LAYERS[self.li];
        let set = self.eval(a);
        // The objects `b` applies to, and those whose question read one of them.
        let mut asked: Vec<usize> = changed.iter().filter_map(|&id| set.index(id)).collect();
        for id in changed.iter() {
            if let Some(readers) = set.readers.get(id) {
                asked.extend(readers.iter().map(|&i| i as usize));
            }
        }
        asked.sort_unstable();
        asked.dedup();
        for v in asked.into_iter().map(|i| &set.list[i]) {
            let on_it = changed.binary_search(&v.id).is_ok();
            let read_changed = v.reads.iter().any(|r| *r != v.id && changed.binary_search(r).is_ok());
            if !on_it && !read_changed {
                continue;
            }
            let Object::Full(o) = object(self.cx.cards, self.cx.state, v.id) else { continue };
            let there = if on_it { at(self.cx, &o, &other, false) } else { v.here.clone() };
            // `b` changes nothing of this object, and nothing else the question read.
            if there.chars == v.here.chars && !read_changed {
                continue;
            }
            let view_b = self.view(other, None);
            let is = applies(&view_b, &o, entry, &there);
            if v.applies != is {
                return true;
            }
            if is {
                let mut does = v.here.chars;
                apply_raw(&view_b, layer, v.id, entry, &mut does);
                // The contract of a static ability's `apply` (`StaticApply`): it reads no value from the object it is given
                // except to guard, so what it does is judged on the object as it stands. Checked on a scratch copy, never
                // kept: applied to the object as `b` left it too, each field must come out the same, or change the same
                // way, or be untouched.
                if verifying() && on_it && there.chars != v.here.chars {
                    let mut after = there.chars;
                    apply_raw(&view_b, layer, v.id, entry, &mut after);
                    if let Some(field) = takes_from_its_object(&v.here.chars, &does, &there.chars, &after, false) {
                        panic!(
                            "an effect of layer {:?} ({}) read its object's {field} through its `chars` and not `view.chars(id)`",
                            LAYERS[self.li],
                            entry.text()
                        );
                    }
                }
                if does != v.did {
                    return true;
                }
            }
        }
        false
    }
}

/// One field of `Chars` as the contract check compares it.
#[derive(PartialEq, Clone)]
enum Field {
    /// A set of bits: the same change is the same bits flipped.
    Bits(Vec<u64>),
    /// A number: the same change is the same difference.
    Number(Option<i16>),
    /// The abilities: the same change is the same ones added and removed, as multisets (CR 113.2c: an ability given twice
    /// is two instances).
    Abilities(Vec<AbilityRef>),
    /// Anything else, which changes only by being set.
    Plain(u64),
}

fn fields(c: &Chars) -> [(&'static str, Field); 19] {
    use std::hash::{Hash, Hasher};
    let plain = |v: &dyn Fn(&mut std::collections::hash_map::DefaultHasher)| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        v(&mut h);
        Field::Plain(h.finish())
    };
    let bits = |v: u64| Field::Bits(vec![v]);
    [
        ("types", bits(c.types.0.into())),
        ("supertypes", bits(u64::from(c.supertypes.basic) | u64::from(c.supertypes.legendary) << 1 | u64::from(c.supertypes.snow) << 2)),
        ("subtypes", Field::Bits(c.subtypes.0.to_vec())),
        ("colors", bits(c.colors.0.into())),
        ("power", Field::Number(c.power)),
        ("toughness", Field::Number(c.toughness)),
        ("keywords", bits(c.keywords.0)),
        ("protection", bits(c.protection.0.into())),
        ("protection from types", bits(c.protection_types.0.into())),
        ("protection from players", bits(c.protection_players.into())),
        ("hexproof from", bits(c.hexproof_from.0.into())),
        ("abilities", Field::Abilities(c.abilities.iter().flatten().copied().collect())),
        ("name", plain(&|h| c.name.hash(h))),
        (
            "protections",
            bits(
                u64::from(c.protection_colorless)
                    | u64::from(c.protection_colored_spells) << 1
                    | u64::from(c.protection_everything) << 2,
            ),
        ),
        // A count: the same change is the same instances added (an audit's field; past i16 it reads as the most).
        ("must_attack", Field::Number(Some(i16::try_from(c.must_attack).unwrap_or(i16::MAX)))),
        // The total toxic value (CR 702.164b), a sum of instances as `must_attack` is.
        ("toxic", Field::Number(Some(i16::try_from(c.toxic).unwrap_or(i16::MAX)))),
        ("losses", bits(u64::from(c.lost_all) | u64::from(c.text_lost) << 1)),
        ("marks", bits(u64::from(c.spell) | u64::from(c.no_cost) << 1 | u64::from(c.name_set) << 2)),
        ("card", plain(&|h| c.card.hash(h))),
    ]
}

/// The change from `from` to `to` of one field, where a change has a shape (`Field`).
fn change(from: &Field, to: &Field) -> Option<Field> {
    match (from, to) {
        (Field::Bits(a), Field::Bits(b)) => Some(Field::Bits(a.iter().zip(b).map(|(a, b)| a ^ b).collect())),
        (Field::Number(Some(a)), Field::Number(Some(b))) => Some(Field::Number(Some((*b).saturating_sub(*a)))),
        (Field::Abilities(a), Field::Abilities(b)) => {
            let mut added = b.clone();
            let mut removed = Vec::new();
            for x in a {
                match added.iter().position(|y| y == x) {
                    Some(at) => {
                        added.remove(at);
                    }
                    None => removed.push(*x),
                }
            }
            added.sort_by_key(|a| (a.card.0, a.index));
            removed.sort_by_key(|a| (a.card.0, a.index));
            added.push(AbilityRef { card: CardId(u16::MAX), index: 0 });
            added.extend(removed);
            Some(Field::Abilities(added))
        }
        _ => None,
    }
}

/// Whether an effect applied to `x` (making `ax`) and to `y` (making `ay`) did, field by field, what one instruction does to
/// any object: the same result, or the same change, or - in a field `x` and `y` agree in, where only a guard on another
/// field can have told them apart - nothing on one side. Nothing on one side in a field they differ in does not excuse a
/// value worked out from it ("+X/+0, where X is its power" does nothing to a 0-power object). Where `x` and `y` differ in
/// more than one field (`strict` off: two objects as the game made them), nothing on one side is excused as well if they
/// differ in another field, which a guard may have read. The field that breaks it, if one does: the effect took a value
/// from the object it was applied to.
fn takes_from_its_object(x: &Chars, ax: &Chars, y: &Chars, ay: &Chars, strict: bool) -> Option<&'static str> {
    let (fx, fax, fy, fay) = (fields(x), fields(ax), fields(y), fields(ay));
    for i in 0..fx.len() {
        let (name, x_i) = &fx[i];
        let (ax_i, y_i, ay_i) = (&fax[i].1, &fy[i].1, &fay[i].1);
        if ax_i == ay_i {
            continue;
        }
        // A set of bits is judged bit by bit: each bit is a thing of its own an instruction sets, clears or leaves.
        if let (Field::Bits(x_b), Field::Bits(ax_b), Field::Bits(y_b), Field::Bits(ay_b)) = (x_i, ax_i, y_i, ay_i) {
            let others = !strict && (0..fx.len()).any(|j| j != i && fx[j].1 != fy[j].1);
            let fine = (0..x_b.len()).all(|w| {
                (0..64).all(|k| {
                    let bit = |v: &Vec<u64>| (v[w] >> k) & 1;
                    let (x, ax, y, ay) = (bit(x_b), bit(ax_b), bit(y_b), bit(ay_b));
                    ax == ay || (x ^ ax) == (y ^ ay) || ((ax == x || ay == y) && (x == y || others))
                })
            });
            if fine {
                continue;
            }
            return Some(name);
        }
        if let (Some(dx), Some(dy)) = (change(x_i, ax_i), change(y_i, ay_i))
            && dx == dy
        {
            continue;
        }
        // Nothing on both sides: the effect does not touch the field.
        if ax_i == x_i && ay_i == y_i {
            continue;
        }
        let nothing = ax_i == x_i || ay_i == y_i;
        // A power or toughness an object does not have is not changed (CR 208.3): nothing there is a guard.
        let undefined = matches!(x_i, Field::Number(None)) || matches!(y_i, Field::Number(None));
        let guarded = x_i == y_i || undefined || (!strict && (0..fx.len()).any(|j| j != i && fx[j].1 != fy[j].1));
        if nothing && guarded {
            continue;
        }
        return Some(name);
    }
    None
}

/// The objects the contract of `apply` (`StaticApply`) is checked on: `base` against copies differing from it in one field
/// each, every field `fields` lists in turn, power and toughness by two amounts.
fn contract_pairs(base: Chars, ability: AbilityRef) -> Vec<(Chars, Chars)> {
    let mut pairs = Vec::new();
    let mut vary = |f: &dyn Fn(&mut Chars)| {
        let mut y = base;
        f(&mut y);
        if y != base {
            pairs.push((base, y));
        }
    };
    // types
    vary(&|c| c.types = c.types.with(CardType::Artifact));
    vary(&|c| c.types = c.types.with(CardType::Creature));
    vary(&|c| c.types = c.types.with(CardType::Land));
    vary(&|c| c.types = TypeSet(c.types.0 & !TypeSet::of(&[CardType::Creature]).0));
    vary(&|c| c.types = TypeSet(c.types.0 & !TypeSet::of(&[CardType::Land]).0));
    // supertypes
    vary(&|c| c.supertypes.basic = !c.supertypes.basic);
    vary(&|c| c.supertypes.legendary = !c.supertypes.legendary);
    vary(&|c| c.supertypes.snow = !c.supertypes.snow);
    // subtypes
    vary(&|c| c.subtypes = c.subtypes.with(Subtype::Goblin));
    vary(&|c| c.subtypes = c.subtypes.with(Subtype::Swamp));
    vary(&|c| c.subtypes = c.subtypes.with(Subtype::Equipment));
    vary(&|c| c.subtypes = SubtypeSet::NONE);
    // colors
    vary(&|c| c.colors = ColorSet::of(Color::Black));
    vary(&|c| c.colors = ColorSet::NONE);
    // power and toughness, by two amounts
    for amount in [1, 10] {
        vary(&|c| c.power = Some(c.power.unwrap_or(0) + amount));
        vary(&|c| c.toughness = Some(c.toughness.unwrap_or(0) + amount));
    }
    // keywords
    vary(&|c| c.keywords = c.keywords.with(Keyword::Flying));
    vary(&|c| c.keywords = Keywords::NONE);
    vary(&|c| c.keywords = Keywords(u64::MAX));
    // protection
    vary(&|c| c.protection = ColorSet::of(Color::Red));
    vary(&|c| c.protection_types = TypeSet::of(&[CardType::Creature]));
    vary(&|c| c.protection_players = 0b10);
    vary(&|c| c.hexproof_from = HexproofFrom(1));
    // abilities
    vary(&|c| {
        if let Some(slot) = c.abilities.iter().position(|a| a.is_none()) {
            c.abilities[slot] = Some(ability);
        }
    });
    vary(&|c| c.abilities = [None; MAX_ABILITIES]);
    // name
    vary(&|c| c.name = "Another Name");
    // the other protections and attack requirement
    vary(&|c| c.protection_colorless = !c.protection_colorless);
    vary(&|c| c.protection_colored_spells = !c.protection_colored_spells);
    vary(&|c| c.protection_everything = !c.protection_everything);
    vary(&|c| c.must_attack += 1);
    vary(&|c| c.toxic += 1);
    // losses
    vary(&|c| c.lost_all = !c.lost_all);
    vary(&|c| c.text_lost = !c.text_lost);
    // the rest
    vary(&|c| c.spell = !c.spell);
    vary(&|c| c.no_cost = !c.no_cost);
    vary(&|c| c.name_set = !c.name_set);
    pairs
}

/// The raw state a static ability's effect may check before it does anything, varied: the source attached to the object or
/// not, a counter of every kind on both, the source at its highest class level and with choices made (a creature type, a
/// color, a player), the object under the other player's control, and life totals high; a crowded game (another of the
/// source's card, an artifact, a black and a white permanent, an Equipment attached to the source, creature cards in the
/// graveyards and cards in hand); and the source in a hand.
const SCENES: usize = 7;

fn contract_scene(cards: &CardTable, card: CardId, target: CardId, scene: usize) -> (GameState, ObjectId, ObjectId) {
    let mut state = GameState::new(2, 0);
    let mut source = Obj::new(card, PlayerId(0), Zone::BATTLEFIELD);
    source.timestamp = 1;
    let mut object = Obj::new(target, PlayerId(if scene == 3 { 1 } else { 0 }), Zone::BATTLEFIELD);
    object.timestamp = 2;
    if scene == 4 {
        source.class_level = 3;
        // Made by the source's own text, as an entering permanent's own `enters` makes them.
        let made = cantrip_state::Entered { subtype: Some(Subtype::Goblin), color: Some(Color::Black), player: Some(PlayerId(1)), ..Default::default() };
        source.marks.choices = <cantrip_state::Choices as cantrip_state::ChoiceRecord>::entered(cards.linked_card(card), made);
    }
    let source = state.push_obj(source);
    state.zone_push(Zone::BATTLEFIELD, source);
    let object = state.push_obj(object);
    state.zone_push(Zone::BATTLEFIELD, object);
    // On the objects once they exist: a new object has none (`GameState::push_obj`, CR 400.7).
    if scene == 2 {
        let every = [1; CounterKind::COUNT];
        for id in [source, object] {
            state.update_obj(id, |o| crate::counters::hypothetical(o, &every));
        }
    }
    if scene != 1 {
        state.update_obj(source, |o| o.attached_to = Some(object));
    }
    if scene == 4 {
        for p in [PlayerId(0), PlayerId(1)] {
            state.update_player(p, |pl| pl.life = 40);
        }
    }
    if scene == 5 {
        let mut put = |name_or: Option<CardId>, zone: Zone, attached: Option<ObjectId>| {
            if let Some(c) = name_or {
                let mut o = Obj::new(c, PlayerId(0), zone);
                o.timestamp = 3;
                o.attached_to = attached;
                let id = state.push_obj(o);
                state.zone_push(zone, id);
            }
        };
        put(Some(card), Zone::BATTLEFIELD, None);
        for name in ["Sol Ring", "Bloodghast", "White Knight"] {
            put(cards.find(name), Zone::BATTLEFIELD, None);
        }
        put(cards.find("Colossus Hammer"), Zone::BATTLEFIELD, Some(source));
        // A creature card with an activated ability, whatever the object is: what an ability that reads the graveyards' cards
        // (Necrotic Ooze's "all activated abilities of all creature cards in all graveyards") has to find there.
        let graveyard_card = cards.find("Llanowar Elves").unwrap_or(target);
        for p in [PlayerId(0), PlayerId(1)] {
            for _ in 0..8 {
                put(Some(graveyard_card), Zone::graveyard(p), None);
            }
            for _ in 0..3 {
                put(Some(target), Zone::hand(p), None);
            }
        }
    }
    if scene == 6 {
        state.zone_remove(Zone::BATTLEFIELD, source);
        state.update_obj(source, |o| o.zone = Zone::hand(PlayerId(0)));
        state.zone_push(Zone::hand(PlayerId(0)), source);
    }
    state.see_statics();
    (state, source, object)
}

/// What the contract check of every static ability in a card table found (`apply_contract_check`).
pub struct ApplyContract {
    /// One line for each static ability whose effect takes a value from the object it is given.
    pub breaches: Vec<String>,
    /// One line for each that changed nothing on any object, raw state or pair it was tried on: its contract went untested.
    pub inert: Vec<String>,
}

/// The contract of every static ability's `apply` in `cards` (`StaticApply`), checked on synthetic objects (`contract_pairs`):
/// the static abilities cards print and those they give, each applied with its source on the battlefield, in each raw state
/// of `contract_scene`, to a creature, a land and an artifact and to copies differing from each in one field.
pub fn apply_contract_check(cards: &'static CardTable) -> ApplyContract {
    let mut breaches = Vec::new();
    let mut inert = Vec::new();
    let objects: Vec<CardId> =
        ["Grizzly Bears", "Llanowar Elves", "Taiga", "Forest", "Sol Ring", "Mox Ruby"].iter().filter_map(|name| cards.find(name)).collect();
    let bases: Vec<Chars> = objects.iter().map(|&card| *printed(cards, card, true)).collect();
    let ability = AbilityRef { card: cards.intrinsic, index: Mana::W as u8 };
    for (index, def) in cards.cards.iter().enumerate() {
        let card = CardId(index as u16);
        let statics: Vec<&'static StaticDef> = def
            .statics
            .iter()
            .chain(def.abilities.iter().chain(def.delayed).filter_map(|a| match a {
                AbilityDef::Static(st) => Some(st),
                _ => None,
            }))
            .collect();
        for st in statics {
            // A control-changing ability's reach reads control through the view (`LayerQuery::controller`), which in layer 2
            // is the controller the effects applied so far leave: asked with the stored controllers swapped and the view's
            // held, its answer must not move.
            if st.layer == Layer::Control {
                for scene in 0..SCENES {
                    let (mut state, source, object) = contract_scene(cards, card, objects.first().copied().unwrap_or(card), scene);
                    let first = chars_before(cards, &state, &[], object, Layer::Control);
                    // Every permanent of the scene is in the held map - the source and the object
                    // under different players, any other under its own - and each one's stored
                    // controller is flipped in turn: a reach that reads a third permanent's
                    // controller ("as long as you control a Wizard") must read it from the map too.
                    let permanents: Vec<ObjectId> = state.zone(Zone::BATTLEFIELD).to_vec();
                    let held: std::collections::HashMap<ObjectId, PlayerId, Quick> = permanents
                        .iter()
                        .map(|&id| (id, if id == object { PlayerId(1) } else if id == source { PlayerId(0) } else { state.obj(id).controller }))
                        .chain([(source, PlayerId(0)), (object, PlayerId(1))])
                        .collect();
                    let ask = |state: &GameState| {
                        let view = View { controllers: Some(&held), ..View::new(cards, state, &[], Layer::Control) };
                        (st.affects)(&view, source, object, &first)
                    };
                    let before = ask(&state);
                    let flip = |p: PlayerId| PlayerId(1 - p.0.min(1));
                    let mut moved = false;
                    for &id in permanents.iter().chain([source, object].iter()) {
                        let was = state.obj(id).controller;
                        state.update_obj(id, |x| x.controller = flip(was));
                        moved |= ask(&state) != before;
                        state.update_obj(id, |x| x.controller = was);
                    }
                    // Then every one at once: a reach that asks whether any of several permanents is
                    // controlled by someone changes with none of them alone.
                    let all: Vec<ObjectId> = permanents.iter().chain([source, object].iter()).copied().collect();
                    let stored: Vec<PlayerId> = all.iter().map(|&id| state.obj(id).controller).collect();
                    for (&id, &was) in all.iter().zip(&stored) {
                        state.update_obj(id, |x| x.controller = flip(was));
                    }
                    moved |= ask(&state) != before;
                    for (&id, &was) in all.iter().zip(&stored) {
                        state.update_obj(id, |x| x.controller = was);
                    }
                    if moved {
                        breaches.push(format!("{} ({}): its reach read the stored controller, not `LayerQuery::controller`", def.name, st.text));
                        break;
                    }
                }
                continue;
            }
            let mut changed = false;
            let mut broke = None;
            'scenes: for scene in 0..SCENES {
                for &target in objects.iter().take(1) {
                    let (state, source, object) = contract_scene(cards, card, target, scene);
                    let view = View::new(cards, &state, &[], st.layer);
                    let id = if st.own { source } else { object };
                    let apply = |c: &Chars| {
                        let mut c = *c;
                        match st.apply_each {
                            Some(each) => each(&view, source, id, &mut c),
                            None => (st.apply)(&view, if st.own { id } else { source }, &mut c),
                        }
                        c
                    };
                    for &base in &bases {
                        for (x, y) in contract_pairs(base, ability) {
                            let (ax, ay) = (apply(&x), apply(&y));
                            changed |= ax != x || ay != y;
                            if let Some(field) = takes_from_its_object(&x, &ax, &y, &ay, true) {
                                broke = Some(field);
                                break 'scenes;
                            }
                        }
                    }
                }
            }
            if let Some(field) = broke {
                breaches.push(format!("{} ({}): its object's {field}", def.name, st.text));
            } else if !changed {
                inert.push(format!("{} ({})", def.name, st.text));
            }
        }
    }
    breaches.sort();
    breaches.dedup();
    inert.sort();
    inert.dedup();
    ApplyContract { breaches, inert }
}

/// Whether a static ability that could give an object an ability functions anywhere now (CR 613.1f): one of layer 6 that
/// affects other objects, of a permanent, an emblem or a card in a graveyard or exile, or one an effect gave a permanent.
pub fn ability_granting_present(cards: &CardTable, state: &GameState) -> bool {
    state.statics_seen && {
        let found = static_sources(cards, state);
        !found.granted.is_empty() || found.list.iter().any(|&(source, _, _)| cards.ability_statics[cards.current(state.obj(source)).0 as usize])
    }
}



/// One control-changing effect of layer 2 (CR 613.1b).
#[derive(Clone, Copy)]
enum Control {
    /// A static ability of `source` ("you control enchanted creature"): its source's controller controls what it affects.
    Static { source: ObjectId, st: &'static StaticDef },
    /// A resolved effect: `to` controls `affected`.
    Effect { affected: ObjectId, to: PlayerId },
}

/// Layer 2 (CR 613.1b): who controls each permanent, on the battlefield or phased out, as the control-changing effects leave
/// it, and whether any applied to it. The effects - control-changing static abilities and resolved control effects - apply
/// in the layer's order (CR 613.7, 613.8): one depends on another if applying the other changes what it applies to or whom
/// it gives control to (its source's controller), each judged on the objects as layer 1 made them and the controllers the
/// effects applied so far leave (`LayerQuery::controller`), so "you control all creatures" does not reach a land a layer-4
/// effect animates. A dependency loop is broken by timestamp order among its members, every other dependency kept
/// (`docs/rules-exceptions.md`, RX-3). Each permanent starts from whoever controlled it before any effect did
/// (`Marks::control_base`). A source whose controller has left the game gives them nothing (CR 800.4a), and none applies to
/// its own source.
pub fn control_order(cards: &CardTable, state: &GameState, log: &[Record]) -> Vec<(ObjectId, PlayerId, bool)> {
    let objects: Vec<ObjectId> =
        state.battlefield().iter().chain(state.zone(Zone::PHASED_OUT)).copied().filter(|&id| live(state, id)).collect();
    let base = |id: ObjectId| {
        let o = state.obj(id);
        o.marks.control_base.unwrap_or(o.controller)
    };
    let mut entries: Vec<(u32, u8, Control)> = Vec::new();
    if state.statics_seen {
        for source in static_sources(cards, state).controllers.clone() {
            for st in cards.get(cards.current(state.obj(source))).statics.iter().filter(|s| s.layer == Layer::Control) {
                entries.push((state.obj(source).timestamp, 0, Control::Static { source, st }));
            }
        }
    }
    for e in &state.effects {
        if let EffectKind::Control(to) = e.kind
            && objects.contains(&e.affected)
        {
            entries.push((e.timestamp, 2, Control::Effect { affected: e.affected, to }));
        }
    }
    entries.sort_by_key(|&(timestamp, seq, _)| (timestamp, seq));
    let mut controllers: std::collections::HashMap<ObjectId, PlayerId, Quick> = objects.iter().map(|&id| (id, base(id))).collect();
    let mut affected: Vec<ObjectId> = Vec::new();
    // Each object as layer 1 made it, asked once.
    let firsts: std::collections::HashMap<ObjectId, Chars, Quick> = if entries.iter().any(|(_, _, c)| matches!(c, Control::Static { .. })) {
        objects.iter().map(|&id| (id, chars_before(cards, state, log, id, Layer::Control))).collect()
    } else {
        Default::default()
    };
    // What an effect does with the controllers as they stand: each object it applies to, and who gets it.
    let reach = |c: &Control, controllers: &std::collections::HashMap<ObjectId, PlayerId, Quick>| -> Vec<(ObjectId, PlayerId)> {
        match *c {
            Control::Effect { affected, to } => vec![(affected, to)],
            Control::Static { source, st } => {
                let who = controllers.get(&source).copied().unwrap_or(state.obj(source).controller);
                if !state.in_game(who) {
                    return Vec::new();
                }
                let view = View { controllers: Some(controllers), ..View::new(cards, state, log, Layer::Control) };
                objects
                    .iter()
                    .copied()
                    // A phased-out permanent is treated as though it doesn't exist (CR 702.26b, 702.26d): no static ability
                    // reaches it; it keeps its controller and the resolved effects on it.
                    .filter(|&id| {
                        id != source
                            && state.obj(id).zone.is(ZoneKind::Battlefield)
                            && firsts.get(&id).is_some_and(|first| (st.affects)(&view, source, id, first))
                    })
                    .map(|id| (id, who))
                    .collect()
            }
        }
    };
    let mut remaining: Vec<usize> = (0..entries.len()).collect();
    while !remaining.is_empty() {
        let m = remaining.len();
        let now: Vec<Vec<(ObjectId, PlayerId)>> = remaining.iter().map(|&k| reach(&entries[k].2, &controllers)).collect();
        // direct[a][b]: applying b changes what a applies to or whom it gives control to.
        let mut direct = vec![vec![false; m]; m];
        for b in 0..m {
            if now[b].iter().all(|&(id, who)| controllers.get(&id) == Some(&who)) {
                continue;
            }
            let mut after = controllers.clone();
            for &(id, who) in &now[b] {
                after.insert(id, who);
            }
            for a in 0..m {
                if a != b && matches!(entries[remaining[a]].2, Control::Static { .. }) {
                    direct[a][b] = reach(&entries[remaining[a]].2, &after) != now[a];
                }
            }
        }
        let mut through = direct.clone();
        for k in 0..m {
            let via = through[k].clone();
            for row in through.iter_mut() {
                if row[k] {
                    for (cell, &on) in row.iter_mut().zip(&via) {
                        *cell |= on;
                    }
                }
            }
        }
        let next = (0..m).find(|&a| (0..m).all(|b| !direct[a][b] || through[b][a])).unwrap_or(0);
        for &(id, who) in &now[next] {
            controllers.insert(id, who);
            if !affected.contains(&id) {
                affected.push(id);
            }
        }
        remaining.remove(next);
    }
    objects.iter().map(|&id| (id, controllers[&id], affected.contains(&id))).collect()
}

/// The layer a resolved effect of this kind applies in (CR 613.1), as `apply_effect` applies it; none for an effect that
/// is not a characteristic's.
///
/// This is also what counts as an effect that changes a characteristic for `ops::carry_effects` (CR 400.7a: such an
/// effect on a permanent spell goes on applying to the permanent it becomes; control is carried beside it). Every
/// `EffectKind` that changes a characteristic must be listed here, or a spell's would be lost as it resolves.
pub(crate) fn effect_layer(kind: EffectKind) -> Option<Layer> {
    Some(match kind {
        EffectKind::ChangeText { .. } | EffectKind::SetName(_) => Layer::Text,
        EffectKind::AddTypes(_)
        | EffectKind::AddSubtype(_)
        | EffectKind::LoseAllCreatureTypes
        | EffectKind::AddAllCreatureTypes
        | EffectKind::SetCreatureType(_)
        | EffectKind::LoseSubtypesOf(_)
        | EffectKind::SetTypes(_) => Layer::Type,
        EffectKind::SetColors(_) => Layer::Color,
        EffectKind::LoseAllAbilities
        | EffectKind::Protection(_)
        | EffectKind::ProtectionFromColorless
        | EffectKind::ProtectionFromTypes(_)
        | EffectKind::ProtectionFromEverything
        | EffectKind::ProtectionFromPlayer(_)
        | EffectKind::AddKeywords(_)
        | EffectKind::Suspected
        | EffectKind::LoseKeywords(_)
        | EffectKind::AddAbility(_)
        | EffectKind::Enchant(_)
        | EffectKind::LoseEnchant(_)
        | EffectKind::MustAttack
        | EffectKind::Toxic(_) => Layer::Ability,
        EffectKind::SetPt(..) => Layer::PtSet,
        EffectKind::Pump(..) => Layer::PtModify,
        EffectKind::SwitchPt => Layer::PtSwitch,
        _ => return None,
    })
}

/// The static abilities of `layer` effects have given `holder` that affect other objects (CR 613.1f), each with its
/// timestamp (CR 613.7a: the later of the holder's and the granting effect's), that it still has once its abilities are
/// settled: a later "loses all abilities" takes them, as a static ability that removes abilities does.
fn granted_to_others(cx: &Ctx, holder: ObjectId, layer: Layer) -> Vec<(u32, ObjectId, &'static StaticDef, Linked)> {
    let (cards, state) = (cx.cards, cx.state);
    // Whether it has the ability is settled by layer 6, so only abilities of the layers after it can be told apart without
    // asking the layers of the very object being worked out (`granted_static_ability`).
    if layer < Layer::PtDefine {
        return Vec::new();
    }
    let object_timestamp = state.obj(holder).timestamp;
    let mut found: Vec<(u32, AbilityRef, &'static StaticDef, Linked)> = Vec::new();
    for e in state.effects.iter().filter(|e| e.affected == holder) {
        match e.kind {
            EffectKind::AddAbility(ability) => {
                if let AbilityDef::Static(s) = cards.ability(ability)
                    && !s.own
                {
                    found.push((e.timestamp.max(object_timestamp), ability, s, given_ability(cards, state, holder, ability, Giver::of_effect(e))));
                }
            }
            EffectKind::LoseAllAbilities => found.clear(),
            _ => {}
        }
    }
    found.retain(|(_, _, s, _)| s.layer == layer);
    if found.is_empty() {
        return Vec::new();
    }
    let has = chars_in(cx, holder, Layer::PtDefine);
    found.into_iter().filter(|(_, ability, _, _)| has.abilities.contains(&Some(*ability))).map(|(t, _, s, by)| (t, holder, s, by)).collect()
}

/// A continuous effect a resolved spell or ability created (CR 611.2), applied if it is in `layer`.
fn apply_effect(effect: &ContinuousEffect, layer: Layer, chars: &mut Chars) {
    {
        match (layer, effect.kind) {
            // CR 612.1: a subtype in the type line is a word of its text; the rules text's words are read by card code.
            (Layer::Text, EffectKind::ChangeText { from: Word::Subtype(from), to: Word::Subtype(to) }) => {
                // Changeling's creature types stay: its ability applies after the layer-3 change.
                if chars.subtypes.has(from) && !(from.is_creature_type() && chars.keywords.has(Keyword::Changeling)) {
                    chars.subtypes = chars.subtypes.without(from).with(to);
                }
                // CR 612.2: the land type of a landwalk ability is a word of the text, "forestwalk" to "islandwalk"; only the
                // printed abilities are there yet (what layer 6 grants comes after).
                if let (Some(walk), Some(new)) = (Keyword::landwalk(from), Keyword::landwalk(to))
                    && chars.keywords.has(walk)
                {
                    chars.keywords = Keywords(chars.keywords.0 & !Keywords::of(&[walk]).0 | Keywords::of(&[new]).0);
                }
            }
            // CR 612.8: an effect that sets the name is a text-changing effect; the object loses every name it had.
            (Layer::Text, EffectKind::SetName(name)) => {
                chars.name = name;
                chars.name_set = true;
            }
            // CR 612.2: a color word of a "hexproof from [color]" or "protection from [color]" ability is a word of the text;
            // only the printed abilities are there yet (what layer 6 grants comes after).
            (Layer::Text, EffectKind::ChangeText { from: Word::Color(from), to: Word::Color(to) }) => {
                let colors = chars.hexproof_from.colors();
                if colors.has(from) {
                    chars.hexproof_from = chars.hexproof_from.with_colors(ColorSet(colors.0 & !ColorSet::of(from).0).with(to));
                }
                if chars.protection.has(from) {
                    chars.protection = ColorSet(chars.protection.0 & !ColorSet::of(from).0).with(to);
                }
            }
            // CR 205.1b: added types are added; a land that becomes a creature is still a land.
            (Layer::Type, EffectKind::AddTypes(types)) => chars.types = TypeSet(chars.types.0 | types.0),
            (Layer::Type, EffectKind::AddSubtype(s)) => chars.subtypes = chars.subtypes.with(s),
            (Layer::Type, EffectKind::LoseAllCreatureTypes) => {
                chars.subtypes = chars.subtypes.iter().filter(|t| !t.is_creature_type()).fold(SubtypeSet::NONE, SubtypeSet::with);
            }
            // CR 205.3d: creature types belong to creatures and Kindreds.
            (Layer::Type, EffectKind::AddAllCreatureTypes) => {
                if chars.is(CardType::Creature) || chars.is(CardType::Kindred) {
                    chars.subtypes = chars.subtypes.union(SubtypeSet::CREATURE_TYPES);
                }
            }
            // CR 205.1a: a creature type that is set replaces every creature type, and no other subtype.
            // CR 205.3d: the type set is had only by a creature or a Kindred (Turn to Frog's ruling: a God that stops being a
            // creature is a blue enchantment, no Frog).
            (Layer::Type, EffectKind::SetCreatureType(s)) => {
                let kept = chars.subtypes.iter().filter(|t| !t.is_creature_type()).fold(SubtypeSet::NONE, SubtypeSet::with);
                let creature = chars.is(CardType::Creature) || chars.is(CardType::Kindred);
                chars.subtypes = if creature { kept.with(s) } else { kept };
            }
            // CR 205.1a, 613.1d: set types replace every card type but an instant's or sorcery's, which it keeps; a subtype
            // stays if it belongs to a card type the object still has, and goes with the types it lost.
            (Layer::Type, EffectKind::SetTypes(types)) => {
                let spell = TypeSet(chars.types.0 & TypeSet::of(&[CardType::Instant, CardType::Sorcery]).0);
                chars.types = TypeSet(types.0 | spell.0);
                let kept = chars.types;
                chars.subtypes = chars.subtypes.iter().filter(|s| s.card_types().0 & kept.0 != 0).fold(SubtypeSet::NONE, SubtypeSet::with);
            }
            // CR 205.1a: the subtypes of one card type's set go, to be replaced.
            (Layer::Type, EffectKind::LoseSubtypesOf(t)) => {
                chars.subtypes = chars.subtypes.iter().filter(|s| !s.card_types().has(t)).fold(SubtypeSet::NONE, SubtypeSet::with);
            }
            // CR 105.3: "becomes white and blue" is what it is now, not what it is as well.
            (Layer::Color, EffectKind::SetColors(colors)) => chars.colors = colors,
            (Layer::PtSet, EffectKind::SetPt(p, t)) => {
                chars.power = Some(p);
                chars.toughness = Some(t);
            }
            // CR 613.1f: every kind of ability goes, at this effect's timestamp: what is granted after it is kept.
            (Layer::Ability, EffectKind::LoseAllAbilities) => chars.lose_all_abilities(),
            (Layer::Ability, EffectKind::Protection(colors)) => chars.protection = ColorSet(chars.protection.0 | colors.0),
            (Layer::Ability, EffectKind::ProtectionFromColorless) => chars.protection_colorless = true,
            (Layer::Ability, EffectKind::ProtectionFromTypes(types)) => chars.protection_types = TypeSet(chars.protection_types.0 | types.0),
            (Layer::Ability, EffectKind::ProtectionFromEverything) => chars.protection_everything = true,
            (Layer::Ability, EffectKind::ProtectionFromPlayer(p)) => chars.protection_players |= 1 << p.0,
            (Layer::Ability, EffectKind::AddKeywords(k)) => chars.keywords = Keywords(chars.keywords.0 | k.0),
            // CR 701.60c: a suspected permanent has menace and "This creature can't block" for as long as it is suspected.
            (Layer::Ability, EffectKind::Suspected) => chars.keywords = chars.keywords.with(Keyword::Menace).with(Keyword::CantBlock),
            // CR 613.1f: lost at this effect's timestamp, so a keyword granted after it is kept. Losing hexproof loses every
            // "hexproof from [quality]" too (CR 702.11e).
            (Layer::Ability, EffectKind::LoseKeywords(k)) => {
                chars.keywords = Keywords(chars.keywords.0 & !k.0);
                if k.has(Keyword::Hexproof) {
                    chars.hexproof_from = HexproofFrom::NONE;
                }
            }
            (Layer::Ability, EffectKind::AddAbility(ability)) => {
                if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
                    chars.abilities[slot] = Some(ability);
                }
            }
            // An enchant ability an effect gives (CR 702.5a, 613.1f): one of its abilities, which a later "loses all
            // abilities" takes.
            (Layer::Ability, EffectKind::Enchant(card)) => {
                if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
                    chars.abilities[slot] = Some(AbilityRef::enchant_given(card));
                }
            }
            // An enchant ability lost (CR 613.1f), every instance of it (CR 113.10b); the abilities after it close up, in
            // order, as those "loses all abilities except mana abilities" keeps do.
            (Layer::Ability, EffectKind::LoseEnchant(card)) => {
                let lost = AbilityRef::enchant(card);
                let mut kept = [None; cantrip_state::MAX_ABILITIES];
                for (slot, ability) in chars.abilities.iter().flatten().filter(|a| **a != lost).enumerate() {
                    kept[slot] = Some(*ability);
                }
                chars.abilities = kept;
            }
            // A gained "this creature attacks each combat if able" (Carnage, Crimson Chaos). Goad and "attacks that player this
            // combat if able" are effects, not abilities: they are read from the effects as attackers are declared.
            (Layer::Ability, EffectKind::MustAttack) => chars.must_attack += 1,
            // A gained toxic N (CR 702.164a): another instance, its N added to the total toxic value (CR 702.164b).
            (Layer::Ability, EffectKind::Toxic(n)) => chars.toxic += n,
            // CR 613.4d: power and toughness switch, whatever set and modified them first.
            (Layer::PtSwitch, EffectKind::SwitchPt) => std::mem::swap(&mut chars.power, &mut chars.toughness),
            (Layer::PtModify, EffectKind::Pump(p, t)) => {
                chars.power = chars.power.map(|v| v + p);
                chars.toughness = chars.toughness.map(|v| v + t);
            }
            _ => {}
        }
    }
}

/// +1/+1 and -1/-1 counters, in layer 7c (CR 613.4c). Keyword counters are layer 6's, among its effects (`keyword_counters`).
fn counters(obj: &Obj, chars: &mut Chars) {
    let plus = obj.counters(CounterKind::PlusOne) - obj.counters(CounterKind::MinusOne);
    if plus != 0 {
        chars.power = chars.power.map(|v| v + plus);
        chars.toughness = chars.toughness.map(|v| v + plus);
    }
}

/// Hone counters (CR 122.1j): "A hone counter on an Equipment gives +1/+0 to any creature that Equipment is attached to",
/// in layer 7c with the other counters (CR 613.4c). The effect is the counter's, not an ability's, so an Equipment that has
/// lost its abilities still gives it, and one that mentions no hone counter does too (the rulings). Each permanent with hone
/// counters attached to `id` gives one +1/+0 per counter while it is an Equipment; a creature gets nothing from a power it
/// lacks.
fn hone(cx: &Ctx, id: ObjectId, honed: &[ObjectId], chars: &mut Chars) {
    let state = cx.state;
    for &equipment in honed {
        let e = state.obj(equipment);
        // Its types are settled by layer 4, before any power is: what it is attached to cannot change them.
        if e.attached_to == Some(id) && chars_in(cx, equipment, Layer::Color).has_subtype(Subtype::Equipment) {
            let bonus = e.counters(CounterKind::Hone);
            chars.power = chars.power.map(|v| v + bonus);
        }
    }
}

/// A Siege's defeat ability (CR 310.12b): the rules give it to every Siege, after the abilities it prints.
fn intrinsic_siege(cards: &CardTable, chars: &mut Chars) {
    if chars.has_subtype(Subtype::Siege)
        && let Some(slot) = chars.abilities.iter().position(|a| a.is_none())
    {
        chars.abilities[slot] = Some(AbilityRef { card: cards.intrinsic, index: cantrip_api::intrinsic::SIEGE_DEFEAT });
    }
}

/// One mana ability per basic land type the land has (CR 305.6): after its printed mana abilities, its printed types
/// in printed order, then any an effect gave it, in WUBRG order (the C# engine's order).
fn intrinsic_mana(cards: &CardTable, def: &CardDef, chars: &mut Chars) {
    let basic = |m: Mana| match m {
        Mana::W => Subtype::Plains,
        Mana::U => Subtype::Island,
        Mana::B => Subtype::Swamp,
        Mana::R => Subtype::Mountain,
        _ => Subtype::Forest,
    };
    let mut order = [None; 5];
    let mut k = 0;
    for &m in def.basic_land_types {
        if chars.has_subtype(basic(m)) && !order.contains(&Some(m)) {
            order[k] = Some(m);
            k += 1;
        }
    }
    for m in [Mana::W, Mana::U, Mana::B, Mana::R, Mana::G] {
        if chars.has_subtype(basic(m)) && !order.contains(&Some(m)) {
            order[k] = Some(m);
            k += 1;
        }
    }
    if k == 0 {
        return;
    }
    if chars.abilities[0].is_none() {
        for (i, m) in order.into_iter().flatten().enumerate() {
            chars.abilities[i] = Some(AbilityRef { card: cards.intrinsic, index: m as u8 });
        }
        return;
    }
    // After the printed mana abilities, which come first (CR 605; the C# engine numbers them first).
    let at = chars
        .abilities
        .iter()
        .position(|a| !matches!(a, Some(r) if matches!(cards.ability(*r), AbilityDef::Activated(d) if d.is_mana_ability())))
        .unwrap_or(MAX_ABILITIES);
    let mut out = [None; MAX_ABILITIES];
    let mut n = 0;
    for a in chars.abilities[..at].iter().flatten() {
        out[n] = Some(*a);
        n += 1;
    }
    for m in order.into_iter().flatten() {
        if n < MAX_ABILITIES {
            out[n] = Some(AbilityRef { card: cards.intrinsic, index: m as u8 });
            n += 1;
        }
    }
    for a in chars.abilities[at..].iter().flatten() {
        if n < MAX_ABILITIES {
            out[n] = Some(*a);
            n += 1;
        }
    }
    chars.abilities = out;
}

/// Whether the object is still the object it was: it has not changed zones (CR 400.7) or ceased to exist.
pub fn live(state: &GameState, id: ObjectId) -> bool {
    let obj = state.obj(id);
    obj.became.is_none() && !obj.zone.is(ZoneKind::Gone)
}

/// The rules' half of target legality (CR 115.4, 702.11, 702.16): the target still exists where it was, and nothing
/// about it forbids `asker` - the spell, or the ability of its source (`Asker::spell`) - from targeting it.
pub fn can_target(cards: &CardTable, state: &GameState, log: &[Record], asker: &Asker, target: Target) -> bool {
    match target {
        Target::None => false,
        // An ability on the stack can be targeted while it is there (CR 115.1, 115.5).
        Target::Ability(serial) => state.stack.iter().any(|i| i.serial == serial && i.kind != StackKind::Spell),
        // A player with hexproof (CR 702.11c) can't be targeted by their opponents' spells and abilities.
        // A player with protection from everything (CR 702.16j) can't be targeted by anything, their own included; one with
        // protection from a player (CR 702.16k), by nothing that player controls.
        // A player with shroud (CR 702.18a) can't be targeted by anything, their own included.
        Target::Player(p) => {
            let opponent = p != asker.controller;
            !state.player(p).lost
                && !(opponent && state.effects.iter().any(|e| e.kind == EffectKind::PlayerHexproof && e.controller == p))
                && !state.effects.iter().any(|e| e.kind == EffectKind::PlayerProtectionFromEverything && e.controller == p)
                && !player_statics_forbid_targeting(cards, state, log, p, asker.controller)
        }
        Target::Object(id) => {
            if !live(state, id) {
                return false;
            }
            let obj = state.obj(id);
            let on_battlefield = obj.zone.is(ZoneKind::Battlefield);
            // The permanents whose static abilities say what can't target what (CR 601.2c, 604.1): most boards have none.
            let forbidders: Vec<ObjectId> = static_sources(cards, state).targeting.clone();
            if !on_battlefield && forbidders.is_empty() {
                return true;
            }
            let opponent = obj.controller != asker.controller;
            let target_chars = on_battlefield.then(|| chars(cards, state, log, id));
            if let Some(target_chars) = &target_chars {
                // Shroud: no player may target it (CR 702.18a); hexproof: no opponent (CR 702.11b).
                if target_chars.has(Keyword::Shroud) {
                    return false;
                }
                if target_chars.has(Keyword::Hexproof) && opponent && !targeting_rule(cards, state, log, TargetingRuleKind::IgnoreHexproof, id) {
                    return false;
                }
            }
            // A spell not yet cast - a card in a hand, a graveyard, exile or a library - is put on the stack before its targets
            // are chosen (CR 601.2a-c), and has there the characteristics of the face it is cast as and the effects that apply
            // to it there (CR 709.3, 715.3, 712.8), which the card where it is need not have. What reads them is judged with it
            // there (`ops::as_cast`); here it is judged on everything else.
            let source = state.obj(asker.source);
            if asker.spell && !source.zone.is(ZoneKind::Battlefield) && !source.zone.is(ZoneKind::Stack) {
                return true;
            }
            let source_chars = source_chars(cards, state, log, asker.source);
            if let Some(target_chars) = &target_chars {
                // Hexproof from [quality] (CR 702.11d): no [quality] spell an opponent controls, nor an ability an opponent
                // controls from a [quality] source, the source as it is now (a spell, or an ability's source on the
                // battlefield or as it last was). A hexproof ability, so what ignores hexproof ignores it (CR 702.11e).
                if opponent
                    && target_chars.hexproof_from.covers(source_chars.colors, source_chars.types)
                    && !targeting_rule(cards, state, log, TargetingRuleKind::IgnoreHexproof, id)
                {
                    return false;
                }
                if target_chars.protected_from_by(&source_chars, asker.controller) {
                    return false;
                }
            }
            !targeting_forbidden(cards, state, log, &forbidders, id, asker, &source_chars)
        }
    }
}

/// What now could refuse a target for what the spell or ability targeting it is (`can_target`): `None` where nothing could;
/// otherwise the permanents with a hexproof from a quality (CR 702.11d) or a protection (CR 702.16), which only they could
/// be refused for, or every object (`Readers::Any`) where a permanent's static ability says what can't target what
/// (`TargetingRuleKind::Forbid`). Where nothing could, a spell not yet cast is judged on all there is by `can_target` where
/// its card is, and need not be judged on the stack (`actions::priority_options`).
pub fn targeting_readers(cards: &CardTable, state: &GameState, log: &[Record]) -> Option<Readers> {
    if !static_sources(cards, state).targeting.is_empty() {
        return Some(Readers::Any);
    }
    let some: Vec<ObjectId> = state
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let c = chars(cards, state, log, id);
            c.hexproof_from != HexproofFrom::NONE || c.has_protection()
        })
        .collect();
    (!some.is_empty()).then_some(Readers::Some(some))
}

/// The targets whose legality reads the spell or ability targeting them (`targeting_readers`).
pub enum Readers {
    Any,
    Some(Vec<ObjectId>),
}

impl Readers {
    /// Whether `target` is one of them.
    pub fn read_by(&self, target: Target) -> bool {
        match (self, target) {
            (Readers::Any, _) => true,
            (Readers::Some(some), Target::Object(id)) => some.contains(&id),
            _ => false,
        }
    }
}

/// Whether a static ability of one of `forbidders` (`StaticSources::targeting`) with its abilities says `subject` can't be
/// the target of `asker` (`TargetingRuleKind::Forbid`, CR 601.2c, 602.2b, 603.3d, 608.2b): "Thrun can't be the target of
/// nongreen spells your opponents control or abilities from nongreen sources your opponents control". No hexproof
/// ability, so nothing that ignores hexproof lifts it (CR 702.11e); `source` is the spell, or the ability's source as it
/// is now or as it last was (CR 608.2b).
fn targeting_forbidden(cards: &CardTable, state: &GameState, log: &[Record], forbidders: &[ObjectId], subject: ObjectId, asker: &Asker, source: &Chars) -> bool {
    if forbidders.is_empty() {
        return false;
    }
    let view = View::new(cards, state, log, Layer::Ability);
    forbidders.iter().any(|&from| {
        cards.get(cards.current(state.obj(from))).targeting_rules.iter().any(|r| match r.kind {
            TargetingRuleKind::Forbid(forbids) => (r.applies)(&view, from, subject) && forbids(&view, from, asker, source),
            _ => false,
        })
        // A permanent that has lost all its abilities has no static abilities (CR 613.1f).
        && !lost_all_abilities(cards, state, log, from)
    })
}

/// Whether `id` has lost all its abilities (CR 613.1f): to a resolved effect, or to a static ability (`StaticDef::loses_all`)
/// where the layers' order applied it (`Chars::lost_all`). `Query::lost_all_abilities` is this.
///
/// A land another's effect sets the subtype of has lost every ability its rules text gives it (CR 305.7, `Chars::text_lost`):
/// every rule read off its card is gone as it is for a loss of all abilities, though abilities other effects grant it stay.
pub fn lost_all_abilities(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId) -> bool {
    if state.lost_all_abilities_to_an_effect(id) {
        return true;
    }
    // Most positions have neither kind of ability in play: one look at the sources says so.
    if !abilities_may_be_lost(cards, state) {
        return false;
    }
    let c = chars(cards, state, log, id);
    c.lost_all || c.text_lost
}

/// Whether a static ability that functions outside the battlefield exists now (`StaticDef::everywhere`, CR 611.3a: Painter's
/// Servant, Arcane Adaptation): the only thing that gives a card outside the battlefield or a spell an effect, beside its own
/// static abilities - a resolved effect and an ability granted to a permanent apply to neither until the spell exists
/// (`applies`). Where none does, a card judged as the face it would be cast as where it is (`chars_as_face`) has what that
/// spell would have on the stack.
pub fn statics_everywhere(cards: &CardTable, state: &GameState) -> bool {
    state.statics_seen
        && static_sources(cards, state).list.iter().any(|&(source, _, _)| cards.get(cards.current(state.obj(source))).statics.iter().any(|st| st.everywhere))
}

/// Whether any static ability could apply to an object on the battlefield: a permanent, emblem or card whose card has static
/// abilities, or a hone counter, is where its abilities function. The guard of `ops::judged_entering`: without one, a
/// permanent about to enter is on the battlefield what its card or copy effect makes it.
pub fn static_sources_present(cards: &CardTable, state: &GameState) -> bool {
    state.statics_seen && {
        let found = static_sources(cards, state);
        !found.list.is_empty() || !found.honed.is_empty() || !found.granted.is_empty()
    }
}

/// Whether a permanent could lose abilities to a static ability once on the battlefield (CR 613.1f, 305.7): a "loses all
/// abilities" static ability of a permanent that has not lost its own abilities to an effect, or one that sets land subtypes,
/// is on the battlefield. The guard of every question about what a permanent about to enter would have there (CR 614.12,
/// `ops::as_it_would_exist`): no resolved effect can name an object that is not there yet.
pub fn abilities_may_be_lost(cards: &CardTable, state: &GameState) -> bool {
    state.statics_seen && {
        let found = static_sources(cards, state);
        !found.strippers.is_empty() || !found.setters.is_empty()
    }
}

/// Whether a targeting rule of `kind` (CR 609.4, 702.11, 702.21) applies to `subject`: some permanent on the battlefield has a static
/// ability that names it. Found from the board, so asked only when it would change the answer (a hexproof target, a ward trigger).
pub fn targeting_rule(cards: &CardTable, state: &GameState, log: &[Record], kind: TargetingRuleKind, subject: ObjectId) -> bool {
    let sources: Vec<ObjectId> = static_sources(cards, state).targeting.clone();
    let view = View::new(cards, state, log, Layer::Ability);
    sources.into_iter().any(|source| {
        let card = cards.current(state.obj(source));
        // A permanent that has lost all its abilities has no static abilities (CR 613.1f).
        cards.get(card).targeting_rules.iter().any(|r| std::mem::discriminant(&r.kind) == std::mem::discriminant(&kind) && (r.applies)(&view, source, subject))
            && !lost_all_abilities(cards, state, log, source)
    })
}

/// Whether a static ability of a permanent on the battlefield with its abilities gives `player` the keyword ability `keyword`
/// (`CardDef::player_keywords`: True Believer's "You have shroud", CR 702.18a). A board with no such permanent costs a
/// table lookup per permanent.
pub fn player_has(cards: &CardTable, state: &GameState, log: &[Record], player: PlayerId, keyword: Keyword) -> bool {
    let view = View::new(cards, state, log, Layer::Ability);
    state.battlefield().iter().any(|&source| {
        let card = cards.current(state.obj(source));
        // A permanent that has lost all its abilities has no static abilities (CR 613.1f).
        cards.player_keywords[card.0 as usize]
            && cards.get(card).player_keywords.iter().any(|r| r.keyword == keyword && (r.applies)(&view, source, player))
            && !lost_all_abilities(cards, state, log, source)
    })
}

/// Whether a static ability of a permanent on the battlefield with its abilities keeps `asker` from targeting `player`, in
/// one look at the battlefield: shroud (CR 702.18a), hexproof against an opponent (CR 702.11c), or protection from `asker`
/// (CR 702.16b, 702.16k) - what `player_has` and `player_protected_by_static` each say.
fn player_statics_forbid_targeting(cards: &CardTable, state: &GameState, log: &[Record], player: PlayerId, asker: PlayerId) -> bool {
    let view = View::new(cards, state, log, Layer::Ability);
    let opponent = player != asker;
    state.battlefield().iter().any(|&source| {
        let card = cards.current(state.obj(source));
        if !cards.player_keywords[card.0 as usize] {
            return false;
        }
        let def = cards.get(card);
        (def.player_keywords
            .iter()
            .any(|r| (r.keyword == Keyword::Shroud || (opponent && r.keyword == Keyword::Hexproof)) && (r.applies)(&view, source, player))
            || def.player_protection.iter().any(|r| (r.applies)(&view, source, player, asker)))
            && !lost_all_abilities(cards, state, log, source)
    })
}

/// The permanent on the battlefield with its abilities whose static ability gives `player` protection from `from`
/// (`CardDef::player_protection`: Absolute Virtue's "You have protection from each of your opponents", CR 702.16k), if any.
pub fn player_protected_by_static(cards: &CardTable, state: &GameState, log: &[Record], player: PlayerId, from: PlayerId) -> Option<ObjectId> {
    let view = View::new(cards, state, log, Layer::Ability);
    state.battlefield().iter().copied().find(|&source| {
        let card = cards.current(state.obj(source));
        cards.player_keywords[card.0 as usize]
            && cards.get(card).player_protection.iter().any(|r| (r.applies)(&view, source, player, from))
            && !lost_all_abilities(cards, state, log, source)
    })
}

/// What gives `player` protection from objects `from` controls (CR 702.16j, 702.16k), if anything: a resolved effect giving
/// them protection from everything (Teferi's Protection, The One Ring), or a static ability giving them protection from
/// `from` (Absolute Virtue). The object whose protection it is, for ordering it with replacement effects (CR 616.1) and
/// recording a prevention. `from` controls the spell, ability, Aura or damage source - its owner, when no one controls it
/// (CR 702.16k "owned by that player but not controlled by another player", 108.4a): a card off the battlefield and the
/// stack is its owner's (`Object::controller`).
pub fn player_protection(cards: &CardTable, state: &GameState, log: &[Record], player: PlayerId, from: PlayerId) -> Option<ObjectId> {
    state
        .effects
        .iter()
        .find(|e| e.kind == EffectKind::PlayerProtectionFromEverything && e.controller == player)
        .map(|e| e.source)
        .or_else(|| player_protected_by_static(cards, state, log, player, from))
}

/// Summoning sickness (CR 302.6): a creature that has not been under its controller's control continuously since
/// their most recent turn began, and has no haste.
pub fn summoning_sick(cards: &CardTable, state: &GameState, log: &[Record], id: ObjectId) -> bool {
    let obj = state.obj(id);
    let c = chars(cards, state, log, id);
    c.is(CardType::Creature) && !c.has(Keyword::Haste) && obj.controlled_since >= state.player(obj.controller).turn_began
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop(_: &dyn LayerQuery, _: ObjectId, _: &mut Chars) {}

    static HOLDER: CardDef = CardDef {
        name: "Holder",
        delayed: &[granted_static(Layer::PtSet, noop, "This creature's base power is 3.")],
        ..CardDef::BLANK
    };

    fn effect(timestamp: u32, kind: EffectKind) -> ContinuousEffect {
        ContinuousEffect {
            source: ObjectId(1),
            controller: PlayerId(0),
            timestamp,
            affected: ObjectId(1),
            kind,
            duration: Duration::EndOfTurn,
        }
    }

    fn stamps(granted: Vec<(u32, &'static StaticDef, AbilityRef, Giver)>) -> Vec<u32> {
        granted.into_iter().map(|(t, ..)| t).collect()
    }

    /// CR 613.1f, 613.7: "loses all abilities" takes the abilities the object has when it applies, so a grant with a LATER
    /// timestamp is kept, one with an EARLIER timestamp is lost, and an object stripped first and given the ability by a
    /// later effect has it (at the later effect's timestamp, CR 613.7a). No card can express the stripped-first case
    /// with Ms. Marvel: without her abilities she has no Embiggen Fist to make the grant.
    #[test]
    fn a_loses_all_abilities_applies_only_to_grants_before_it() {
        let cards = CardTable::new([&HOLDER]);
        let card = cards.find("Holder").unwrap();
        let ability = AbilityRef::delayed(card, 0);
        let mut abilities = [None; MAX_ABILITIES];
        abilities[0] = Some(ability);
        let chars = Chars {
            card,
            name: "Holder",
            types: TypeSet::NONE,
            supertypes: Supertypes::default(),
            subtypes: SubtypeSet::NONE,
            colors: ColorSet::NONE,
            power: Some(1),
            toughness: Some(4),
            keywords: Keywords::NONE,
            protection: ColorSet::NONE,
            protection_colorless: false,
            protection_players: 0,
            protection_types: TypeSet::NONE,
            spell: false,
            protection_colored_spells: false,
            protection_everything: false,
            hexproof_from: HexproofFrom::NONE,
            must_attack: 0,
            toxic: 0,
            abilities,
            no_cost: false,
            name_set: false,
            lost_all: false,
            text_lost: false,
        };
        let grant = |t| effect(t, EffectKind::AddAbility(ability));
        let strip = |t| effect(t, EffectKind::LoseAllAbilities);

        assert!(granted_statics(&cards, 1, &[grant(5), strip(7)], &chars).is_empty());
        assert_eq!(stamps(granted_statics(&cards, 1, &[strip(5), grant(7)], &chars)), [7]);
        assert_eq!(stamps(granted_statics(&cards, 1, &[grant(3), strip(5), grant(7)], &chars)), [7]);
        assert_eq!(stamps(granted_statics(&cards, 9, &[grant(7)], &chars)), [9]);
    }

    // --- The dependency system (CR 613.8), with static abilities and effects of layer 4 made for the purpose.

    fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.is(CardType::Creature)
    }
    fn land(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.is(CardType::Land)
    }
    fn add_artifact(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.types = c.types.with(CardType::Artifact);
    }
    fn add_creature(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.types = c.types.with(CardType::Creature);
    }
    fn add_enchantment(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.types = c.types.with(CardType::Enchantment);
    }

    fn other_creature(_: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
        id != source && c.is(CardType::Creature)
    }
    fn other_land(_: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
        id != source && c.is(CardType::Land)
    }
    fn other_nonartifact(_: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
        id != source && !c.is(CardType::Artifact)
    }
    fn other_nonenchantment(_: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
        id != source && !c.is(CardType::Enchantment)
    }

    /// "Creatures are artifacts" (Ygra's shape).
    static CREATURES_ARE_ARTIFACTS: CardDef =
        CardDef { name: "Creatures Are Artifacts", statics: &[static_ability(Layer::Type, other_creature, add_artifact, "")], ..CardDef::BLANK };
    /// "Lands are creatures" (Life and Limb's shape).
    static LANDS_ARE_CREATURES: CardDef =
        CardDef { name: "Lands Are Creatures", statics: &[static_ability(Layer::Type, other_land, add_creature, "")], ..CardDef::BLANK };
    static NONARTIFACTS_ARE_ENCHANTMENTS: CardDef = CardDef {
        name: "Nonartifacts Are Enchantments",
        statics: &[static_ability(Layer::Type, other_nonartifact, add_enchantment, "")],
        ..CardDef::BLANK
    };
    static NONENCHANTMENTS_ARE_ARTIFACTS: CardDef = CardDef {
        name: "Nonenchantments Are Artifacts",
        statics: &[static_ability(Layer::Type, other_nonenchantment, add_artifact, "")],
        ..CardDef::BLANK
    };
    static THING: CardDef = CardDef { name: "Thing", ..CardDef::BLANK };
    static CREATURE_THING: CardDef = CardDef { name: "Creature Thing", types: TypeSet::of(&[CardType::Creature]), ..CardDef::BLANK };
    static LAND_THING: CardDef = CardDef { name: "Land Thing", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

    /// The types of the first `things` named, put onto the battlefield after the static abilities' sources in `sources` (in
    /// that timestamp order), once each resolved effect of `effects` (later than all of them) applies to the first.
    fn types_after(sources: &[&str], things: &[&str], effects: &[EffectKind]) -> TypeSet {
        let cards = CardTable::new([
            &CREATURES_ARE_ARTIFACTS,
            &LANDS_ARE_CREATURES,
            &NONARTIFACTS_ARE_ENCHANTMENTS,
            &NONENCHANTMENTS_ARE_ARTIFACTS,
            &THING,
            &CREATURE_THING,
            &LAND_THING,
        ]);
        let names: Vec<&str> = things.iter().chain(sources).copied().collect();
        let (mut state, ids) = battlefield(&cards, &names);
        // The sources after the things: renumber the timestamps so that the sources are the earlier.
        for (k, &id) in ids.iter().enumerate() {
            let timestamp = if k < things.len() { (sources.len() + k + 1) as u32 } else { (k - things.len() + 1) as u32 };
            state.update_obj(id, |o| o.timestamp = timestamp);
        }
        for (k, &kind) in effects.iter().enumerate() {
            state.effect_push(ContinuousEffect { affected: ids[0], timestamp: 100 + k as u32, ..effect(0, kind) });
        }
        chars(&cards, &state, &[], ids[0]).types
    }

    fn types(of: &[CardType]) -> TypeSet {
        TypeSet::of(of)
    }

    /// CR 613.8a: "creatures are artifacts" depends on a later effect that makes the object a creature, and waits for it.
    #[test]
    fn a_static_ability_waits_for_the_later_effect_that_changes_what_it_applies_to() {
        let made = types_after(&["Creatures Are Artifacts"], &["Thing"], &[EffectKind::AddTypes(types(&[CardType::Creature]))]);
        assert_eq!(made, types(&[CardType::Creature, CardType::Artifact]));
    }

    /// CR 613.7, 613.9: an effect that changes nothing about what the other applies to leaves them in timestamp order, and
    /// the later type-setting effect replaces what the earlier one added (Oko's Elk on a creature).
    #[test]
    fn independent_effects_keep_their_timestamp_order() {
        let made = types_after(&["Creatures Are Artifacts"], &["Creature Thing"], &[EffectKind::SetTypes(types(&[CardType::Creature]))]);
        assert_eq!(made, types(&[CardType::Creature]));
    }

    /// CR 613.8b: two effects that each change what the other applies to form a loop, and apply in timestamp order: the
    /// earlier applies, and then the later no longer applies to the object.
    #[test]
    fn a_dependency_loop_applies_in_timestamp_order() {
        let (one, other) = ("Nonartifacts Are Enchantments", "Nonenchantments Are Artifacts");
        assert_eq!(types_after(&[one, other], &["Thing"], &[]), types(&[CardType::Enchantment]));
        assert_eq!(types_after(&[other, one], &["Thing"], &[]), types(&[CardType::Artifact]));
    }

    /// CR 613.8a, 613.8c: whether one effect depends on another is judged on the game as it stands at each step. "Creatures
    /// are artifacts" depends on "lands are creatures" while there is a land for it to make a creature; with none, applying
    /// it changes nothing about what "creatures are artifacts" applies to, so the earlier applies first - before the later
    /// effect that makes the object a land, which "lands are creatures" waits for.
    #[test]
    fn a_chain_of_dependencies_is_judged_on_the_game_at_each_step() {
        let sources = ["Creatures Are Artifacts", "Lands Are Creatures"];
        let land = [EffectKind::AddTypes(types(&[CardType::Land]))];
        assert_eq!(types_after(&sources, &["Thing"], &land), types(&[CardType::Land, CardType::Creature]));
        // A land on the battlefield already: "creatures are artifacts" waits for "lands are creatures", which waits for the
        // effect, and every one applies to the object.
        assert_eq!(types_after(&sources, &["Thing", "Land Thing"], &land), types(&[CardType::Land, CardType::Creature, CardType::Artifact]));
    }

    /// CR 613.8b (`docs/rules-exceptions.md`, RX-3): an effect in a loop that also depends on one outside it waits for that
    /// one; the loop's other member, which depends only on it, goes first though it is the later.
    #[test]
    fn a_loop_member_still_waits_for_what_it_depends_on_outside_the_loop() {
        // "Nonartifacts are enchantments" and "nonenchantments are artifacts" depend on each other; the first also depends on
        // the latest, an effect that makes the object an artifact. So "nonenchantments are artifacts" applies first, and
        // then the first no longer applies.
        let sources = ["Nonartifacts Are Enchantments", "Nonenchantments Are Artifacts"];
        assert_eq!(types_after(&sources, &["Thing"], &[EffectKind::AddTypes(types(&[CardType::Artifact]))]), types(&[CardType::Artifact]));
    }

    // --- Effects in parts (CR 613.6), with static abilities made for the purpose on permanents of a real state.

    fn other_non_aura_enchantment(_: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
        id != source && c.is(CardType::Enchantment) && !c.has_subtype(Subtype::Aura)
    }
    fn noncreature_artifact(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.is(CardType::Artifact) && !c.is(CardType::Creature)
    }
    fn creature_too(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.types = c.types.with(CardType::Creature);
    }
    fn base<const P: i16, const T: i16>(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.power = Some(P);
        c.toughness = Some(T);
    }

    /// Opalescence's shape, its base power and toughness fixed at 4/4 (its mana value): layer 4, then 7b.
    static ANIMATOR: CardDef = CardDef {
        name: "Animator",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[
            static_ability(Layer::Type, other_non_aura_enchantment, creature_too, ""),
            static_ability(Layer::PtSet, other_non_aura_enchantment, base::<4, 4>, "").continues(),
        ],
        ..CardDef::BLANK
    };
    /// Humility's shape: layer 6, then 7b.
    static HUMBLER: CardDef = CardDef {
        name: "Humbler",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[loses_all_abilities(creature, ""), static_ability(Layer::PtSet, creature, base::<1, 1>, "").continues()],
        ..CardDef::BLANK
    };
    /// "All noncreature artifacts are 2/2 artifact creatures" (CR 613.6's third example, as a static ability).
    static MARCH: CardDef = CardDef {
        name: "March",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[
            static_ability(Layer::Type, noncreature_artifact, creature_too, ""),
            static_ability(Layer::PtSet, noncreature_artifact, base::<2, 2>, "").continues(),
        ],
        ..CardDef::BLANK
    };
    static RELIC: CardDef = CardDef { name: "Relic", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

    /// Puts each named card onto the battlefield in turn, with timestamps in that order (CR 613.7d).
    fn battlefield(cards: &CardTable, names: &[&str]) -> (GameState, Vec<ObjectId>) {
        let mut state = GameState::new(2, 0);
        let ids = names
            .iter()
            .enumerate()
            .map(|(k, name)| {
                let mut obj = Obj::new(cards.find(name).unwrap(), PlayerId(0), Zone::BATTLEFIELD);
                obj.timestamp = k as u32 + 1;
                let id = state.push_obj(obj);
                state.zone_push(Zone::BATTLEFIELD, id);
                id
            })
            .collect();
        state.see_statics();
        (state, ids)
    }

    fn pt(cards: &CardTable, state: &GameState, id: ObjectId) -> (Option<i16>, Option<i16>) {
        let c = chars(cards, state, &[], id);
        (c.power, c.toughness)
    }

    /// CR 613.6, its third example: the part of an effect in layer 7b applies to the permanents its layer-4 part applied to,
    /// "even though those permanents aren't noncreature artifacts by then".
    #[test]
    fn a_later_part_applies_to_the_objects_its_first_part_applied_to() {
        let cards = CardTable::new([&MARCH, &RELIC]);
        let (state, ids) = battlefield(&cards, &["March", "Relic"]);
        let relic = chars(&cards, &state, &[], ids[1]);
        assert!(relic.is(CardType::Creature) && relic.is(CardType::Artifact));
        assert_eq!((relic.power, relic.toughness), (Some(2), Some(2)));
    }

    /// CR 613.6, 613.1f, 613.7 (the rulings of Opalescence and Humility): an Opalescence that loses its abilities to
    /// Humility in layer 6 has already begun to apply in layer 4, so its layer-7b part goes on applying, to what its
    /// layer-4 part applied to; Humility's own 7b part goes on applying though Humility took its own abilities. Timestamp
    /// order decides in 7b: with Opalescence, Humility, Opalescence, the second Opalescence is 1/1 and the other two 4/4;
    /// Humility first, everything is 4/4; Humility last, everything is 1/1.
    #[test]
    fn an_effect_that_began_before_its_source_lost_its_abilities_goes_on_applying() {
        let cards = CardTable::new([&ANIMATOR, &HUMBLER]);
        let sizes = |names: &[&str]| {
            let (state, ids) = battlefield(&cards, names);
            ids.iter().map(|&id| pt(&cards, &state, id).0).collect::<Vec<_>>()
        };
        assert_eq!(sizes(&["Animator", "Humbler", "Animator"]), [Some(4), Some(4), Some(1)]);
        assert_eq!(sizes(&["Humbler", "Animator", "Animator"]), [Some(4), Some(4), Some(4)]);
        assert_eq!(sizes(&["Animator", "Animator", "Humbler"]), [Some(1), Some(1), Some(1)]);
        // One Opalescence: it animates Humility alone, which is 1/1 if Humility is the later, 4/4 if the earlier.
        assert_eq!(sizes(&["Animator", "Humbler"]), [None, Some(1)]);
        assert_eq!(sizes(&["Humbler", "Animator"]), [Some(4), None]);
    }

    fn forest_or_saproling(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.has_subtype(Subtype::Forest) || c.has_subtype(Subtype::Saproling)
    }
    fn saproling_forest(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.types = c.types.with(CardType::Creature).with(CardType::Land);
        c.subtypes = c.subtypes.with(Subtype::Saproling).with(Subtype::Forest);
    }
    fn no_forest(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.subtypes = c.subtypes.without(Subtype::Forest);
    }

    /// Life and Limb's shape: "Forests and Saprolings are Saproling creatures and Forest lands".
    static LIMB: CardDef = CardDef {
        name: "Limb",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[static_ability(Layer::Type, forest_or_saproling, saproling_forest, "")],
        ..CardDef::BLANK
    };
    /// "Lands are no Forests", what Blood Moon does to a nonbasic Forest's Forest type.
    static UNFOREST: CardDef = CardDef {
        name: "Unforest",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[static_ability(Layer::Type, land, no_forest, "")],
        ..CardDef::BLANK
    };
    static WOODS: CardDef =
        CardDef { name: "Woods", types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Forest]), ..CardDef::BLANK };

    fn draw_nothing(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
        Ok(())
    }

    /// Blood Sun's shape: "all lands lose all abilities except mana abilities".
    static SUN: CardDef = CardDef {
        name: "Sun",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[loses_all_abilities_except_mana_abilities(land, "")],
        ..CardDef::BLANK
    };
    /// A Swamp with a mana ability, an ability that costs mana and adds none, and a keyword.
    static VAULT: CardDef = CardDef {
        name: "Vault",
        types: TypeSet::of(&[CardType::Land]),
        subtypes: SubtypeSet::of(&[Subtype::Swamp]),
        keywords: Keywords::of(&[Keyword::Indestructible]),
        abilities: &[mana_ability(TAP, add(Mana::C), ""), ability(cost("{1}"), draw_nothing, "")],
        ..CardDef::BLANK
    };

    /// CR 613.1f, 605.1a, 305.6: a land that loses all abilities except mana abilities keeps its printed mana ability and
    /// its basic land type's, and loses the ability that costs mana and its keyword; the rules read off its card are gone
    /// (`lost_all_abilities`). A land card elsewhere is not affected.
    #[test]
    fn a_land_losing_all_but_its_mana_abilities_keeps_them_alone() {
        let cards = CardTable::new([&SUN, &VAULT]);
        let (state, ids) = battlefield(&cards, &["Sun", "Vault"]);
        let vault = chars(&cards, &state, &[], ids[1]);
        let kept: Vec<AbilityRef> = vault.abilities().collect();
        let card = cards.find("Vault").unwrap();
        assert_eq!(kept, [AbilityRef { card, index: 0 }, AbilityRef { card: cards.intrinsic, index: Mana::B as u8 }]);
        assert!(!vault.has(Keyword::Indestructible));
        assert!(lost_all_abilities(&cards, &state, &[], ids[1]));
    }

    /// CR 613.8a(b): an earlier effect depends on a later one that would change what it applies to, judged on the object as
    /// the earlier one found it - its own application must not hide the change. Limb, the older, would make Woods a Saproling,
    /// which would keep Limb applying once Unforest takes the Forest type; but applying Unforest changes what Limb applies to,
    /// so Unforest applies first and Limb then does not apply: Woods is a land and no creature, whichever is older.
    #[test]
    fn an_effect_whose_own_application_hides_the_dependency_still_waits() {
        let cards = CardTable::new([&LIMB, &UNFOREST, &WOODS]);
        for names in [["Limb", "Unforest", "Woods"], ["Unforest", "Limb", "Woods"]] {
            let (state, ids) = battlefield(&cards, &names);
            let woods = chars(&cards, &state, &[], ids[2]);
            assert!(!woods.is(CardType::Creature) && !woods.has_subtype(Subtype::Saproling), "{names:?}");
        }
    }

    // --- The contract of `apply` (what it does is judged on the object as it stands), and its check.

    fn creature_chars(power: i16, flying: bool) -> Chars {
        let cards = CardTable::new([&SPROUT]);
        let mut c = *printed(&cards, cards.find("Sprout").unwrap(), false);
        c.power = Some(power);
        c.toughness = Some(power);
        if flying {
            c.keywords = c.keywords.with(Keyword::Flying);
        }
        c
    }

    /// The check of `apply`'s contract (`takes_from_its_object`): "has flying" on a creature with flying and on one without
    /// (CR 613.9's pair) and "+1/+1" on two powers keep it; "+X/+0 where X is its power" read from the object it was given
    /// breaks it.
    #[test]
    fn an_effect_that_takes_a_value_from_its_object_is_found() {
        let flying = |mut c: Chars| {
            c.keywords = c.keywords.with(Keyword::Flying);
            c
        };
        let (x, y) = (creature_chars(2, true), creature_chars(2, false));
        assert_eq!(takes_from_its_object(&x, &flying(x), &y, &flying(y), true), None, "has flying");
        let pump = |mut c: Chars| {
            c.power = c.power.map(|p| p + 1);
            c
        };
        let (x, y) = (creature_chars(2, false), creature_chars(5, false));
        assert_eq!(takes_from_its_object(&x, &pump(x), &y, &pump(y), true), None, "+1/+1");
        let its_power = |mut c: Chars| {
            c.power = c.power.map(|p| p + p);
            c
        };
        assert_eq!(takes_from_its_object(&x, &its_power(x), &y, &its_power(y), true), Some("power"), "+X/+0, X its power");
        // On a 0-power object it does nothing, which does not excuse the value it takes from its object.
        let (x, y) = (creature_chars(0, false), creature_chars(2, false));
        assert_eq!(takes_from_its_object(&x, &its_power(x), &y, &its_power(y), true), Some("power"), "+X/+0 on a 0-power object");
    }

    // --- A face-down card judged as the card it is (CR 406.3a-b): what is worked out then is kept apart.

    fn any(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, _: &Chars) -> bool {
        true
    }
    fn blue(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.colors = ColorSet::of(Color::Blue);
    }

    /// Painter's Servant's shape: "All cards that aren't on the battlefield, spells, and permanents are blue", everywhere.
    static PAINTER: CardDef = CardDef {
        name: "Painter",
        types: TypeSet::of(&[CardType::Artifact]),
        statics: &[static_ability(Layer::Color, any, blue, "").everywhere()],
        ..CardDef::BLANK
    };
    static SPROUT: CardDef =
        CardDef { name: "Sprout", types: TypeSet::of(&[CardType::Creature]), colors: Some(ColorSet::of(Color::Green)), ..CardDef::BLANK };

    static STOMPING: CardDef =
        CardDef { name: "Stomping", types: TypeSet::of(&[CardType::Instant]), colors: Some(ColorSet::of(Color::Red)), ..CardDef::BLANK };
    static STOMPER: CardDef = CardDef {
        name: "Stomper",
        types: TypeSet::of(&[CardType::Creature]),
        colors: Some(ColorSet::of(Color::Red)),
        adventure: Some(&STOMPING),
        ..CardDef::BLANK
    };

    /// CR 715.3, 601.2a: a card judged as the Adventure it would be cast as is that instant alone, with what applies to it
    /// where it is through the layers - the Painter makes it blue - and the card itself is unchanged after.
    #[test]
    fn a_card_judged_as_its_adventure_has_the_face_and_what_applies_to_it() {
        let cards = CardTable::new([&PAINTER, &STOMPER, &STOMPING]);
        let (mut state, _) = battlefield(&cards, &["Painter"]);
        let card = state.push_obj(Obj::new(cards.find("Stomper").unwrap(), PlayerId(0), Zone::hand(PlayerId(0))));
        state.zone_push(Zone::hand(PlayerId(0)), card);
        let face = chars_as_face(&cards, &state, &[], card, cards.id_of(&STOMPING));
        assert!(face.is(CardType::Instant) && !face.is(CardType::Creature) && face.spell);
        assert_eq!(face.colors, ColorSet::of(Color::Blue));
        let whole = chars(&cards, &state, &[], card);
        assert!(whole.is(CardType::Creature) && !whole.spell);
    }

    /// CR 406.3a: a card exiled face down has no characteristics; judged as the card it is (`judged_face_up`), it is the
    /// card with what applies to it (blue, under the Painter). Asked again once the judgment is over, it has none: what was
    /// worked out while it was judged is not what the position's pass keeps.
    #[test]
    fn a_face_down_card_judged_as_the_card_it_is_poisons_nothing() {
        let cards = CardTable::new([&PAINTER, &SPROUT]);
        let (mut state, ids) = battlefield(&cards, &["Painter"]);
        let mut obj = Obj::new(cards.find("Sprout").unwrap(), PlayerId(0), Zone::exile(PlayerId(0)));
        obj.face_down = true;
        obj.timestamp = 9;
        let card = state.push_obj(obj);
        state.zone_push(Zone::exile(PlayerId(0)), card);
        let _ = ids;
        assert_eq!(chars(&cards, &state, &[], card).colors, ColorSet::NONE, "no characteristics");
        {
            let _judged = judged_face_up(&state, card);
            let judged = chars(&cards, &state, &[], card);
            assert_eq!(judged.colors, ColorSet::of(Color::Blue), "the card, made blue");
            assert!(judged.is(CardType::Creature));
        }
        assert_eq!(chars(&cards, &state, &[], card).colors, ColorSet::NONE, "no characteristics again");
        assert!(!chars(&cards, &state, &[], card).is(CardType::Creature));
    }

    // --- Control-changing static abilities (CR 613.1b, layer 2).

    fn attached(view: &dyn LayerQuery, source: ObjectId, id: ObjectId, _: &Chars) -> bool {
        view.state().obj(source).attached_to == Some(id)
    }

    /// "You control enchanted creature" (Mind Control's shape).
    static CONTROLLER: CardDef = CardDef {
        name: "Controller",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[control_changing(attached, "You control enchanted creature.")],
        ..CardDef::BLANK
    };
    static BEAR: CardDef = CardDef { name: "Bear", types: TypeSet::of(&[CardType::Creature]), ..CardDef::BLANK };

    /// CR 613.1b, 613.7e: each control-changing ability that applies to the permanent gives its source's controller control,
    /// at its source's timestamp; one attached elsewhere gives nothing, and none applies to its own source.
    #[test]
    fn a_control_changing_static_ability_gives_its_controller_what_it_affects() {
        let cards = CardTable::new([&CONTROLLER, &BEAR]);
        let (mut state, ids) = battlefield(&cards, &["Bear", "Bear", "Controller", "Controller"]);
        state.update_obj(ids[2], |o| {
            o.attached_to = Some(ids[0]);
            o.controller = PlayerId(1);
        });
        state.update_obj(ids[3], |o| o.attached_to = Some(ids[0]));
        state.see_statics();
        let order = control_order(&cards, &state, &[]);
        let of = |id| order.iter().find(|&&(o, _, _)| o == id).map(|&(_, p, by)| (p, by));
        // Both apply to the first Bear; the later, of the second Controller, decides.
        assert_eq!(of(ids[0]), Some((PlayerId(0), true)));
        assert_eq!(of(ids[1]), Some((PlayerId(0), false)));
        assert_eq!(of(ids[2]), Some((PlayerId(1), false)));
    }

    /// CR 613.1: layer 2 applies before layer 6 could take the ability away, so a source that has lost all its abilities to
    /// an effect still gives control.
    #[test]
    fn a_control_changing_ability_applies_though_its_source_lost_its_abilities() {
        let cards = CardTable::new([&CONTROLLER, &BEAR]);
        let (mut state, ids) = battlefield(&cards, &["Bear", "Controller"]);
        state.update_obj(ids[1], |o| {
            o.attached_to = Some(ids[0]);
            o.controller = PlayerId(1);
        });
        state.effect_push(ContinuousEffect { affected: ids[1], ..effect(9, EffectKind::LoseAllAbilities) });
        state.see_statics();
        let order = control_order(&cards, &state, &[]);
        assert!(order.contains(&(ids[0], PlayerId(1), true)));
    }

    /// CR 612.2: the land type of a printed landwalk and the color of a printed protection are words of the text, which a
    /// text-changing effect changes in layer 3: "swampwalk" becomes "islandwalk", "protection from black" "protection from red".
    #[test]
    fn a_text_change_changes_a_printed_landwalk_and_protection() {
        let cards = CardTable::new([&HOLDER]);
        let mut chars = *printed(&cards, cards.find("Holder").unwrap(), false);
        chars.keywords = Keywords::of(&[Keyword::Swampwalk, Keyword::Flying]);
        chars.protection = ColorSet::of(Color::Black);
        let swamp = effect(5, EffectKind::ChangeText { from: Word::Subtype(Subtype::Swamp), to: Word::Subtype(Subtype::Island) });
        apply_effect(&swamp, Layer::Text, &mut chars);
        assert_eq!(chars.keywords, Keywords::of(&[Keyword::Islandwalk, Keyword::Flying]));
        let black = effect(6, EffectKind::ChangeText { from: Word::Color(Color::Black), to: Word::Color(Color::Red) });
        apply_effect(&black, Layer::Text, &mut chars);
        assert_eq!(chars.protection, ColorSet::of(Color::Red));
        // A creature type is no land type: no landwalk names it.
        let elf = effect(7, EffectKind::ChangeText { from: Word::Subtype(Subtype::Elf), to: Word::Subtype(Subtype::Goblin) });
        apply_effect(&elf, Layer::Text, &mut chars);
        assert_eq!(chars.keywords, Keywords::of(&[Keyword::Islandwalk, Keyword::Flying]));
    }

    // --- Copy exceptions that state types (CR 707.9b, 707.9d, 205.1a).

    /// A legendary green artifact creature - Equipment Shapeshifter with changeling and flying, 2/3: everything "except it's a
    /// 5/5 black Demon" replaces, and what it keeps.
    static IDOL: CardDef = CardDef {
        name: "Idol",
        colors: Some(ColorSet::of(Color::Green)),
        types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
        supertypes: Supertypes { basic: false, legendary: true, snow: false },
        subtypes: SubtypeSet::of(&[Subtype::Equipment, Subtype::Shapeshifter]),
        keywords: Keywords::of(&[Keyword::Changeling, Keyword::Flying]),
        power: Some(2),
        toughness: Some(3),
        ..CardDef::BLANK
    };
    /// An instant with a spell type, for the type CR 205.1a keeps.
    static TRICK: CardDef = CardDef {
        name: "Trick",
        types: TypeSet::of(&[CardType::Instant]),
        subtypes: SubtypeSet::of(&[Subtype::Arcane]),
        ..CardDef::BLANK
    };

    fn ardyn(card: CardId) -> Copiable {
        Copiable::of(card)
            .typed(TypeSet::of(&[CardType::Creature]), SubtypeSet::of(&[Subtype::Demon]))
            .colored(ColorSet::of(Color::Black))
            .sized(5, 5)
    }

    /// CR 707.9b, 205.1a, 707.9d (Ardyn, the Usurper's ruling: "a Demon creature instead of its other types and ... black
    /// instead of its other colors. Its base power and toughness are 5/5"): a copy "except it's a 5/5 black Demon" is a
    /// Demon creature alone - no artifact, no Equipment, no Shapeshifter, and no creature type from Changeling, which defines
    /// the subtypes the exception states and so is not copied - black, 5/5, still legendary (a supertype) and still flying.
    /// A copy of that copy has the same values (CR 707.9b), and an exception applied on top of them adds to them.
    #[test]
    fn a_copy_except_its_a_demon_is_a_demon_creature_instead_of_its_other_types() {
        let cards = CardTable::new([&IDOL, &TRICK]);
        let idol = cards.find("Idol").unwrap();
        let (mut state, ids) = battlefield(&cards, &["Idol", "Idol", "Idol", "Idol"]);
        // The card itself, for contrast: its own types, and every creature type.
        let own = chars(&cards, &state, &[], ids[0]);
        assert!(own.is(CardType::Artifact) && own.has_subtype(Subtype::Equipment) && own.has_subtype(Subtype::Demon));
        state.update_obj(ids[1], |o| o.copy = Some(ardyn(idol)));
        // A copy of the copy (Clone): its copiable values are the copy's, exception included.
        let values = state.obj(ids[1]).copy.unwrap();
        state.update_obj(ids[2], |o| o.copy = Some(values));
        // A copy of the copy with an exception of its own on top (Phantasmal Image's Illusion).
        state.update_obj(ids[3], |o| o.copy = Some(values.with_subtype(Subtype::Illusion).with_type(CardType::Artifact)));
        for id in [ids[1], ids[2]] {
            let c = chars(&cards, &state, &[], id);
            assert_eq!(c.types, TypeSet::of(&[CardType::Creature]));
            assert_eq!(c.subtypes, SubtypeSet::of(&[Subtype::Demon]));
            assert_eq!(c.colors, ColorSet::of(Color::Black));
            assert_eq!((c.power, c.toughness), (Some(5), Some(5)));
            assert!(c.supertypes.legendary && c.has(Keyword::Flying) && !c.has(Keyword::Changeling));
            assert_eq!(c.name, "Idol");
        }
        let image = chars(&cards, &state, &[], ids[3]);
        assert_eq!(image.types, TypeSet::of(&[CardType::Artifact, CardType::Creature]));
        assert_eq!(image.subtypes, SubtypeSet::of(&[Subtype::Demon, Subtype::Illusion]));
        // Stated types applied over an earlier exception's replace them (The Scarab God copying the token's card), and an
        // exception that gives changeling gives every creature type again.
        let zombie_values = values.with_type(CardType::Artifact).typed(TypeSet::of(&[CardType::Creature]), SubtypeSet::of(&[Subtype::Zombie]));
        state.update_obj(ids[3], |o| o.copy = Some(zombie_values));
        let zombie = chars(&cards, &state, &[], ids[3]);
        assert_eq!(zombie.types, TypeSet::of(&[CardType::Creature]));
        assert_eq!(zombie.subtypes, SubtypeSet::of(&[Subtype::Zombie]));
        state.update_obj(ids[3], |o| o.copy = Some(zombie_values.with_keyword(Keyword::Changeling)));
        let changeling = chars(&cards, &state, &[], ids[3]);
        assert!(changeling.has_subtype(Subtype::Demon) && changeling.has(Keyword::Changeling) && !changeling.has_subtype(Subtype::Equipment));
        assert!(values.has_exceptions() && Copiable::of(idol).typed(TypeSet::NONE, SubtypeSet::NONE).has_exceptions());
    }

    /// CR 205.1a: "an object with either the instant or sorcery card type retains that type", and its spell type with it.
    #[test]
    fn a_copy_that_states_types_keeps_an_instants_type() {
        let cards = CardTable::new([&IDOL, &TRICK]);
        let (mut state, ids) = battlefield(&cards, &["Trick"]);
        let trick = cards.find("Trick").unwrap();
        state.update_obj(ids[0], |o| o.copy = Some(ardyn(trick)));
        let c = chars(&cards, &state, &[], ids[0]);
        assert_eq!(c.types, TypeSet::of(&[CardType::Creature, CardType::Instant]));
        assert_eq!(c.subtypes, SubtypeSet::of(&[Subtype::Arcane, Subtype::Demon]));
    }
}

#[cfg(test)]
mod player_keyword_tests {
    use super::*;

    fn controller_is(q: &dyn LayerQuery, source: ObjectId, player: PlayerId) -> bool {
        q.state().obj(source).controller == player
    }

    static VEIL: CardDef = CardDef {
        name: "Player Shroud Test Veil",
        player_keywords: &[PlayerKeywordRule { keyword: Keyword::Shroud, applies: controller_is }],
        ..CardDef::BLANK
    };
    static WARD: CardDef = CardDef {
        name: "Player Hexproof Test Ward",
        player_keywords: &[PlayerKeywordRule { keyword: Keyword::Hexproof, applies: controller_is }],
        ..CardDef::BLANK
    };
    static SPARK: CardDef = CardDef { name: "Player Keyword Test Spark", ..CardDef::BLANK };

    fn put(cards: &CardTable, state: &mut GameState, name: &str, controller: u8, zone: Zone) -> ObjectId {
        let id = state.push_obj(Obj::new(cards.find(name).unwrap(), PlayerId(controller), zone));
        state.zone_push(zone, id);
        id
    }

    /// CR 702.18a: a player with shroud can't be the target of any spell or ability, their own included; CR 702.11c: a
    /// player with hexproof can't be the target of their opponents' only. Both last only while the permanent whose static
    /// ability gives them is on the battlefield (CR 611.3a), and the other player is untouched.
    #[test]
    fn a_static_ability_gives_its_controller_shroud_or_hexproof_while_it_is_on_the_battlefield() {
        let cards = CardTable::new([&VEIL, &WARD, &SPARK]);
        let mut state = GameState::new(2, 0);
        let veil = put(&cards, &mut state, "Player Shroud Test Veil", 0, Zone::BATTLEFIELD);
        let ward = put(&cards, &mut state, "Player Hexproof Test Ward", 1, Zone::BATTLEFIELD);
        let mine = Asker { source: put(&cards, &mut state, "Player Keyword Test Spark", 0, Zone::hand(PlayerId(0))), controller: PlayerId(0), by: None, spell: false };
        let theirs = Asker { source: put(&cards, &mut state, "Player Keyword Test Spark", 1, Zone::hand(PlayerId(1))), controller: PlayerId(1), by: None, spell: false };
        state.see_statics();
        assert!(player_has(&cards, &state, &[], PlayerId(0), Keyword::Shroud));
        assert!(!player_has(&cards, &state, &[], PlayerId(1), Keyword::Shroud));
        assert!(!player_has(&cards, &state, &[], PlayerId(0), Keyword::Hexproof));
        // Shroud: nobody targets player 0, player 0 included.
        assert!(!can_target(&cards, &state, &[], &mine, Target::Player(PlayerId(0))));
        assert!(!can_target(&cards, &state, &[], &theirs, Target::Player(PlayerId(0))));
        // Hexproof: player 1 targets themselves, player 0 can't target them.
        assert!(can_target(&cards, &state, &[], &theirs, Target::Player(PlayerId(1))));
        assert!(!can_target(&cards, &state, &[], &mine, Target::Player(PlayerId(1))));
        // Gone from the battlefield, the abilities are gone.
        state.zone_remove(Zone::BATTLEFIELD, veil);
        state.zone_remove(Zone::BATTLEFIELD, ward);
        assert!(can_target(&cards, &state, &[], &theirs, Target::Player(PlayerId(0))));
        assert!(can_target(&cards, &state, &[], &mine, Target::Player(PlayerId(1))));
    }
}

#[cfg(test)]
mod pass_read_tests {
    use super::*;

    /// A pass whose work asked the past, used again for the same history, asks it again as far as the rules that find a
    /// loop of passes are concerned (`loops`): what it holds depends on the history, though nothing reads it this time. A
    /// pass that never asked the past is used again without a read.
    #[test]
    fn a_kept_pass_that_asked_the_past_notes_a_read_when_used_again() {
        let cards = CardTable::new([]);
        let state = GameState::new(2, 0);
        let first = pass(&cards, &state, &[]);
        cantrip_api::history::take_reads();
        let again = pass(&cards, &state, &[]);
        assert!(std::rc::Rc::ptr_eq(&first, &again));
        assert_eq!(cantrip_api::history::take_reads(), 0, "it never asked the past");
        first.log.set(Some(0));
        let again = pass(&cards, &state, &[]);
        assert!(std::rc::Rc::ptr_eq(&first, &again));
        assert_eq!(cantrip_api::history::take_reads(), 1, "what it holds was worked out asking the past");
    }
}
