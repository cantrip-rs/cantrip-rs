//! What card code may do: read anything ([`Query`]), and change the game only through the operations of [`Game`],
//! each of which is a rules procedure the core owns (zone changes go through replacement, damage through
//! prevention, everything through the journal).

use cantrip_state::*;

use crate::card::{CounterTo, EntersAs, FaceDown, LibraryPlace, TurnUp, WayDef};
use crate::card::{AbilityDef, ActivatedDef, Asker, CardDef, CauseKind, PlayerCounter};
use crate::chars::Chars;
use crate::history::{History, Record};
use crate::DamageResult;

/// A die roll (CR 706.2): the face the die showed, and the result once the roll was modified (CR 706.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Roll {
    pub natural: u32,
    pub result: i32,
}

/// The effect has asked a question that has no answer yet. Returned with `?` all the way out of the card's code; the
/// engine records the question and runs the effect again once it is answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suspend;

/// Whether an object is still the object it was: it has not changed zones (CR 400.7) or ceased to exist.
pub fn is_live(q: &dyn Query, id: ObjectId) -> bool {
    let obj = q.obj(id);
    obj.became.is_none() && !obj.zone.is(ZoneKind::Gone)
}

/// The defending player of an attack at `attack` being made now (CR 508.5, 310.9d): the player attacked, the controller
/// of the planeswalker attacked, or the protector of the battle attacked. None for anything else. Only the core asks it,
/// as a creature begins attacking (`Attacker::defending`); afterwards a creature's defending player is the one it had
/// then (`Query::defending_player`), whoever controls that planeswalker now.
pub fn defending_player_of(state: &GameState, attack: Target) -> Option<PlayerId> {
    match attack {
        Target::Player(p) => Some(p),
        Target::Object(id) => {
            let obj = state.obj(id);
            obj.marks.protector.or(Some(obj.controller))
        }
        _ => None,
    }
}

/// What a cast made as an effect resolves (CR 608.2g) is cast for, beyond its being cast.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CastTerms {
    /// The cost paid instead of the mana cost (CR 118.9).
    pub instead: Option<ManaCost>,
    /// Mana of any type may pay for it (CR 609.4b, 118.14).
    pub any_type: bool,
    /// Generic mana added to its cost (CR 601.2f).
    pub more: u8,
    /// Generic mana taken off what an {X} in `instead` adds, as X is announced and before anything increases the cost, never
    /// below nothing (CR 107.3a, 118.7a): what a reduction of a cost paid instead leaves for X once the cost's own generic
    /// mana is gone (Aminatou, Veil Piercer's "its mana cost reduced by {4}" on The Meathook Massacre).
    pub less: u8,
    /// Life paid with its cost (CR 118.9, 119.4): Bolas's Citadel's mana value.
    pub life: u8,
    /// "If that spell would be put into your graveyard, exile it instead" (CR 614.1a), as the effect's text words it: of any
    /// spell (`ExileInstead::Spell`), or only of an instant or sorcery spell (`ExileInstead::InstantOrSorcery`). A move to a
    /// hand or a library is not replaced.
    pub exile_instead: ExileInstead,
    /// Cast with flashback (CR 702.34a): exiled instead of put anywhere else any time it would leave the stack.
    pub flashback: bool,
    /// Cast a copy of the card, which stays where it is (CR 707.12, 702.170a): the copy is a card_copy in the same zone, and
    /// ceases to exist when it leaves the stack or the cast is given up.
    pub copy: bool,
    /// The way it is cast (CR 709.3, 715.3, 712.8a): one of `CastWay::way` from `cast_ways_during_resolution`, for a card that
    /// can be cast as a half, its Adventure or Omen, or its back face. Left out, a card with one way is cast that way and one with
    /// several asks its caster which, as they choose it as they cast it.
    pub way: Option<u8>,
    /// The player who casts it (CR 608.2g, 601.2), where the effect names one other than its controller: "the exiled card's
    /// owner may cast that card" (Spell Queller), "that player may cast" (Knowledge Pool, Eye of the Storm), "that opponent
    /// may cast" (Allure of the Unknown). They announce it, choose its modes, targets and costs, pay them and control the
    /// spell; its face is asked of them, and what keeps them from casting it is judged for them. Left out, the resolving
    /// effect's controller casts it. Whether they may is judged by `could_cast_during_resolution_by` and
    /// `cast_ways_during_resolution_by` with the same player.
    pub caster: Option<PlayerId>,
}

/// One way a card could be cast as an effect resolves (CR 608.2g): the card as itself, one half of a split card (CR 709.3),
/// its Adventure (CR 715.3) or Omen (CR 720.3), or the back face of a modal double-faced card (CR 712.8a). Only that face is evaluated to see if
/// it can be cast (CR 709.3a, 715.3a, 712.8a) and only it is on the stack (CR 709.3b), so its mana value is the spell's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastWay {
    /// What `CastTerms::way` names.
    pub way: u8,
    /// The face that would be cast.
    pub face: CardId,
    /// Its mana value as the spell on the stack: the printed cost's, an X in it being 0 (CR 107.3b).
    pub mana_value: u32,
}

/// Whether a spell or ability controlled by `causer` may cause `player` to discard, sacrifice or search, as `kind` says
/// (CR 101.2): nothing on the battlefield forbids it. Judged when the effect would ask or move, so an impossible discard,
/// sacrifice or search is not offered as a choice either.
pub fn may_cause(q: &dyn Query, causer: PlayerId, player: PlayerId, kind: CauseKind) -> bool {
    for &id in q.state().battlefield() {
        let o = q.obj(id);
        let def = q.card(o.copied_card().unwrap_or(o.card));
        // A permanent that has lost all its abilities has no such rule (CR 613.1f).
        if def.cause_rules.is_empty() || q.lost_all_abilities(id) {
            continue;
        }
        let asker = Asker { source: id, controller: o.controller, by: None, spell: false };
        if def.cause_rules.iter().any(|rule| !rule(q, &asker, causer, player, kind)) {
            return false;
        }
    }
    true
}

/// An object's mana cost (CR 202.1) as its characteristics `c` give it (`Query::mana_cost`): the card's or the one it
/// copies, none for a copy that has none (CR 707.9b) or a face-down permanent (CR 708.2a), and a Room's unlocked doors on
/// the battlefield (CR 709.5).
pub fn mana_cost_of(card: &dyn Fn(CardId) -> &'static CardDef, obj: &Obj, c: &Chars) -> ManaCost {
    if c.no_cost {
        return ManaCost::ZERO;
    }
    let def = card(c.card);
    // A Room on the battlefield has the mana cost of its unlocked doors alone (CR 709.5); anywhere else the card's.
    if def.is_room() && obj.zone.is(ZoneKind::Battlefield) { def.room_cost(obj.marks.doors) } else { def.cost }
}

/// An object's mana value (CR 202.3) as its characteristics `c` give it (`Query::mana_value`, `LayerQuery::mana_value`).
pub fn mana_value_of(card: &dyn Fn(CardId) -> &'static CardDef, state: &GameState, id: ObjectId, c: &Chars) -> u32 {
    let obj = state.obj(id);
    let front = card(obj.card);
    // CR 202.3b, 712.8c, 712.8e: a nonmodal double-faced permanent with its back face up, or a spell of one cast transformed (its
    // own back face as the face it is on the stack as, not a copy of one: `card_copy`), has its front face's mana cost for its
    // mana value; a copy of a back face has 0.
    let cast_transformed = obj.zone.is(ZoneKind::Stack) && !obj.card_copy && front.back.is_some_and(|back| std::ptr::eq(card(c.card), back));
    let cost = match front.back {
        Some(back)
            if ((obj.transformed && obj.copied_card().is_none()) || cast_transformed) && !back.has_cost && !front.back_land && !c.no_cost =>
        {
            front.cost
        }
        _ => mana_cost_of(card, obj, c),
    };
    let x = if obj.zone.is(ZoneKind::Stack) { obj.x } else { state.left_stack(id).map_or(0, |item| item.x) };
    cost.mana_value() + cost.x as u32 * x as u32
}

/// Reading the game.
pub trait Query {
    fn state(&self) -> &GameState;

    /// The "defending player" of `attacker`, an attacking creature or one that attacked (CR 508.5, 310.9d): the player it
    /// attacks, or the controller of the planeswalker or the protector of the battle it attacks, as it began attacking
    /// (`Attacker::defending`) - so still once that planeswalker or battle has changed controller or protector, or been
    /// removed from combat (CR 506.4c) - and, once it is no longer attacking, the one it had when it attacked last
    /// (`Event::Attacks::defending`). None for a creature that has not attacked.
    fn defending_player(&self, attacker: ObjectId) -> Option<PlayerId> {
        if let Some(a) = self.state().attackers.iter().find(|a| a.id == attacker) {
            return Some(a.defending);
        }
        self.history().all().iter().rev().find_map(|r| match r.event {
            crate::Event::Attacks { attacker: a, defending, .. } if a == attacker => Some(defending),
            _ => None,
        })
    }

    fn card(&self, id: CardId) -> &'static CardDef;

    /// The activated ability (CR 602) an ability reference names, if it names one: what an activation recorded in the
    /// history was, or one of an object's abilities (`chars(id).abilities()`) is - a printed one, or one an effect grants
    /// (`AbilityRef::delayed`, in the granting card's `delayed` list: Deconstruction Hammer's). The core's own abilities and
    /// the history's markers (madness, rebound, miracle, a granted way used, extra loyalty activations allowed) are none.
    fn activated_ability(&self, ability: AbilityRef) -> Option<&'static ActivatedDef> {
        let card = self.card(ability.card);
        let def = if ability.is_delayed() {
            card.delayed.get((ability.index - AbilityRef::DELAYED) as usize)
        } else {
            card.abilities.get(ability.index as usize)
        };
        match def {
            Some(AbilityDef::Activated(def)) => Some(def),
            _ => None,
        }
    }

    /// An object's rules information after every continuous effect (CR 613). A phased-out source is read as it last
    /// existed phased in (CR 608.2h, 702.15c); projected current characteristics are separate from source information.
    fn chars(&self, id: ObjectId) -> Chars;

    /// Token characteristics at creation, including copy exceptions (CR 614.12, 707.2). Used by token-creation
    /// replacements.
    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars;

    /// Whether `id` has lost all its abilities (CR 613.1f), to a resolved effect or a static ability, so that the
    /// abilities its card prints are gone: the one answer every rule read off a card is judged by - static abilities
    /// from layer 6 on, play rules, cost changes, replacement effects, trigger and block rules - so that they agree with
    /// layer 6. Abilities granted after the loss are the object's own and do not come back through the card. A land an effect
    /// sets the subtype of has lost the abilities of its rules text (CR 305.7), and answers so too.
    fn lost_all_abilities(&self, id: ObjectId) -> bool;

    /// The words an object's text has now (CR 612): every color word, basic land type and creature type its rules text
    /// names is read through these, `words(source).subtype(Subtype::Goblin)` for "Goblin", so a text-changing effect
    /// changes what the card does.
    fn words(&self, id: ObjectId) -> Words {
        crate::words(self.state(), id)
    }

    /// The id of a definition in the card table (a token's, an exception's).
    fn card_id(&self, def: &'static CardDef) -> CardId;

    /// Every event since the game began, in order (`crate::history`).
    fn events(&self) -> &[Record];

    /// The game's event history, for any question about what has happened (`crate::history`).
    fn history(&self) -> History<'_> {
        History::new(self.events(), self.state().turn.number)
    }

    /// The objects of the batch a "one or more" ability (`TriggeredDef::once_per_batch`) was triggered by
    /// (CR 603.2c, 608.2h).
    fn event_objects(&self, item: &StackItem) -> &[ObjectId] {
        self.state().members(item.members)
    }

    /// The permanents an activated ability's cost chose and tapped, sacrificed or returned (CR 601.2h by way of 602.2b),
    /// in the order chosen: "Put a number of charge counters on this permanent equal to the tapped creature's power"
    /// (station, CR 702.184a), "deals damage equal to the tapped creature's power" (Unerring Sling), "where X is the power of
    /// the creature tapped this way" (Keldon Battlewagon, Impelled Giant). A crew or saddle cost's creatures are among
    /// them, and after them any card the cost revealed from a hand (behold, CR 701.4a). Each is the object as it was chosen:
    /// read its characteristics with `chars`, which gives those it last had on the battlefield once it has left (CR 608.2h).
    /// Empty for a spell or a triggered ability.
    fn cost_objects(&self, item: &StackItem) -> &[ObjectId] {
        if item.kind != StackKind::Activated {
            return &[];
        }
        self.state().members(item.members)
    }

    fn obj(&self, id: ObjectId) -> &Obj {
        self.state().obj(id)
    }

    fn is_on_battlefield(&self, id: ObjectId) -> bool {
        self.obj(id).zone.is(ZoneKind::Battlefield)
    }

    /// Current controller in the expected public zone, or the source's last controller while absent (CR 608.2h).
    fn controller(&self, id: ObjectId) -> PlayerId {
        self.state().last_known(id).or_else(|| self.state().phased_last_known(id)).map_or(self.obj(id).controller, |last| last.controller)
    }

    /// An object's mana cost (CR 202.1) as its characteristics give it: the card's or the one it copies, none for a copy
    /// that has none (CR 707.9b) or a face-down permanent (CR 708.2a), and a Room's unlocked doors on the battlefield
    /// (CR 709.5). Its {X} symbols are counted in `x`, not valued.
    fn mana_cost(&self, id: ObjectId) -> ManaCost {
        mana_cost_of(&|card| self.card(card), self.obj(id), &self.chars(id))
    }

    /// An object's mana value (CR 202.3): its mana cost's, or 0 for a copy that has none (CR 707.9b). Each {X} in it
    /// is the value chosen while it is a spell on the stack, as it last was there once it has left (CR 202.3e,
    /// 608.2h), and 0 anywhere else.
    ///
    /// The back face of a nonmodal double-faced object, up, has no mana cost but the mana value of its front face's
    /// (CR 202.3b, 712.8c, 712.8e: a transformed permanent, a spell cast transformed); a copy of that face has 0.
    fn mana_value(&self, id: ObjectId) -> u32 {
        mana_value_of(&|card| self.card(card), self.state(), id, &self.chars(id))
    }

    /// The characteristics a face-down permanent would have if it were face up (CR 702.37e, 702.168d: "if the permanent wouldn't
    /// have a morph cost if it were face up"): its normal copiable values - its card's, or the copy effects' on it (CR 708.8,
    /// 708.10) - with every effect that applies to it now, which still apply once it is turned up (CR 708.8), worked out by the
    /// layers as for any object: "loses all abilities" (Humility, Dress Down, Turn to Frog) takes its morph, an effect that
    /// grants an ability gives it. Judged by naming it on `GameState::judged_face_up` for the length of the question, which
    /// keeps the layers' pass for it apart; any other object's are its characteristics.
    fn chars_if_face_up(&self, id: ObjectId) -> Chars {
        let state = self.state();
        let obj = state.obj(id);
        if obj.face_down_values().is_none() || !matches!(obj.zone.kind(), ZoneKind::Battlefield | ZoneKind::PhasedOut) {
            return self.chars(id);
        }
        let was = state.judged_face_up.replace(Some(id));
        let chars = self.chars(id);
        state.judged_face_up.set(was);
        chars
    }

    /// The morph (megamorph's included) or disguise way a face-down permanent would have if it were face up, `by` naming which
    /// (CR 702.37e, 702.168d: "show all players what the permanent's morph cost would be if it were face up"), its `turn_up`
    /// the cost: the way of its normal copiable values - the card's own, or the copy effects' on it (CR 708.8, 708.10) - where
    /// it would have that keyword face up (`chars_if_face_up`: an effect that took its abilities leaves it none).
    fn turn_up_way(&self, id: ObjectId, by: TurnUp) -> Option<&'static WayDef> {
        let obj = self.obj(id);
        let face_up = obj.copy.as_ref().filter(|c| !obj.own_faces(c)).map_or(obj.card, |c| c.card);
        let way = self.card(face_up).ways.iter().find(|w| by.is(w.kind))?;
        let keyword = if by == TurnUp::Disguise { Keyword::Disguise } else { Keyword::Morph };
        self.chars_if_face_up(id).has(keyword).then_some(way)
    }

    /// An object's copiable values (CR 707.2): what a copy of it copies, the exceptions of its own copy effect
    /// included, and the face that is up for a double-faced permanent (CR 707.8). Those of a permanent or card represented
    /// by a double-faced card or token name its front face and whether its back face is up (`Copiable::faces`): a token copy
    /// of it is double-faced too, each face as modified by the copy effects on it (CR 707.8a) - Glasspool Mimic's copy
    /// effect over both of its faces. A Clone, a token made of one face, a face-down permanent are no double-faced object.
    fn copiable_values(&self, id: ObjectId) -> Copiable {
        let obj = self.obj(id);
        if obj.marks.face_down_as.is_some() {
            return obj.copiable().unwrap_or_else(|| Copiable::of(self.chars(id).card));
        }
        let front = obj.represented_double_faced(self.card(obj.card).back.is_some()).then_some(obj.card);
        let values = match obj.copiable() {
            // A double-faced token's own faces: the face up, under its exceptions.
            Some(c) if obj.own_faces(&c) => Copiable { card: self.chars(id).card, ..c },
            Some(c) => c,
            None => Copiable::of(self.chars(id).card),
        };
        values.from_faces(front, obj.transformed)
    }

    /// The cards `player` owns outside the game (CR 400.11, 108.3b) - in a match, their sideboard (CR 100.4, 400.11a) - in
    /// the order the sideboard lists them: what "a card you own from outside the game" chooses among. Nothing else in the
    /// game sees them (CR 400.11c).
    ///
    /// While another player controls `player` (CR 723), there is nothing: the controller can't see those cards (723.4), so
    /// they can't have the player choose any of them (Emrakul, the Promised End's ruling).
    fn outside_the_game(&self, player: PlayerId) -> Vec<ObjectId> {
        if self.state().player_controller(player) != player {
            return Vec::new();
        }
        self.state().zone(Zone::sideboard(player)).to_vec()
    }

    /// The permanents a player controls, in battlefield order.
    fn permanents_of(&self, player: PlayerId) -> Vec<ObjectId> {
        self.state().battlefield().iter().copied().filter(|&id| self.obj(id).controller == player).collect()
    }

    /// The targets already chosen for the earlier slots of the spell or ability whose target is being chosen or
    /// checked (CR 601.2c): "up to one target creature they control", where "they" is the player an earlier slot
    /// chose. Empty outside a target filter.
    fn chosen_targets(&self) -> &[Target] {
        &[]
    }

    /// The opponent the target slot being chosen or checked is for, when it is one of a `TargetSpec::EACH_OPPONENT`
    /// ("for each opponent, up to one target creature that player controls", CR 601.2c). `None` outside such a slot.
    /// Whether the targets are being judged as if the gift of the spell they are for were promised (CR 702.174m): asked of a
    /// gift spell with no target without it, to see whether the promise would give it one. `gift::promised` reads it.
    fn gift_assumed(&self) -> bool {
        false
    }

    fn slot_opponent(&self) -> Option<PlayerId> {
        None
    }

    /// The subject of the event a triggered ability triggered on (`Fired::subject`), for the filter of its targets: "target
    /// planeswalker that player controls", where the player is the one the damage was dealt to (CR 603.3d). `Target::None`
    /// outside a trigger's target filter.
    fn event_subject(&self) -> Target {
        Target::None
    }

    /// The X announced for the spell or activated ability whose target is being chosen or checked (CR 107.3a, 601.2b,
    /// 602.2b), for the filter of a target "with mana value X": that spell's or ability's own, as it is announced, as it is
    /// judged again on resolution (CR 608.2b) and as its targets are changed or chosen anew (CR 115.7), never another
    /// activation's of the same source (CR 107.3k). `None` outside a target filter, for a triggered ability, and while it
    /// is only asked whether some announcement could have a legal target, before X is announced: the filter then allows
    /// what some X would.
    fn announced_x(&self) -> Option<u32> {
        None
    }

    /// The cards in exile that were exiled with `by` (CR 607.2a): what an ability linked to the one that exiled them refers
    /// to - an ability of the same object printed in the same text (`Linked`), so cards exiled by an ability of another text
    /// the object has (a copy effect's, a granted one's) are not among them (CR 607.5). A permanent that left and came back
    /// is a new object (CR 400.7) with none; in exile order.
    fn exiled_with(&self, by: Linked) -> Vec<ObjectId> {
        let s = self.state();
        s.seats().flat_map(|p| s.zone(Zone::exile(p)).iter().copied()).filter(|&id| self.obj(id).exiled.is_some_and(|l| l.is_by(by))).collect()
    }

    /// The card a face's abilities are printed on, as linked abilities know it (CR 607.1, 607.1b): `CardTable::linked_card`.
    fn linked_card(&self, card: CardId) -> CardId;

    /// A spell or ability on the stack, resolving or about to (`StackItem`), as linked abilities know it (CR 607.1, 607.5): its
    /// source, the card whose text it is printed in, and how its source acquired it (`StackItem::acquired`). What `exile_with` links and `exiled_with` asks for, from a resolving ability.
    fn linked(&self, item: &StackItem) -> Linked {
        Linked { object: item.object, text: self.linked_card(item.ability.card), acquired: item.acquired }
    }

    /// An ability `source` has from the text it has now (CR 607.1, 607.5): its card's, the face that is up, or a copy effect's
    /// (CR 707.2), acquired with that text (`CardTable::linked_to`) - what a card's own hook of that text links and asks for,
    /// one asked with the object alone: a cost change. A function asked with an `Asker` asks `linked_to` of it; a triggered
    /// ability's matcher that reads a choice or an exile is asked with its controller (`when_asked`) for that asker, never
    /// with this, which an ability an effect gave the object would misread (CR 607.5).
    fn text_linked(&self, source: ObjectId) -> Linked;

    /// The ability `asker` asks as, as linked abilities know it (CR 607.1, 607.5): the one an effect gave its source, which
    /// the engine names (`Asker::by`), or else one of the text the source has (`text_linked`). What a triggered or activated
    /// ability's functions - its trigger event and condition, its costs, targets and colors of mana - and a card's own
    /// hooks asked with an `Asker` link and ask for.
    fn linked_to(&self, asker: &Asker) -> Linked {
        asker.by.unwrap_or_else(|| self.text_linked(asker.source))
    }

    /// `item`, a spell or ability resolving, asking with `controller` as its "you" - its own controller, or the player an
    /// instruction names ("that opponent ...") - as linked abilities know it (`linked`, `Asker::by`): what a filter or a choice
    /// it makes as it resolves is asked with.
    fn item_asker(&self, item: &StackItem, controller: PlayerId) -> Asker {
        let by = self.linked(item);
        Asker { source: item.object, controller, by: (by != self.text_linked(item.object)).then_some(by), spell: item.kind == StackKind::Spell }
    }

    /// The object `left` became when it moved to a public zone, if that object is still where the move put it (CR 400.7e):
    /// "return it to the battlefield" from a "when this dies" ability finds the card in the graveyard it went to, and
    /// nothing once that card has moved again (a new object, CR 400.7). Persist, earthbend, Enduring Innocence and Enduring Curiosity return it.
    fn found_after_move(&self, left: ObjectId) -> Option<ObjectId> {
        self.obj(left).became.filter(|&new| self.obj(new).became.is_none() && !self.obj(new).zone.is(ZoneKind::Gone))
    }

    /// What an Aura or Equipment is attached to, if both are still the permanents they were and on the battlefield: one
    /// that has left is attached to nothing (CR 400.7), though its old row still names what it was on.
    fn attached_to(&self, id: ObjectId) -> Option<ObjectId> {
        let o = self.obj(id);
        if o.became.is_some() || !o.zone.is(ZoneKind::Battlefield) {
            return None;
        }
        let host = o.attached_to?;
        let h = self.obj(host);
        (h.became.is_none() && h.zone.is(ZoneKind::Battlefield)).then_some(host)
    }

    /// Whether a creature was declared as an attacker this turn (CR 508.1): still attacking or not, but still the
    /// object that attacked.
    fn attacked_this_turn(&self, id: ObjectId) -> bool {
        self.obj(id).became.is_none() && self.history().attacked_this_turn(id)
    }

    /// Whether an attacking creature is attacking (CR 506.3): declared as an attacker, or put onto the battlefield
    /// attacking (CR 508.4), and not removed from combat since.
    fn is_attacking(&self, id: ObjectId) -> bool {
        self.state().attackers.iter().any(|a| a.id == id)
    }

    /// Whether a creature that has left the battlefield was attacking as it last was there (CR 506.3, 603.10a), from its
    /// last-known information: combat forgets it the moment it leaves (CR 506.4), so `is_attacking` no longer says.
    fn was_attacking(&self, id: ObjectId) -> bool {
        self.state().last_known(id).is_some_and(|row| row.attacking)
    }

    /// Whether an attacking creature has been blocked (CR 509.1h): it stays blocked when its blockers leave.
    fn is_blocked(&self, id: ObjectId) -> bool {
        self.state().attackers.iter().any(|a| a.id == id && a.blocked)
    }

    /// Whether a permanent is saddled (CR 702.171b): it was this turn. One that has left the battlefield is as it last
    /// was there (CR 608.2h).
    fn is_saddled(&self, id: ObjectId) -> bool {
        let o = self.obj(id);
        let here = o.zone.is(ZoneKind::Battlefield) && o.became.is_none();
        o.marks.saddled_turn == self.state().turn.number && (here || self.state().last_known(id).is_some())
    }

    /// Whether a permanent is solved (CR 719.3b): the designation it was given when it became solved, kept until it leaves
    /// the battlefield. One that has left is as it last was there (CR 608.2h); the new object it became is not solved.
    fn is_solved(&self, id: ObjectId) -> bool {
        self.obj(id).marks.solved
    }

    /// Whether a permanent is suspected (CR 701.60b): the designation lasts until it leaves the battlefield.
    fn is_suspected(&self, id: ObjectId) -> bool {
        self.state().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::Suspected)
    }

    /// Whether `target` could be chosen by `asker` for a spell or ability: hexproof and protection (CR 702.11,
    /// 702.16b). The slot's own filter is the card's; this is the rules'.
    fn can_target(&self, asker: &Asker, target: Target) -> bool;
}

/// Changing the game.
/// Why a plain move together moves objects (`Game::move_together`), for the triggers that read it (`ZoneReason`): only the
/// reasons no verb of its own carries out. Sacrificing, destroying, discarding, drawing and milling each have theirs, which
/// apply what the rules ask of them (who sacrifices and what they control, CR 701.21a; regeneration and indestructible, CR
/// 701.19); a move can't be made for those reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveReason {
    /// "Put ... into", "return ... to": a move no keyword action names.
    Other,
    /// "Exile" a set of objects at once (CR 701.13a).
    Exile,
    /// The cards a search found (CR 701.23), put where it says.
    Search,
}

impl From<MoveReason> for ZoneReason {
    fn from(reason: MoveReason) -> ZoneReason {
        match reason {
            MoveReason::Other => ZoneReason::Other,
            MoveReason::Exile => ZoneReason::Exile,
            MoveReason::Search => ZoneReason::Search,
        }
    }
}

pub trait Game: Query {
    /// Asks `player` a question. The first time, the effect stops here (`Err(Suspend)`); once answered, the effect
    /// runs again from its start and this returns the answer.
    fn ask(&mut self, player: PlayerId, kind: RequestKind) -> Result<Answer, Suspend>;

    /// For an effect written as explicit stages: the stage it is at, and the answer that brought it there.
    fn stage(&self) -> u16;
    fn staged_answer(&self) -> Option<Answer>;
    fn scratch(&self) -> [u32; 4];
    fn set_scratch(&mut self, scratch: [u32; 4]);
    /// Asks, and continues at `next` once answered, without running the stages before it again.
    fn ask_then(&mut self, next: u16, player: PlayerId, kind: RequestKind) -> Suspend;
    /// Whether `player` is still in the game at this point of the resolution: false once they have left (CR 800.4a), a
    /// concession made as they were about to be asked included (`cantrip_mechanics::players::reached`).
    fn still_in_game(&mut self, player: PlayerId) -> bool;

    fn draw(&mut self, player: PlayerId, count: u32);
    fn discard(&mut self, card: ObjectId);
    /// Discards cards at once (CR 701.9): one event for what watches "one or more" discards.
    ///
    /// A card its owner can't be made to discard by this spell or ability stays (CR 101.2), and madness applies
    /// (CR 702.35a). The new objects, in order, of the cards that were discarded.
    fn discard_together(&mut self, cards: &[ObjectId]) -> Vec<ObjectId>;
    /// `discard_together`, where the cards among them in `paying` are discarded to pay a cost that specifies a
    /// characteristic of them (CR 118.12): "discard two cards unless you discard a creature card", "you may discard a land
    /// card instead". Those can't be put into a hidden zone unrevealed (CR 701.9c): no replacement that would is offered
    /// for them (`ZoneChange::pays_characteristic`).
    fn discard_paying(&mut self, cards: &[ObjectId], paying: &[ObjectId]) -> Vec<ObjectId>;
    /// `discard_together` for cards the same instruction revealed ("target opponent reveals their hand. You choose a card
    /// from it. That player discards that card", CR 701.20a): each moves revealed, so a replacement that puts it into a hand
    /// or a library puts it there seen by all, not undefined (CR 603.10a, 701.9c).
    fn discard_revealed(&mut self, cards: &[ObjectId]) -> Vec<ObjectId>;
    /// `player` discards their hand (CR 701.9): every card in it, at once, through `discard_together`. The new objects.
    fn discard_hand(&mut self, player: PlayerId) -> Vec<ObjectId> {
        let hand = self.state().zone(Zone::hand(player)).to_vec();
        self.discard_together(&hand)
    }
    /// Mills `count` cards (CR 701.17): the top cards of the library together. The new objects, in order, of the cards
    /// that were put into a graveyard or, by a replacement effect, into exile instead (CR 701.17c): the cards "from among
    /// them" an effect looks at.
    fn mill(&mut self, player: PlayerId, count: u32) -> Vec<ObjectId>;
    /// Several players mill at once (CR 701.17, 101.4): "each player mills", "any number of target players each mill".
    /// Each pair is a player and how many they mill, worked out before any card moves; every card moves in one
    /// simultaneous zone change, one event, so "whenever one or more cards are milled" triggers once (CR 603.2c). The new
    /// objects, as `mill` returns them, player by player.
    fn mill_together(&mut self, mills: &[(PlayerId, u32)]) -> Vec<ObjectId>;
    /// Deals damage (CR 120.3) from `source` to `target`: marked on a creature, removed as loyalty from a planeswalker,
    /// lost as life by a player, each after prevention and replacement. How much was dealt, which is none when the damage
    /// was prevented or the target is gone.
    fn damage(&mut self, source: ObjectId, target: Target, amount: u16) -> u16;
    /// Deals damage as `damage` does, returning its actual recipient, amount and excess (CR 120.10), or none if no
    /// damage was dealt. Violent Echoes reads the excess after dealing damage; damage triggers such as Toralf's and
    /// Aegar's can read the same rules information from `Event::Damage::recipient_excess`.
    fn damage_result(&mut self, source: ObjectId, target: Target, amount: u16) -> Option<DamageResult>;
    /// Deals damage to several recipients at once (CR 120.2, 603.2c): "N damage to each creature and each opponent" is one
    /// event, so "whenever this deals damage" triggers once. Each entry is a source, a recipient and an amount; an effect
    /// that heals the damage already marked does not heal what the event itself dealt. How much each was dealt, in order.
    fn damage_together(&mut self, damage: &[(ObjectId, Target, u16)]) -> Vec<u16>;
    /// Destroys permanents at once (CR 701.8a, 101.4, 603.10a): a leaves-the-battlefield ability sees all of them leave. An
    /// indestructible one stays (CR 702.12b); a regeneration shield replaces its destruction (CR 701.19a); the rest go to
    /// their owners' graveyards. Something that is no longer the object it was is not destroyed (CR 400.7). The new objects,
    /// in order, of what was destroyed.
    fn destroy_all(&mut self, permanents: &[ObjectId]) -> Vec<ObjectId>;
    /// `destroy_all` for permanents that "can't be regenerated" (CR 701.19c): a regeneration shield does not save them.
    fn destroy_all_no_regen(&mut self, permanents: &[ObjectId]) -> Vec<ObjectId>;
    /// Destroys a permanent (CR 701.8a): the one-object case of `destroy_all`.
    fn destroy(&mut self, permanent: ObjectId) {
        self.destroy_all(&[permanent]);
    }
    /// Destroys a permanent that "can't be regenerated" (CR 701.19c): the one-object case of `destroy_all_no_regen`.
    fn destroy_no_regen(&mut self, permanent: ObjectId) {
        self.destroy_all_no_regen(&[permanent]);
    }
    /// `player` sacrifices permanents (CR 701.21a), together, in the order given. One `player` doesn't control ("a player
    /// can't sacrifice ... a permanent they don't control"), one that can't be sacrificed, or any when an opponent's spell or
    /// ability can't make `player` sacrifice, stays (CR 101.2, `can_sacrifice`). The player is the one the text names: "you"
    /// (the spell's or ability's controller, a delayed ability's included, CR 603.7d-e), "target player", or for "its
    /// controller sacrifices it" that permanent's controller as it resolves. The new objects, in order, of what was
    /// sacrificed: "if you do" asks whether this is empty.
    fn sacrifice(&mut self, player: PlayerId, permanents: &[ObjectId]) -> Vec<ObjectId>;
    /// Several players sacrifice at once (CR 701.21a, 101.4): "each player sacrifices", each pair who sacrifices and what,
    /// each judged as `sacrifice` judges it. The new objects, in order.
    fn sacrifice_each(&mut self, sacrifices: &[(PlayerId, ObjectId)]) -> Vec<ObjectId>;
    /// Whether `player` could sacrifice this permanent now, as `sacrifice` would (CR 701.21a, 101.2): it is on the
    /// battlefield and `player` controls it, nothing says it can't be sacrificed (`Keyword::CantBeSacrificed`,
    /// `EffectKind::CantBeSacrificed`, `CardDef::sacrifice_bans`), and the resolving spell or ability may cause `player` to
    /// sacrifice (`may_cause`). A choice of what to sacrifice asked with `Purpose::Sacrifice` already offers the chooser
    /// only these; an effect that takes what it sacrifices without asking, offers "you may sacrifice it" (CR 608.2d: not
    /// offered when it is impossible), offers a sacrifice as one option among others, or as a cost to pay "unless", asks it
    /// (CR 118.3).
    fn can_sacrifice(&self, player: PlayerId, permanent: ObjectId) -> bool;
    /// Whether `player` could get `kind` counters now (CR 122.1, 101.2): no static ability says they can't (Melira,
    /// Sylvok Outcast's "you can't get poison counters"). A cost of getting counters (ward) is paid only when this holds
    /// (CR 118.3).
    fn can_get_counters(&self, player: PlayerId, kind: PlayerCounter) -> bool;
    /// Returns objects to their owners' hands at once (CR 400.3, 101.4): from the battlefield, a graveyard, exile or
    /// anywhere else, whoever controls them. A token that leaves the battlefield ceases to exist (CR 111.7, 704.5d). Something
    /// that is no longer the object it was is not returned (CR 400.7). The new objects, in order.
    fn return_to_hand(&mut self, objects: &[ObjectId]) -> Vec<ObjectId>;
    /// `return_to_hand` for the instruction that reveals the cards it puts there: "reveal it and put it into your hand",
    /// "reveal the top card ... if it's a land, put it into your hand" (CR 701.20a). The reveal lasts through that
    /// instruction, so each card is one all players can see as it moves (CR 603.10a); it is shown with `reveal` first. A
    /// later instruction that is not about the revealed card (a draw) moves it with `return_to_hand` or its own verb.
    fn return_to_hand_revealed(&mut self, cards: &[ObjectId]) -> Vec<ObjectId>;
    /// Puts cards their owners have outside the game (`Query::outside_the_game`) into their owners' hands, bringing them
    /// into the game (CR 400.11b): each is a new object from then on. One no longer outside the game is not moved. The new
    /// objects, in order.
    fn bring_into_hand(&mut self, cards: &[ObjectId]) -> Vec<ObjectId>;
    /// `bring_into_hand` for the instruction that reveals the cards it brings in ("reveal two cards you own from outside
    /// the game ... put that card into your hand", CR 701.20a): each is a card all players can see as it moves (CR 603.10a).
    fn bring_into_hand_revealed(&mut self, cards: &[ObjectId]) -> Vec<ObjectId>;
    /// Puts objects into their owners' libraries (CR 401.4, 400.3) at once, in `place`. The new objects, in order, of those
    /// that arrived in a library: a replacement may have sent one elsewhere, and it is not arranged.
    fn put_into_library(&mut self, objects: &[ObjectId], place: LibraryPlace) -> Vec<ObjectId>;
    /// Exiles objects at once (CR 406.2, 101.4), each to its owner's exile zone. Something that is no longer the object it
    /// was is not exiled (CR 400.7). The new objects, in order, of what was exiled.
    fn exile_all(&mut self, objects: &[ObjectId]) -> Vec<ObjectId>;
    /// Puts objects into their owners' graveyards at once (CR 400.3, 101.4), in the order given unless their owner arranges
    /// them (CR 404.3, asked by the core as they arrive when the order can matter), from wherever they are but
    /// the battlefield (a permanent among them is not moved): "put the rest into your graveyard" is no discard, no mill and no
    /// destruction. Something that is no longer the object it was is not moved (CR 400.7). The new objects, in order. Malevolent Rumble and Satyr Wayfinder
    /// put what they did not take there.
    fn put_into_graveyard(&mut self, objects: &[ObjectId]) -> Vec<ObjectId>;
    /// Moves an object to a zone (top of a library), returning the new object it becomes. Not for the battlefield:
    /// entering it is `put_onto_battlefield`, which applies entering replacements.
    fn move_to(&mut self, object: ObjectId, to: Zone) -> ObjectId;
    /// Moves several objects at once (CR 603.10a: a leaves-the-battlefield ability sees all of them leave), for a
    /// reason triggers can read (`MoveReason`: none that a verb of its own carries out, so a sacrifice is `sacrifice` or
    /// `sacrifice_each`, which name who sacrifices, CR 701.21a). The new objects, in order.
    fn move_together(&mut self, moves: &[(ObjectId, Zone)], reason: MoveReason) -> Vec<ObjectId>;
    /// `move_together` for the instruction that reveals the cards it moves ("reveal it, put it into your hand", "put the
    /// cards revealed this way into your hand and the rest into your graveyard", CR 701.20a): each is a card all players can
    /// see as it moves (CR 603.10a), as `return_to_hand_revealed` says.
    fn move_together_revealed(&mut self, moves: &[(ObjectId, Zone)], reason: MoveReason) -> Vec<ObjectId>;
    /// Puts an object onto the battlefield under `controller` (CR 110.2a), its entering replacements applied.
    fn put_onto_battlefield(&mut self, object: ObjectId, controller: PlayerId, tapped: bool) -> Result<ObjectId, Suspend>;
    /// `put_onto_battlefield`, the object entering with these counters as well (CR 122.6, 614.1c): persist's and undying's
    /// "return it with a counter on it". They are part of how it enters, so a replacement of counters applies (CR 614.1).
    fn put_onto_battlefield_with_counters(&mut self, object: ObjectId, controller: PlayerId, tapped: bool, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend>;
    /// Puts an object onto the battlefield tapped and attacking `defender` (CR 508.4, 702.49a): attacking, but never
    /// declared as an attacker, so nothing that watches attackers being declared sees it. Outside combat it is just
    /// tapped. The object enters with `counters` as well (CR 122.6, 614.1c: Grim Reaper's finality counter), none for most.
    /// Returns the new object.
    fn put_onto_battlefield_attacking(&mut self, object: ObjectId, controller: PlayerId, defender: Target, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend>;
    /// Puts an object onto the battlefield and attaches an Aura already on the battlefield to it, which now
    /// enchants "the creature put onto the battlefield with this Aura" (Animate Dead, Necromancy). Returns the new
    /// object; nothing is attached if the Aura has left or the object did not arrive.
    fn put_onto_battlefield_with_aura(&mut self, object: ObjectId, controller: PlayerId, aura: ObjectId) -> Result<ObjectId, Suspend>;
    /// Puts an object onto the battlefield under `controller` as the effect of `item` says it is (`EntersAs`, CR 614.12):
    /// it enters as that - an Enduring creature's card "is an enchantment" and never enters as a creature, so nothing that
    /// watches creatures enter sees it - and stays so for as long as it is that object (CR 611.2a, 400.7). As an Aura
    /// (`EntersAs::aura`) its controller chooses what it enchants as it enters, a legal object its enchant ability allows,
    /// without targeting (CR 303.4f); with none it stays where it is (CR 303.4g), and it is put into its owner's graveyard
    /// once what it enchants is no longer legal (CR 303.4c, 704.5m). The permanent, or None if it did not enter. Old-Growth
    /// Troll, Harold and Bob, First Numens and Bronzehide Lion return as Auras; Enduring Innocence, Curiosity and Courage
    /// as enchantments.
    fn put_onto_battlefield_as(&mut self, object: ObjectId, controller: PlayerId, item: &StackItem, what: &'static EntersAs) -> Result<Option<ObjectId>, Suspend>;
    /// Puts several objects onto the battlefield at once (CR 110.2a), each under the player given.
    fn put_all_onto_battlefield(&mut self, objects: &[(ObjectId, PlayerId)]) -> Result<Vec<ObjectId>, Suspend>;
    /// The same, each with whether it enters tapped: Winds of Abandon puts the basic lands it found onto the battlefield
    /// tapped, together.
    fn put_all_onto_battlefield_as(&mut self, objects: &[(ObjectId, PlayerId, bool)]) -> Result<Vec<ObjectId>, Suspend>;
    /// `put_all_onto_battlefield_as`, each entering with these counters as well (CR 122.6, 614.1c): Ghost Vacuum puts each
    /// creature card exiled with it onto the battlefield together, each with a flying counter.
    fn put_all_onto_battlefield_with_counters(&mut self, objects: &[(ObjectId, PlayerId, bool)], counters: &[(CounterKind, i16)]) -> Result<Vec<ObjectId>, Suspend>;
    /// `player` sacrifices permanents as cards are put onto the battlefield at the same moment (CR 101.4, 701.21a): Goblin
    /// Welder. What `player` can't sacrifice (`can_sacrifice`) stays.
    fn sacrifice_and_return(&mut self, player: PlayerId, sacrificed: &[ObjectId], returned: &[(ObjectId, PlayerId)]) -> Result<(), Suspend>;
    /// Puts an object onto the battlefield transformed, its back face up (CR 712.14a): "exile it, then return it to
    /// the battlefield transformed", or "tapped and transformed" with `tapped` (CR 110.5b). A card with no back face stays
    /// where it is (CR 712.14a).
    fn put_onto_battlefield_transformed(&mut self, object: ObjectId, controller: PlayerId, tapped: bool) -> Result<ObjectId, Suspend>;
    /// `put_onto_battlefield_transformed`, the object entering with these counters as well (CR 122.6, 614.1c): Esper Origins'
    /// "put it onto the battlefield transformed ... with a finality counter on it". They are part of how it enters, put by its
    /// controller (CR 122.6a), so a replacement of counters applies to them (CR 614.1). A card with no back face stays where it
    /// is (CR 712.14a).
    fn put_onto_battlefield_transformed_with_counters(&mut self, object: ObjectId, controller: PlayerId, tapped: bool, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend>;
    /// Transforms a permanent (CR 701.27a): the other face is up. It stays the same object (CR 712.18). Nothing if it is not
    /// a double-faced permanent on the battlefield (CR 701.27c). The face it turns over to applies its "as this transforms"
    /// ability as it does (CR 712.20, `CardDef::as_transforms`), which may ask a question.
    fn transform(&mut self, permanent: ObjectId) -> Result<(), Suspend>;
    /// `transform` as the instruction of the spell or ability `item`, named explicitly: an ability of the permanent does
    /// nothing if the permanent has transformed since the ability was put on the stack, or since a delayed ability was
    /// created (CR 701.27f, `StackItem::since`).
    fn transform_by(&mut self, permanent: ObjectId, item: &StackItem) -> Result<(), Suspend>;
    /// Creates `count` tokens that are copies of a permanent (CR 707.2, 111.1), all at once, under `controller`.
    fn create_token_copies(&mut self, of: ObjectId, controller: PlayerId, count: u32) -> Result<Vec<ObjectId>, Suspend>;
    /// Creates `count` tokens that are copies of these copiable values (CR 707.2, 111.1), the exceptions the effect
    /// states (CR 707.9b) among them: eternalize's "except it's a black Zombie 4/4 with no mana cost".
    fn create_token_copies_of(&mut self, values: Copiable, controller: PlayerId, count: u32) -> Result<Vec<ObjectId>, Suspend>;
    /// The player gets an emblem (CR 114.2): a new object in the command zone, owned by them, with the abilities
    /// of this definition. It never leaves.
    fn create_emblem(&mut self, def: CardId, player: PlayerId) -> ObjectId;
    /// Creates `count` tokens of a definition (CR 111.2), under `controller`.
    /// Creates tokens of several definitions as one instruction (CR 111.1, 101.4): they enter together, so a "one or
    /// more" ability sees one event. Each is a definition and who controls it.
    fn create_tokens_together(&mut self, tokens: &[(CardId, PlayerId)]) -> Result<Vec<ObjectId>, Suspend>;
    fn create_tokens(&mut self, def: CardId, controller: PlayerId, count: u32, tapped: bool) -> Result<Vec<ObjectId>, Suspend>;
    /// `create_tokens`, each token entering with these counters (CR 122.6, 111.1): "create a token with two +1/+1
    /// counters on it", its text having `words` as `create_tokens_worded`'s does (`Words::NONE` for a token a rule defines:
    /// incubate's Incubator, CR 701.53a).
    fn create_tokens_with_counters(&mut self, def: CardId, controller: PlayerId, count: u32, tapped: bool, counters: &[(CounterKind, i16)], words: Words) -> Result<Vec<ObjectId>, Suspend>;
    /// Creates a delayed triggered ability (CR 603.7), controlled by `controller`, about `subject`.
    fn delayed_trigger(&mut self, when: DelayedWhen, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target);
    /// `delayed_trigger`, carrying the objects the ability is about to the moment it triggers (`event_objects` of its
    /// stack item): every Warrior a mobilize ability created, which it sacrifices together (CR 603.7c).
    fn delayed_trigger_about(&mut self, when: DelayedWhen, source: ObjectId, controller: PlayerId, ability: AbilityRef, objects: &[ObjectId]);
    /// Copies a spell on the stack (CR 707.10) for `copier`, not cast; with `new_targets` the copier may choose new
    /// targets (CR 707.10c). The copy, or nothing if the spell is not on the stack.
    fn copy_spell(&mut self, spell: ObjectId, copier: PlayerId, new_targets: bool) -> Result<Option<ObjectId>, Suspend>;
    /// `copy_spell` with exceptions (CR 707.9, 707.9b): the copy has `values` as its copiable values, the spell's own
    /// (`Query::copiable_values`) with the exceptions applied - "the copy isn't legendary and has starting loyalty X" is
    /// `g.copiable_values(spell).nonlegendary().with_loyalty(x)`. A copy of a permanent spell becomes a token with them as it
    /// resolves (CR 707.10f).
    fn copy_spell_as(&mut self, spell: ObjectId, copier: PlayerId, new_targets: bool, values: Copiable) -> Result<Option<ObjectId>, Suspend>;
    /// Copies the activated or triggered ability on the stack that `ability` names (`Target::Ability`) for `copier` (CR
    /// 707.10), with the original's source; an activated one that has left the stack is copied as it last was there. With
    /// `new_targets` the copier may choose new targets (CR 707.10c). The copy's serial, or nothing.
    fn copy_ability(&mut self, ability: Target, copier: PlayerId, new_targets: bool) -> Result<Option<u32>, Suspend>;
    /// A reflexive triggered ability (CR 603.12): "when you do, ...", triggered as the effect that made it resolves and
    /// put on the stack, targets chosen, the next time a player would receive priority. `ability` is one of the
    /// source card's `delayed` abilities.
    fn reflexive_trigger(&mut self, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target);
    /// `reflexive_trigger` for an ability that counts what the first part did (CR 603.12): "When you discard one or
    /// more nonland cards this way, tap up to that many target creatures". `count` is that many: a
    /// `TargetSpec::COUNTED` bound of the ability's target slots reads it as the ability is put on the stack, and its
    /// stack item's `x` is it, for an effect that does "that many" of something (and for a copy of the ability).
    fn reflexive_trigger_counted(&mut self, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target, count: u16);
    /// It becomes day (CR 731.1): every daybound or nightbound permanent whose face disagrees is turned over. Nothing if
    /// it already is day; a game that has had a designation always has one. A permanent turned over applies its face's "as
    /// this transforms" ability (CR 712.20), which may ask a question.
    fn become_day(&mut self) -> Result<(), Suspend>;
    fn become_night(&mut self) -> Result<(), Suspend>;
    /// Records that the controller did what the ability lets them do only once each turn (CR 603.2h), for
    /// `History::done`.
    fn note_done(&mut self, source: ObjectId, ability: AbilityRef);
    /// Marks a permanent to be exiled instead if it would leave the battlefield (CR 702.84a).
    fn exile_on_leave(&mut self, permanent: ObjectId);
    fn shuffle(&mut self, player: PlayerId);
    /// A number below `bound`, from the game's own random source ("at random"): journalled, so the same
    /// game and answers give the same result.
    fn random(&mut self, bound: u32) -> u32;
    /// `player` rolls a die with this many sides (CR 706.2) and the roll is recorded as `Event::DieRolled`, which
    /// triggers "whenever you roll a die". The natural result is the face, 1 to `sides`; it is the result too.
    fn roll_die(&mut self, player: PlayerId, sides: u32) -> Roll {
        self.roll_die_modified(player, sides, 0)
    }
    /// A roll whose result is modified (CR 706.4): `modifier` added to the natural result.
    fn roll_die_modified(&mut self, player: PlayerId, sides: u32, modifier: i32) -> Roll;
    /// One of `count` things is chosen at random: its index, below `count`; recorded as
    /// `Event::ChosenAtRandom`. Nothing is drawn when there is no choice (`count` of 0 or 1 gives 0).
    fn choose_at_random(&mut self, count: usize) -> usize;
    /// `player` flips a coin (CR 705.1): true if they win the flip (CR 705.2). A flip is a two-sided die whose first face
    /// wins; it is recorded as `Event::CoinFlipped` and is not a die roll.
    fn flip_coin(&mut self, player: PlayerId) -> bool;
    /// `player` flips a coin that cares only whether it comes up heads (CR 705.2): true for heads. No one wins or loses
    /// it, so "whenever you win a coin flip" does not trigger, and "whenever you flip a coin" does. Its face is the same
    /// two-sided die's (the first face is heads), recorded as `Event::CoinFlipped`.
    fn flip_coin_for_heads(&mut self, player: PlayerId) -> bool;
    /// Moves a card already in a library to `from_top` cards from its top (0 is the top): "second from the top".
    fn place_in_library(&mut self, card: ObjectId, from_top: usize);
    /// "After this phase, there is an additional combat phase" (CR 500.8), from the precombat main phase or a combat phase: a
    /// whole combat phase, from its beginning, follows the combat phase that is under way (or the one that is next).
    fn add_combat_phase(&mut self);
    /// "End the turn" (CR 724.1), from a resolving spell or ability: triggered abilities waiting to be put on the stack
    /// cease to exist (724.1a); every object on the stack is exiled, the resolving one included (724.1b), whose
    /// remaining instructions still happen after this; state-based actions are checked with no player given priority
    /// and no triggered ability put on the stack (724.1c); and once the resolution is over the step under way ends, every
    /// creature and planeswalker is removed from combat, and the game skips straight to the cleanup step (724.1d) - a new
    /// cleanup step when it is ended during one. Abilities that trigger from here on go on the stack in that cleanup step,
    /// and players then get priority, with another cleanup step after it (724.1f, 514.3a).
    fn end_the_turn(&mut self) -> Result<(), Suspend>;

    /// `player` wins the game (CR 104.2b): their opponents lose and the game ends at once. Nothing if they have lost, or
    /// are losing to a state-based action (CR 104.3f).
    /// `permanent` becomes a copy of these copiable values for `duration` (CR 707.2, 611.2b): the same object, as another
    /// card. `Duration::EndOfTurn` for "until end of turn" (Shifting Woodland, Saheeli); `Duration::Indefinite` for an
    /// effect that states no duration, which lasts as long as the permanent stays (CR 611.2a-b: Likeness Looter). `source`
    /// is the spell or ability doing it.
    fn become_copy(&mut self, source: ObjectId, permanent: ObjectId, values: Copiable, duration: Duration);
    fn win_game(&mut self, player: PlayerId);
    /// An effect states that `player` loses the game (CR 104.3e): they are out at once, and the last one left has won.
    fn lose_game(&mut self, player: PlayerId);
    /// `player` gets poison counters (CR 122.1f); ten lose the game (CR 704.5c).
    fn give_poison(&mut self, player: PlayerId, amount: u32);
    /// `player` gets experience counters (CR 122.1).
    fn give_experience(&mut self, player: PlayerId, amount: u32);
    /// `count` extra turns for `player` after the current one (CR 500.7); the most recently created is taken first.
    fn add_extra_turns(&mut self, player: PlayerId, count: u32);
    /// One extra turn for `player` after the current one (CR 500.7), and the number naming it, for an ability that says
    /// "that turn" (`DelayedWhen::ExtraTurnStep`); none for a player who has left the game (CR 800.4a).
    fn add_extra_turn(&mut self, player: PlayerId) -> Option<u32>;
    /// "[player] skips their next `count` turns" (CR 500.11, 614.10), from `source`: a replacement effect for each turn,
    /// applied as each of that player's turns would begin and used up by it; a turn already begun is not skipped, and
    /// two such effects skip a turn each (CR 614.10a). Nothing for a count of zero. The effect is the player's, not the
    /// source's, and outlives it.
    fn skip_next_turns(&mut self, source: ObjectId, player: PlayerId, count: u32) {
        let controller = self.obj(source).controller;
        // "Skips their next X turns": one instruction (CR 608.2c).
        let _ = self.one_instruction(&mut |g| {
            for _ in 0..count {
                g.add_player_effect(controller, source, EffectKind::SkipNextTurn(player), Duration::Indefinite);
            }
            Ok(())
        });
    }
    /// "You control target player during that player's next turn" (CR 723.1), made by the resolving spell or ability `item`:
    /// its controller makes every choice and decision `player` would make during the next turn `player` actually takes
    /// (723.5; a skipped turn is not it, 723.1b), paying with `player`'s resources (723.5a) and seeing what `player` sees of
    /// the game (723.4). The last such effect created on a player is the one that works (723.1a); one on themselves gives
    /// no one else control (723.9). It waits on the player, not on `item`'s source. Mindslaver, Sorin Markov, Worst Fears.
    fn control_next_turn(&mut self, item: &StackItem, player: PlayerId) {
        let kind = EffectKind::ControlNextTurn { player, then_extra_turn: false };
        self.add_player_effect(item.controller, item.object, kind, Duration::Indefinite);
    }
    /// The same, and "After that turn, that player takes an extra turn" (Emrakul, the Promised End): the extra turn comes
    /// right after the controlled turn, however long that turn waits (its rulings), and is given even when a later effect
    /// overwrites this one's control (723.1a).
    fn control_next_turn_then_extra_turn(&mut self, item: &StackItem, player: PlayerId) {
        let kind = EffectKind::ControlNextTurn { player, then_extra_turn: true };
        self.add_player_effect(item.controller, item.object, kind, Duration::Indefinite);
    }
    /// "[Kind of] spells [player] casts this turn cost ... less / more to cast" (CR 611.2a/c, 601.2f), made by the resolving
    /// spell or ability `item`: the `index`th of the `lasting_cost_changes` of its card (the face that is up, for a permanent's
    /// ability) changes the total cost of every spell `player` casts until the turn ends (CR 514.2), whether or not the source
    /// stays on the battlefield. Armor Wars, Urza, Planeswalker, Ballad of the Black Flag, Spellbinding Soprano and Academy
    /// Loremaster print it. The effect is controlled by `item`'s controller (CR 109.5).
    fn spells_cost_this_turn(&mut self, item: &StackItem, player: PlayerId, index: u8) {
        let kind = EffectKind::SpellCostChange { card: item.ability.card, index, player };
        self.add_player_effect(item.controller, item.object, kind, Duration::EndOfTurn);
    }
    /// "You may play [`count`] additional land[s] this turn" (CR 305.2, 611.2c), made by the resolving spell or ability
    /// `item`: a permission for its controller (CR 109.5), asked nothing as it resolves, that adds `count` to the lands they
    /// can play until the turn ends (CR 514.2), whatever happens to the source, and adds to every other effect and static
    /// ability that does the same. Lands already played this turn count against the new total (CR 305.2a), and it lets
    /// nothing be played on another player's turn (CR 305.3). Nothing for a count of zero. Explore, Urban Evolution, Scale
    /// the Heights, Kiora, the Crashing Wave's −1, Way of the Paradox, Sword of Forge and Frontier; Journey of Discovery's
    /// "up to two" is 2 and Nahiri's Lithoforming's "X additional lands" X.
    fn additional_land_plays(&mut self, item: &StackItem, count: u32) {
        if count > 0 {
            self.add_player_effect(item.controller, item.object, EffectKind::ExtraLandPlays(count), Duration::EndOfTurn);
        }
    }
    /// A permanent with a prepare spell becomes prepared (CR 722.3a/c): a copy of the spell is put in exile, which its
    /// controller may cast while it stays prepared. Nothing if it already is, or is not on the battlefield.
    fn become_prepared(&mut self, permanent: ObjectId);
    /// A permanent becomes unprepared (CR 722.3b): removes its prepared designation, ending its prepare copy's
    /// permission and exemption from ceasing to exist (CR 722.3c, 704.5e). Nothing if it is no longer on the battlefield.
    fn become_unprepared(&mut self, permanent: ObjectId);
    /// A Class becomes a level (CR 716.2a), announced to what watches it. Nothing if it is not on the battlefield.
    fn set_class_level(&mut self, permanent: ObjectId, level: u8);
    /// Puts cards of a player's library on its top in this order, the first on top (CR 401.4), or on its bottom,
    /// the first highest and the last lowest. The cards are already in the library; nothing changes zones.
    fn arrange_library(&mut self, player: PlayerId, top: &[ObjectId], bottom: &[ObjectId]);
    /// Puts cards already in a player's library on its bottom in a random order (CR 401.4), drawn from the game's own
    /// random source: "put the rest on the bottom of your library in a random order". Nothing changes zones.
    fn bottom_in_random_order(&mut self, player: PlayerId, cards: &[ObjectId]);
    /// Whether the spell or ability resolving may cause `searcher` to search the library of `library` (CR 701.23)
    /// now: false when a rule forbids it (CR 101.2: "can't" wins, `CardDef.cause_rules` with `CauseKind::Search`, as Ashiok,
    /// Dream Render's "can't cause their controller to search their library"). A search that can't be made finds nothing
    /// and records no search, but a "then shuffle" still shuffles; a "you may search" is not offered (Ashiok's rulings).
    /// The library recipes ask it (`cantrip_mechanics::search_library`, `library::ask_may_search`): a card that picks
    /// cards out of a library some other way asks it itself (Doomsday).
    fn may_search(&self, searcher: PlayerId, library: PlayerId) -> bool;
    /// Records that `player` searched the library of `library` (CR 701.23: a player may search another player's
    /// library, and "searches their library" is only the case where the two are the same), for whatever
    /// watches searches.
    fn searched(&mut self, player: PlayerId, library: PlayerId);
    /// Records that `player` scried `count`, having looked at `looked` cards (CR 701.22a, 701.22d), for whatever watches
    /// scries: `cantrip_mechanics::scry` calls it once the scry is complete, and never for scry 0 (CR 701.22b).
    fn scried(&mut self, player: PlayerId, count: u32, looked: u32);
    /// Records that `player` surveilled `count`, having looked at `looked` cards (CR 701.25a, 701.25d):
    /// `cantrip_mechanics::surveil` calls it once the surveil is complete, and never for surveil 0 (CR 701.25c).
    fn surveilled(&mut self, player: PlayerId, count: u32, looked: u32);
    /// `player` gave a gift to `to` (CR 702.174c): the gift triggered ability of a permanent they control resolved. (A
    /// spell's gift is announced by the core as the spell resolves.)
    fn gave_gift(&mut self, player: PlayerId, to: PlayerId, gift: Gift);
    /// Attaches an Aura or Equipment to a permanent (CR 701.3a): nothing happens if it could not be attached there
    /// (CR 701.3b) or already is. It becomes a new object with respect to what it is attached to (CR 701.3c, 613.7e).
    /// Returns whether it became attached (`Event::Attached`): "when one or more Equipment become attached this way".
    fn attach(&mut self, attachment: ObjectId, host: ObjectId) -> bool;
    /// Unattaches an Aura or Equipment (CR 701.3d); it stays on the battlefield.
    fn unattach(&mut self, attachment: ObjectId);
    /// Taps a permanent (CR 701.26a): nothing happens to one already tapped, gone (CR 400.7) or phased out (CR 702.26b).
    fn tap(&mut self, permanent: ObjectId);
    /// Untaps a permanent (CR 701.26a). One with a stun counter on it has one removed instead (CR 122.1d).
    fn untap(&mut self, permanent: ObjectId);
    /// Exerts a permanent (CR 701.43a): it won't untap during its controller's next untap step.
    fn exert(&mut self, permanent: ObjectId);
    /// Monstrosity N (CR 701.37a): if the permanent is not monstrous, it gets N +1/+1 counters and becomes monstrous,
    /// which abilities that watch for it see.
    fn monstrosity(&mut self, permanent: ObjectId, counters: i16);
    /// A creature becomes saddled until end of turn (CR 702.171b).
    fn become_saddled(&mut self, permanent: ObjectId);
    /// A permanent becomes solved (CR 719.3a-b): one on the battlefield that is not solved is given the designation, and
    /// what watches for it sees `Event::Solved`. One that has left the battlefield, or is solved already, is unchanged.
    fn solve(&mut self, permanent: ObjectId);
    /// Suspect a creature (CR 701.60a): a permanent on the battlefield that is not suspected becomes suspected (CR
    /// 701.60d) until it leaves the battlefield, having menace and "This creature can't block" (CR 701.60c).
    fn suspect(&mut self, permanent: ObjectId);
    /// Two creatures fight (CR 701.14a): each deals damage equal to its power to the other, at once; one that fights
    /// itself deals twice its power to itself (CR 701.14c). Nothing happens unless both are creatures on the battlefield
    /// (CR 701.14b).
    fn fight(&mut self, first: ObjectId, second: ObjectId);
    /// Creates tokens that enter attacking `defender` (CR 508.4), tapped when `tapped` ("tapped and attacking"; otherwise they enter untapped): attacking, but never declared as
    /// attackers, so nothing that watches attackers being declared sees them. They are put onto the battlefield
    /// together, as one instruction. Outside combat they are just tokens, tapped or not as `tapped` says.
    fn create_tokens_attacking(&mut self, tokens: &[(CardId, PlayerId)], tapped: bool, defender: Target) -> Result<Vec<ObjectId>, Suspend>;
    /// Creates tokens together whose text has `words` (CR 612.4), tapped when `tapped`, and attacking `attacking` if one is
    /// given (CR 508.4: tapped only if the effect says "tapped and attacking"). Every other
    /// way of creating tokens gives them the words of the text making them, so a text change reaches the colors and
    /// creature types it names (CR 612.2a); a keyword's own token and a predefined one are the rules' (CR 111.10;
    /// reminder text is no rules text, CR 207.2a), so For Mirrodin!'s Rebel is made with `Words::NONE`, and so is every
    /// token `cantrip_mechanics::tokens::create_by_rule` makes.
    fn create_tokens_worded(&mut self, tokens: &[(CardId, PlayerId)], tapped: bool, attacking: Option<Target>, words: Words) -> Result<Vec<ObjectId>, Suspend>;
    /// Creates a token "attached to" `host` (CR 111.1, 303.4): an Aura token enters attached to it, chosen by the effect
    /// and not targeted, so hexproof and shroud do not matter (the Role rulings, 2023-09-01). If the Aura could not legally
    /// enchant `host` - gone from the battlefield, or protected from it - it is not created (CR 303.4i); an Equipment token
    /// that could not equip it is created unattached (CR 301.5e), and any other token enters unattached (CR 303.4h). Every token a replacement makes of it
    /// (CR 614.1a) enters so. The tokens created.
    fn create_token_attached(&mut self, def: CardId, controller: PlayerId, host: ObjectId) -> Result<Vec<ObjectId>, Suspend>;
    /// Exiles cards together and remembers them as exiled with `by`, the ability exiling them (`Query::linked`; CR 607.2a,
    /// 610.3). With `until_leaves` they return to where they were exiled from the moment `by`'s object is no longer on the
    /// battlefield (CR 610.3), no stack and nothing
    /// to respond to; and if `by` has already left, nothing is exiled at all (CR 610.3b). The new objects.
    fn exile_with(&mut self, cards: &[ObjectId], by: Linked, until_leaves: bool) -> Vec<ObjectId>;
    /// Cards already in exile become linked to `by` as though exiled by it (CR 607.2a), for `exiled_with(by)` to find: the
    /// exiled cards used to craft a permanent (CR 702.167c), linked to the craft card in exile that is about to return as it.
    /// Nothing returns them (CR 610.3 does not apply). A card no longer in exile, or no longer the object it was, is left
    /// alone.
    fn link_exiled(&mut self, cards: &[ObjectId], by: Linked);
    /// Exiles a spell card encoded on a creature (CR 702.99a): linked to it as `exile_with` links, and marked encoded
    /// (`ExileLink::encoded`), which only cipher's encoding is. The card in exile, if it got there.
    fn encode(&mut self, card: ObjectId, creature: ObjectId) -> Option<ObjectId>;
    /// Exiles cards together face down (CR 406.3), linked to `by` if an ability exiled them (CR 607.2a). Each is hidden
    /// from every player who could not see where it came from; `look_at` shows it to one more. The new objects.
    fn exile_face_down(&mut self, cards: &[ObjectId], by: Option<Linked>) -> Vec<ObjectId>;
    /// `player` looks at face-down cards in exile (CR 406.3), and may from now on.
    fn look_at(&mut self, player: PlayerId, cards: &[ObjectId]);
    /// Puts an object onto the battlefield face down under `controller`'s control (CR 708.3), tapped if `tapped`: what it is
    /// face down is `how` it was put there (`FaceDown`: manifested, CR 701.40a; cloaked, CR 701.58a), and its controller
    /// may turn it face up for its mana cost if it is a creature card (CR 701.40b, 701.58b). The permanent, or None if it
    /// did not enter face down: a card that changed zones since it was chosen (CR 400.7), or one a rule or effect kept off
    /// the battlefield or sent elsewhere, which keeps its own characteristics (CR 701.40f, 701.58f).
    fn put_onto_battlefield_face_down(&mut self, object: ObjectId, controller: PlayerId, how: FaceDown, tapped: bool) -> Result<Option<ObjectId>, Suspend>;
    /// `put_onto_battlefield_face_down` for several objects at once (CR 110.2a): they enter together, so each sees the others
    /// enter (CR 603.6a). For each object, the permanent it became, or None if it did not enter face down.
    fn put_all_onto_battlefield_face_down(&mut self, objects: &[ObjectId], controller: PlayerId, how: FaceDown, tapped: bool) -> Result<Vec<Option<ObjectId>>, Suspend>;
    /// `player` turns a face-down permanent face up (CR 701.40b, 708.2, 708.8): the player taking the special action, or
    /// the controller of the effect that turns it up. Recorded as `Event::TurnedFaceUp`. Nothing if it is not one.
    fn turn_face_up(&mut self, permanent: ObjectId, player: PlayerId);
    /// `turn_face_up` by the special action of morph or disguise (CR 702.37e, 702.168d), with a +1/+1 counter put on it as it
    /// is turned face up where `megamorph` (its megamorph cost was paid to turn it up, CR 702.37b, 708.11): the rules' own
    /// (`intrinsic`), never a card's.
    fn turn_face_up_as(&mut self, permanent: ObjectId, player: PlayerId, megamorph: bool);
    /// "[These permanents] phase out" (CR 702.26b), all at once: each one still on the battlefield, and indirectly what is
    /// attached to it (CR 702.26g). They phase in during the untap step of the player who controlled them (CR 502.1).
    fn phase_out(&mut self, permanents: &[ObjectId]);
    /// "[It] phases out until [source] leaves the battlefield" (CR 610.4): it phases in once `source` has left, not in an
    /// untap step, and is tapped as it does if `tapped` ("tap that creature as it phases in this way"). Nothing if `source`
    /// has already left (CR 610.4b).
    fn phase_out_until_leaves(&mut self, permanent: ObjectId, source: ObjectId, tapped: bool);
    /// `player` unlocks a door of a Room (CR 709.5f): the permanent is given that half's unlocked designation, which
    /// triggers "when you unlock this door" (CR 709.5h). `door` is `DOOR_LEFT` or `DOOR_RIGHT`. Nothing if the permanent is
    /// not a Room on the battlefield or the door is already unlocked.
    fn unlock_door(&mut self, permanent: ObjectId, door: u8, player: PlayerId);
    /// A card name chosen for a permanent as `by`, an ability of it, resolves (`Query::linked` of the resolving ability),
    /// remembered by that permanent (CR 607.2d): "When this land enters, choose a land card name" (Petrified Hamlet), "II —
    /// Choose a card name" (Medomai's Prophecy). Kept with that ability (`Chosen`): the abilities linked to it read it with
    /// `GameState::chosen_card`, as they read a name chosen as it entered (`Entering::chosen_card`, CR 614.12a). It is the
    /// object's alone: a new object the permanent becomes has no chosen name (CR 400.7), and a later choice for the same
    /// object replaces the earlier one. Nothing if the permanent is no longer on the battlefield.
    fn remember_chosen_name(&mut self, by: Linked, name: CardId);
    fn exile(&mut self, object: ObjectId) -> ObjectId {
        let owner = self.obj(object).owner;
        self.move_to(object, Zone::exile(owner))
    }
    /// `player` gains life (CR 119.3): none, if an effect or a static ability says they can't (CR 119.7,
    /// `EffectKind::CantGainLife`, `CardDef::cant_gain_life`).
    fn gain_life(&mut self, player: PlayerId, amount: i32);
    fn lose_life(&mut self, player: PlayerId, amount: i32);
    /// "`player`'s life total becomes `total`" (CR 119.5): they gain or lose the difference, so a player who can't gain life
    /// keeps a total lower than `total` (CR 119.7).
    fn set_life_total(&mut self, player: PlayerId, total: i32);
    /// Each opponent of `player` who is still in the game loses `amount` life (CR 119.3), in turn order. The total they lost.
    fn each_opponent_loses_life(&mut self, player: PlayerId, amount: i32) -> i32;
    fn add_mana(&mut self, player: PlayerId, mana: Mana, amount: u32);
    /// Adds mana that lasts until the end of combat (CR 106.4, 500.5a): not lost as the steps before it end.
    fn add_mana_until_end_of_combat(&mut self, player: PlayerId, mana: Mana, amount: u32);
    /// Adds mana that can only be spent as `restriction` allows (CR 106.6).
    fn add_restricted_mana(&mut self, player: PlayerId, mana: Mana, amount: u32, restriction: ManaRestriction);
    /// Puts `amount` counters of one kind on an object, as the effect being carried out does (CR 609.1, 122.6): put by the
    /// player whose effect it is (CR 608.2, 122.6a) - the controller of the spell or ability resolving, of the replacement
    /// effect applying, or the player doing the action - and through the replacement effects of counters being put
    /// (CR 614.1a). A negative `amount` takes that many off instead (`remove_counters`).
    fn add_counters(&mut self, object: ObjectId, kind: CounterKind, amount: i16);
    /// Puts counters on several objects, or of several kinds on one, as one instruction of the effect being carried out does
    /// (CR 608.2, 101.4, 122.6): one simultaneous event, which "whenever one or more counters are put on one or more ..." sees
    /// once (CR 603.2c). Each entry is `amount` counters of a kind on an object; entries for one object are one placement on
    /// it, its replacements applying once. Each object's counters go through the replacement effects of counters being put on it (CR 614.1a), all of them
    /// before any is put on; then they are put on and announced together. Put by the player `add_counters` names; an object
    /// that has changed zones (CR 400.7) or an amount of none or fewer gets none. "Put a +1/+1 counter on each creature you
    /// control", proliferate's permanents, "two +1/+1 counters and a flying counter on it".
    fn add_counters_together(&mut self, counters: &[(ObjectId, CounterKind, i16)]);
    /// Removes up to `amount` counters of one kind from an object (CR 122.1) and says how many came off: none from an
    /// object that has changed zones (CR 400.7). It answers "if you do" and "for each counter removed this way", as Dawn of a
    /// New Age and Coalition Relic ask.
    fn remove_counters(&mut self, object: ObjectId, kind: CounterKind, amount: i16) -> i16;
    /// "Remove up to `maximum` counters from target creature" as the effect resolves (CR 608.2d, 122.1): `player` (the
    /// effect's controller) chooses which of the counters on `object`, of any kinds and how many of each, from none to
    /// `maximum` in all, asked as `RequestKind::RemoveCounters` with `up_to`, and they come off. `source` is the spell or
    /// ability. Nothing is asked of an object with no counters, or one that has changed zones (CR 400.7). Says how many
    /// came off, for "for each counter removed this way" (Hex Parasite). Heartless Act, Hex Parasite, Price of Betrayal.
    fn remove_counters_up_to(&mut self, player: PlayerId, source: ObjectId, object: ObjectId, maximum: u8) -> Result<u16, Suspend>;
    /// "Remove a counter from a [permanent] you control" as the effect resolves (CR 608.2d, 122.1): `player` chooses one counter
    /// of any kind on any of `permanents` (the ones the effect may take it from), asked as `RequestKind::RemoveCounters`, and it
    /// comes off. Whether one did, for "if you do": none when none of them has a counter or has changed zones (CR 400.7).
    /// Mister Hyde, Monster Within, Sanctuary Warden and Chisei, Heart of Oceans print it.
    fn remove_a_counter_from_among(&mut self, player: PlayerId, source: ObjectId, permanents: &[ObjectId]) -> Result<bool, Suspend>;
    /// A continuous effect from `source` on `affected` begins (CR 611.2). It is controlled by the controller of the spell or
    /// ability whose instruction makes it (CR 611.2a, 113.8) - or of the replacement effect or mana ability being carried
    /// out - whoever controls `source` by then: The One Ring's "you gain protection from everything" is its caster's though
    /// another player has taken the Ring before the trigger resolves (CR 603.7d's reasoning), and a goad stays the goader's
    /// (CR 701.15b).
    fn add_effect(&mut self, source: ObjectId, affected: ObjectId, kind: EffectKind, duration: Duration);
    /// An effect about `controller` and not an object (`EffectKind::outlives_affected`), for the player who controls the
    /// resolving ability or spell (CR 109.5: "you") even if its source has since changed hands or left.
    fn add_player_effect(&mut self, controller: PlayerId, source: ObjectId, kind: EffectKind, duration: Duration);
    /// `player` gains control of a permanent (CR 108.4, 613.1b) for `duration`, by `source`'s spell or ability.
    fn gain_control(&mut self, source: ObjectId, permanent: ObjectId, player: PlayerId, duration: Duration) {
        self.add_effect(source, permanent, EffectKind::Control(player), duration);
    }
    /// The controllers of two permanents exchange control of them (CR 701.12a), with no end. The exchange is all or nothing:
    /// if either is not on the battlefield, or both are controlled by one player (CR 701.12b), nothing changes hands. Whether
    /// it happened.
    fn exchange_control(&mut self, source: ObjectId, first: ObjectId, second: ObjectId) -> bool {
        let on_battlefield = |g: &Self, id: ObjectId| g.obj(id).became.is_none() && g.is_on_battlefield(id);
        if !on_battlefield(self, first) || !on_battlefield(self, second) {
            return false;
        }
        let (a, b) = (self.controller(first), self.controller(second));
        if a == b {
            return false;
        }
        // One exchange (CR 701.12a): nothing is settled between its halves, and the state triggers judge it once it is done,
        // not with one permanent given and the other not yet taken.
        let _ = self.one_instruction(&mut |g| {
            g.settle_once(&mut |g| {
                g.gain_control(source, first, b, Duration::Indefinite);
                g.gain_control(source, second, a, Duration::Indefinite);
            });
            Ok(())
        });
        true
    }
    /// Carries out `instruction` as one instruction (CR 608.2c): what continuous effects say of control and "for as long
    /// as" durations is settled once it has finished, not between its parts (an exchange of control's two halves).
    fn settle_once(&mut self, instruction: &mut dyn FnMut(&mut dyn Game));
    /// Carries out `f`, several calls that are one instruction of a card's text, as one: what changes the game with no event
    /// (an effect beginning, an object becoming a copy) is judged by the state triggers once it is all done (CR 603.8), not
    /// part-way - an "each" or "all" is never judged half-applied (CR 611.2c). Animating a permanent (types, power and
    /// toughness, abilities), "gets +2/+2 and gains flying", and an exchange of control are each one. [`instruction`] is the
    /// same for code that asks nothing.
    ///
    /// A resolution that changes the game with no event more than once says, between each two such changes, either this or
    /// [`Game::end_sentence`]: which calls are one instruction only the card's text knows, and under `CANTRIP_VERIFY` a
    /// resolution that says neither is refused, so the scenarios find a card that has not said it.
    fn one_instruction(&mut self, f: &mut dyn FnMut(&mut dyn Game) -> Result<(), Suspend>) -> Result<(), Suspend>;
    /// An instruction of the card's text ends here - a sentence, or a "then" (CR 608.2c: the instructions are followed in
    /// order): what it changed with no event is judged by the state triggers now (CR 603.8), before the next one. See
    /// [`Game::one_instruction`] for when a card says it.
    fn end_sentence(&mut self);
    /// Counters a spell (CR 701.5): it leaves the stack for its owner's graveyard, or exile if it was flashed back.
    fn counter(&mut self, spell: ObjectId);
    /// Counters a spell and puts it in its owner's hand, on top or bottom of their library, or in exile (CR 701.6a).
    fn counter_to(&mut self, spell: ObjectId, to: CounterTo);
    /// A spell on the stack changes controllers (CR 112.2): it resolves for `player`, and "you" in it means them. Its
    /// targets, modes and X stay as they were chosen.
    fn change_controller(&mut self, spell: ObjectId, player: PlayerId);
    /// `chooser` may choose new targets for a spell on the stack (CR 115.7d): each slot is offered every legal target it
    /// has now, its current one among them; a slot with no other choice is not asked.
    fn choose_new_targets(&mut self, spell: ObjectId, chooser: PlayerId) -> Result<(), Suspend>;
    /// "Change the target of target spell or ability" (CR 115.7a): the spell or ability (a `Target::Object` spell or a
    /// `Target::Ability`) on the stack has each of its targets changed to another legal one, `chooser` choosing; if any has no
    /// other legal target, none changes. A target with one other choice is not asked.
    fn change_target(&mut self, of: Target, chooser: PlayerId) -> Result<(), Suspend>;
    /// "Change a target of target spell or ability to [this]" (CR 115.7b): one target of the spell or ability (a
    /// `Target::Object` spell or a `Target::Ability`) on the stack becomes `to`, if `to` is a legal target for it and the
    /// change leaves its other targets legal; `chooser` chooses which when more than one may change. A target that already
    /// is `to` stays, and nothing changes if no target can, or if `to` has left the battlefield. Spellskite.
    fn change_a_target_to(&mut self, of: Target, to: ObjectId, chooser: PlayerId) -> Result<(), Suspend>;
    /// A spell leaves the stack without being countered (CR 400.3) for its owner's hand, library or exile: Sink into
    /// Stupor, Subtlety. A spell that can't be countered goes too.
    fn remove_spell(&mut self, spell: ObjectId, to: CounterTo);
    /// Counters whatever spell or ability of `object` is on the stack (CR 701.6), at `hint` if it is still there.
    fn counter_item(&mut self, object: ObjectId, hint: usize);
    fn reveal(&mut self, player: PlayerId, cards: &[ObjectId]);
    /// Whether target `slot` of the resolving item is still legal (CR 608.2b).
    fn target_legal(&self, item: &StackItem, slot: usize) -> bool;
    /// A cost paid as the spell or ability resolves (CR 118.12, 608.2d): "[payer] may pay [cost]. If they do, ...", and
    /// "... unless [payer] pays [cost]" (CR 118.12a). `cost` is its mana and its life (`cost("{W}{B}").and(life(2))`), one
    /// payment shown whole: the payer may make mana for it while asked (CR 605.3a), and may pay only once the pool covers
    /// the mana and their life total the life, judged as it is paid (CR 118.3, 119.4) - a payer without the life is not
    /// asked, and one whose mana ability costs them the life they needed (a painland) can only decline. `subject` is what
    /// the payment is about, and `question` says what paying does, in the card's words (`{cost}` and `{subject}` where
    /// the cost and the subject's name go), its verb what the bots read it by. Whether it was paid: "if they do" (CR
    /// 118.12) asks exactly that, not what the paid-for effect did.
    fn may_pay(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: crate::card::Cost, question: &'static Question) -> Result<bool, Suspend>;
    /// "You may pay {X}" (CR 118.12, 608.2d) where the text leaves X to the payer (CR 107.3f): X chosen as it is paid, in one
    /// question that raises and lowers it and makes mana, any X from 0 up. `cost` is the cost with its {X}
    /// (`cost("{X}")`), and `question` what paying does, as for `may_pay`. The X paid, or `None` when the payer declined:
    /// "when you do" asks whether it was paid (CR 603.12), and a reflexive ability that reads X is given it
    /// (`reflexive_trigger_counted`).
    fn may_pay_x(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: crate::card::Cost, question: &'static Question) -> Result<Option<u16>, Suspend>;
    /// `may_pay` for a cost with a part whose size the payer sets as they pay it ("you may discard one or more land cards",
    /// `VariableCost::DiscardOneOrMore`): how many that part took, or `None` when the payer declined - the "that many" of a
    /// reflexive "when you do" (CR 603.12, `reflexive_trigger_counted`).
    fn may_pay_counted(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: crate::card::Cost, question: &'static Question) -> Result<Option<u16>, Suspend>;
    /// "You may pay [mana] up to `most` times" (`ANY_NUMBER_OF_TIMES`: "any number of times") as the effect resolves (CR
    /// 118.12, 608.2d): one payment whose number the payer chooses as they pay it, from 1 up to `most`, in one question that
    /// raises and lowers it and makes mana, and pays `mana` that many times over. How many times it was paid, 0 when the
    /// payer declined: "when you pay this cost one or more times" (CR 603.12a) triggers once for any number above 0, and
    /// is given it (`reflexive_trigger_counted`): "that many".
    fn may_pay_times(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, mana: ManaCost, most: u16, question: &'static Question) -> Result<u16, Suspend>;
    /// Whether `card` could be cast now by the ability's controller as an effect resolves (CR 608.2g), from where it
    /// is: it is not a land and has a legal target for every target it needs. The mana is not asked: a player who
    /// finds they cannot pay gives the cast up (CR 733.1).
    fn could_cast_during_resolution(&mut self, card: ObjectId) -> bool;
    /// `could_cast_during_resolution` for `player`, who the effect has cast it (`CastTerms::caster`; CR 608.2g): judged for
    /// them - their targets, what keeps them from casting spells - rather than for the effect's controller.
    fn could_cast_during_resolution_by(&mut self, card: ObjectId, player: PlayerId) -> bool;
    /// Whether the resolving effect's controller could play `card` as a land now (CR 305.1, 608.2g): it is their own turn
    /// (CR 305.3) and they have a land play left (CR 305.2); the phase and the stack do not matter.
    fn could_play_land_during_resolution(&self, card: ObjectId) -> bool;
    /// The resolving effect's controller plays `card` as a land from where it is (CR 305.1, 608.2g), counted as one of the
    /// turn's land plays (CR 305.2a). The effect has already asked whether to ("you may play"). The new object, or none if
    /// it could not be played.
    fn play_land_during_resolution(&mut self, card: ObjectId) -> Result<Option<ObjectId>, Suspend>;
    /// The resolving effect's controller casts `card` from where it is (CR 608.2g, 601.2): its targets are chosen and
    /// its costs paid as for any cast, for `instead` rather than its mana cost if given (CR 118.9: madness). Whether
    /// it was cast; a cast the player gives up (CR 733.1) leaves the card where it was.
    fn cast_during_resolution(&mut self, card: ObjectId, instead: Option<ManaCost>) -> Result<bool, Suspend>;
    /// The same, with terms (CR 608.2g): mana of any type may pay for it, an instant or sorcery is exiled instead of
    /// going to a graveyard.
    fn cast_during_resolution_as(&mut self, card: ObjectId, terms: CastTerms) -> Result<bool, Suspend>;
    /// Creates a copy of an object in exile, owned and controlled by `player` (CR 707.12, 702.192a: "create a copy of this
    /// object in exile"), and returns it. `of` may have left the zone it was in, a spell that has resolved: the copy is of it
    /// as it last existed (CR 608.2h). The copy is a copy of a card, which ceases to exist as a state-based action unless it
    /// is cast (CR 707.10a, 704.5e).
    fn create_copy_in_exile(&mut self, of: ObjectId, player: PlayerId) -> ObjectId;
    /// Each way the resolving effect's controller could cast `card` now (CR 608.2g), the faces it offers judged one at a time
    /// (CR 709.3a, 715.3a): a split card's halves, an aftermath half only from a graveyard (CR 702.127a); an Adventurer or omen card as itself
    /// and as its Adventure or Omen; a modal double-faced card's two faces. A fused cast of a fuse card in a hand (CR 702.102a,
    /// mana value the sum of both halves, CR 702.102c) is a way too: a limit on mana value is judged against that sum. Empty
    /// when `could_cast_during_resolution` is false.
    fn cast_ways_during_resolution(&mut self, card: ObjectId) -> Vec<CastWay>;
    /// `cast_ways_during_resolution` for `player`, who the effect has cast it (`CastTerms::caster`; CR 608.2g).
    fn cast_ways_during_resolution_by(&mut self, card: ObjectId, player: PlayerId) -> Vec<CastWay>;
    /// `player` may play or cast `card` from where it is (CR 601.3, 305.1) under `grant`: a card in exile or a
    /// graveyard, a new object after a move (CR 400.7), so the permission is for this object only.
    fn allow_play(&mut self, card: ObjectId, player: PlayerId, grant: PlayGrant);
    /// "Creatures can't block this turn" (CR 509.1b): a rule of the game until the turn ends, reaching creatures that
    /// arrive later too (CR 611.2c).
    fn creatures_cant_block(&mut self);
    /// `player` becomes the monarch (CR 725.1) and whoever was stops being it (CR 725.3).
    fn become_monarch(&mut self, player: PlayerId);
    /// `player` takes the initiative (CR 726.1): whoever had it stops, and the taker ventures into Undercity
    /// (CR 726.2), even one who already had it (CR 726.5).
    fn take_initiative(&mut self, player: PlayerId);
    /// `player` ventures into Undercity (CR 701.49): enters it, or moves on to the next room, their choice when
    /// there are two. The room's ability triggers (CR 309.4c). The one who ventures out of the bottommost room
    /// completes the dungeon (CR 701.49c, 309.7) and enters it again.
    fn venture(&mut self, player: PlayerId) -> Result<(), Suspend>;
    /// `player`'s speed increases by `amount` (CR 702.179): from none it becomes `amount` (CR 702.179c), and it goes no
    /// higher than 4, the max speed (CR 702.179e; the Aetherdrift release notes: "can't increase it any higher").
    fn increase_speed(&mut self, player: PlayerId, amount: u8);
    /// Whether the resolving effect's controller could cast `card` transformed now, from where it is (CR 608.2g,
    /// 712.11a): a double-faced card that is not modal, judged as its back face alone.
    fn could_cast_transformed_during_resolution(&mut self, card: ObjectId) -> bool;
    /// The resolving effect's controller casts `card` transformed, for nothing (CR 712.11a, 118.9): put on the stack
    /// with its back face up, it resolves into a permanent back face up (CR 712.13). Whether it was cast.
    fn cast_transformed_during_resolution(&mut self, card: ObjectId) -> Result<bool, Suspend>;
    /// "Damage can't be prevented this turn" (CR 615.12): a rule of the game until the turn ends.
    fn damage_cant_be_prevented(&mut self);
    /// Makes the object about to enter a copy (CR 707.2).
    fn set_copy(&mut self, object: ObjectId, copy: Copiable);
    /// `player` gets energy counters (CR 107.14), or with a negative `delta` pays them (CR 122.1): never below none.
    fn change_energy(&mut self, player: PlayerId, delta: i32);
    /// "You may activate this permanent's loyalty abilities `times` more times this turn" (CR 606.3): recorded in the
    /// history, which is what the one-a-turn limit is judged against.
    fn allow_extra_loyalty_activations(&mut self, permanent: ObjectId, times: u8);
    /// `player` proliferates (CR 701.34a): chooses any number of permanents and players that have a counter, each of
    /// which gets another counter of each kind it already has. One question, asked of `player` about `source`'s effect;
    /// nothing is asked when no permanent or player has a counter.
    fn proliferate(&mut self, player: PlayerId, source: ObjectId) -> Result<(), Suspend>;
}

/// Carries out `f`, several calls that are one instruction of a card's text and ask no question, as one
/// (`Game::one_instruction`): "target creature gets +2/+2 and gains hexproof", "each creature you control gets +1/+1".
pub fn instruction(g: &mut dyn Game, f: impl FnOnce(&mut dyn Game)) {
    let mut f = Some(f);
    let _ = g.one_instruction(&mut |g| {
        if let Some(f) = f.take() {
            f(g);
        }
        Ok(())
    });
}
