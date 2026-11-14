//! Merging with permanents (CR 730) and the mutating creature spell that does it (CR 702.140c-f).
//!
//! A merged permanent is the object it was before it merged (CR 730.2c): its row keeps its id, controller, counters, status,
//! attachments and the effects on it, and its own card stays its card. What else represents it is its `Merge`
//! (`GameState::merges`, named by `GameState::merged_objects`): each card, token or copy that merged with it, over or under, with the
//! time it merged. Each merge is a copiable effect at that time (CR 730.2a, 613.2), worked out in the copy chain with the
//! copy effects on it (`GameState::rechain_merged`): the permanent has its topmost component's characteristics and every
//! ability of each other component (CR 702.140e), which its copiable values carry (`Copiable::merged`), so a copy of it has
//! them too. As it leaves the battlefield, one permanent leaves and each component goes to the same zone, a new object
//! each (CR 730.3, `components_moving`).

use cantrip_api::*;
use cantrip_state::{Merge, MergedPart};

use crate::{Cx, R, ops};

/// Whether the spell `spell` is a mutating creature spell (CR 702.140a): cast for its mutate cost, it has its card's mutating
/// face as its copiable values (`cantrip_api::mutating_face`) until it ceases to be one or merges.
pub(crate) fn mutating(cx: &Cx, spell: ObjectId) -> bool {
    cx.s().obj(spell).copy.is_some_and(|c| cx.card(c.card).mutating)
}

/// `values`, the copiable values of a mutating creature spell, as those of the card it is once it is no longer mutating (CR
/// 702.140b-c): the card's own, or - for a copy of a spell whose values have exceptions (CR 707.10) - those exceptions over
/// the card. `None` where they are the card's own.
fn unmutated(cards: &CardTable, card: CardId, values: Option<Copiable>) -> Option<Copiable> {
    let values = values?;
    if !cards.get(values.card).mutating {
        return Some(values);
    }
    let card_values = Copiable { card, ..values };
    card_values.has_exceptions().then_some(card_values)
}

/// CR 702.140b, 608.3b: a mutating creature spell whose target is illegal as it begins to resolve ceases to be one, and goes
/// on resolving as the creature spell it is.
pub(crate) fn cease_mutating(cx: &mut Cx, spell: ObjectId) {
    let obj = *cx.s().obj(spell);
    let values = unmutated(cx.cards(), obj.card, obj.copy);
    cx.st().update_obj(spell, |o| o.copy = values);
}

/// CR 702.140c, 608.3d, 730.2: the mutating creature spell `item` resolves with `target`, its target, legal: it doesn't enter
/// the battlefield but merges with the target, put on top of it or under it as the spell's controller chooses. The spell
/// leaves the stack and becomes part of the permanent, which is the same object it was (CR 730.2b-c): not new to the
/// battlefield, nor to its controller. An effect that refers to or modifies the spell refers to or modifies the merged
/// permanent from then on (CR 702.140f): the effects on the spell go on to it, and so does what the spell's paying gave it
/// (haste from mana, a granted way's ability); and the spell, gone, is followed to it (`Obj::became`).
pub(crate) fn merge(cx: &mut Cx, item: &StackItem, target: ObjectId) -> R<()> {
    let spell = item.object;
    let ask = RequestKind::ChooseModes {
        source: spell,
        modes: vec![Label::Mutate { on_top: true }, Label::Mutate { on_top: false }],
        eligible: vec![0, 1],
        count: 1,
        at_most: 0,
        affordable: Vec::new(),
        additional_cost: false,
        repeat: false,
    };
    let on_top = match cx.ask_core(item.controller, ask, false)? {
        Answer::Modes(chosen) => chosen.first().is_none_or(|&n| n == 0),
        _ => true,
    };
    let s = cx.s();
    let card = *s.obj(spell);
    let permanent = *s.obj(target);
    let part = MergedPart {
        card: card.card,
        face: cx.cards().face(card.card, card.transformed),
        // A copy of a permanent spell becomes a token as it resolves (CR 707.10f, 111.1): merging, it is a token component.
        token: card.token || card.card_copy,
        commander_id: card.commander_id,
        copy: unmutated(cx.cards(), card.card, card.copy),
        on_top,
        timestamp: 0,
        prior: permanent.copy,
        melded: false,
    };
    let mut merge = s.merge_of(target).cloned().unwrap_or_else(|| Merge {
        own_face: cx.cards().face(permanent.card, permanent.transformed),
        own_token: permanent.token,
        own_commander_id: permanent.commander_id,
        own_face_down: None,
        parts: Vec::new(),
    });
    // The spell leaves the stack (CR 730.2b): it is no object any more, but part of the permanent, which an effect that refers
    // to it finds (CR 702.140f).
    if let Some(at) = cx.s().stack.iter().rposition(|i| i.serial == item.serial) {
        cx.st().stack_remove(at);
    }
    let known = ops::last_known_of(cx, spell);
    let row = cx.st().last_known_push(known);
    cx.st().zone_remove(Zone::STACK, spell);
    cx.st().update_obj(spell, |o| {
        o.became = Some(target);
        o.last_known = Some(row);
    });
    let timestamp = cx.st().timestamp();
    merge.parts.push(MergedPart { timestamp, ..part });
    if on_top {
        // A merged permanent is a token only while its topmost component is (CR 730.2d), and face down only while its topmost
        // component is (CR 730.2e): a face-down permanent a face-up card is put on top of is face up, though no effect counts it
        // as turned face up, and its own card stays face down under it.
        if permanent.marks.face_down_as.is_some() && merge.parts.iter().filter(|p| p.on_top).count() == 1 {
            merge.own_face_down = permanent.marks.face_down_as;
        }
        cx.st().update_obj(target, |o| {
            o.token = part.token;
            o.face_down = false;
            o.marks.face_down_as = None;
        });
    }
    let commander_id = merge.parts.iter().rfind(|p| p.on_top).map_or(merge.own_commander_id, |p| p.commander_id);
    let has_commander = merge.own_commander_id.is_some() || merge.parts.iter().any(|p| p.commander_id.is_some());
    cx.st().set_merge(target, merge);
    cx.st().update_obj(target, |obj| {
        obj.commander_id = commander_id;
        obj.commander = has_commander;
    });
    // A card with static abilities is now part of a permanent (CR 702.140e): the layers look for them from here on.
    ops::note_statics(cx, target);
    // What modifies the spell modifies the merged permanent (CR 702.140f): one instruction of the merge, judged once it is done.
    cx.instruction(
        |cx| {
            ops::carry_effects(cx, spell, target);
            if card.marks.gains_haste {
                Game::add_effect(cx, target, target, EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])), Duration::EndOfTurn);
            }
            if let Some(ability) = card.marks.gains {
                Game::add_effect(cx, target, target, EffectKind::AddAbility(ability), Duration::Indefinite);
            }
            cx.state_changed();
        },
        |_| true,
    );
    cx.emit(Event::Mutated { permanent: target, spell, on_top });
    Ok(())
}

/// The meld card `card` joining `permanent`, its counterpart as it enters (CR 701.42a, 712.14c): it leaves its zone - no object
/// any more, but part of the permanent, which an effect that refers to it finds (CR 400.7) - and the permanent has only the
/// pair's combined back face's characteristics (CR 712.8g), its own face here, over the card it is. As it leaves, the two
/// cards go to the new zone (CR 712.21), as any merged permanent's components do (`components`). The event that records it.
pub(crate) fn meld_with(cx: &mut Cx, permanent: ObjectId, card: ObjectId) -> Event {
    let own = cx.s().obj(permanent).card;
    let into = cx.cards().id_of(cx.card(own).meld.expect("a melding card is a meld card").into);
    let joining = *cx.s().obj(card);
    let known = ops::last_known_of(cx, card);
    let row = cx.st().last_known_push(known);
    cx.st().zone_remove(joining.zone, card);
    cx.st().update_obj(card, |o| {
        o.became = Some(permanent);
        o.last_known = Some(row);
    });
    let timestamp = cx.st().timestamp();
    let part = MergedPart { card: joining.card, face: joining.card, token: false, commander_id: joining.commander_id, copy: None, on_top: false, timestamp, prior: None, melded: true };
    let own_commander_id = cx.s().obj(permanent).commander_id;
    cx.st().set_merge(permanent, Merge { own_face: into, own_token: false, own_face_down: None, own_commander_id, parts: vec![part] });
    Event::Melded { permanent, card }
}

/// One component of a merged permanent (CR 730.2), as it leaves the battlefield (CR 730.3): the card, token or copy it is, and
/// how it is.
#[derive(Clone, Copy)]
pub(crate) struct Component {
    pub card: CardId,
    pub commander_id: Option<ObjectId>,
    pub token: bool,
    pub transformed: bool,
    /// Face down as it leaves: revealed as it moves (CR 708.9).
    pub face_down: bool,
    /// It is the topmost component: the one whose characteristics the permanent has had (CR 730.2a), which the permanent's
    /// own move to its new zone is the move of.
    pub top: bool,
}

/// Commander identities physically represented by a merged permanent, for damage events at that moment.
pub(crate) fn commander_ids(s: &cantrip_state::GameState, id: ObjectId) -> [ObjectId; 8] {
    let mut ids = [ObjectId::NONE; 8];
    let mut next = 0;
    let mut add = |identity: Option<ObjectId>| {
        if let Some(identity) = identity && !ids.contains(&identity) && next < ids.len() {
            ids[next] = identity;
            next += 1;
        }
    };
    add(s.obj(id).commander_id);
    if let Some(merge) = s.merge_of(id) {
        add(merge.own_commander_id);
        for part in &merge.parts { add(part.commander_id); }
    }
    ids
}

/// Every component of the merged permanent `id` (CR 730.2): its own card first, then each that merged with it in the order it
/// merged. None for a permanent that isn't merged.
pub(crate) fn components(cx: &Cx, id: ObjectId) -> Option<Vec<Component>> {
    let merge = cx.s().merge_of(id)?;
    let obj = cx.s().obj(id);
    let top = merge.parts.iter().rposition(|p| p.on_top).map_or(0, |i| i + 1);
    let mut all = Vec::with_capacity(merge.parts.len() + 1);
    // Each is a new object where it goes (CR 400.7), with none of the values copy effects gave the permanent, as any object
    // that leaves the battlefield.
    all.push(Component {
        card: obj.card,
        commander_id: merge.own_commander_id,
        token: merge.own_token,
        transformed: obj.transformed,
        face_down: obj.face_down || merge.own_face_down.is_some(),
        top: top == 0,
    });
    for (i, p) in merge.parts.iter().enumerate() {
        all.push(Component { card: p.card, commander_id: p.commander_id, token: p.token, transformed: p.face != p.card, face_down: obj.face_down, top: top == i + 1 });
    }
    Some(all)
}

/// The values of every component of `id` but its topmost, whose abilities it has (CR 702.140e): the values its copiable
/// values carry (`Copiable::merged`). Empty for an object that isn't merged and isn't a copy of one.
pub(crate) fn sources<'a>(s: &'a GameState, values: Option<&Copiable>) -> &'a [Copiable] {
    match values {
        Some(c) if c.merged != 0 => s.merged_sources(c),
        _ => &[],
    }
}

/// CR 730.3a: the components of a merged permanent put into their owner's library together - at its top, as they arrived,
/// `group` its own card's object first - are in the order that player arranges, which they don't reveal (the question is
/// theirs alone).
pub(crate) fn arrange_in_library(cx: &mut Cx, owner: PlayerId, group: &[ObjectId]) {
    let library = Zone::library(owner);
    // Top first, as they lie.
    let listed: Vec<ObjectId> = cx.s().zone(library).iter().copied().filter(|id| group.contains(id)).collect();
    if listed.len() < 2 {
        return;
    }
    let source = match cx.cause() {
        crate::Cause::Spell(spell) => spell,
        crate::Cause::Ability { source, .. } => source,
        _ => listed[0],
    };
    let ask = RequestKind::DistributeCards {
        source,
        cards: listed.clone(),
        destinations: vec![Destination { kind: DestinationKind::LibraryTop, min: listed.len() as u8, max: listed.len() as u8, eligible: listed.clone() }],
        may_shuffle: false,
    };
    let order = match cx.ask_rule(owner, ask) {
        Ok(Some(Answer::Placements { placements, .. })) => placements.into_iter().next().unwrap_or_default(),
        _ => return,
    };
    if order == listed || order.len() != listed.len() || !listed.iter().all(|id| order.contains(id)) {
        return;
    }
    let at = cx.s().zone(library).iter().position(|id| listed.contains(id)).unwrap_or(0);
    for &card in &listed {
        cx.st().zone_remove(library, card);
    }
    for (k, &card) in order.iter().enumerate() {
        cx.st().zone_insert(library, at + k, card);
    }
}


/// CR 730.2i: transforming the merged permanent `id` turns each of its double-faced components that can transform (CR
/// 701.27a, 701.27d: not one whose other face is an instant or sorcery) so that its other face is up, its own card as any
/// double-faced permanent's and each other's in its merge. Whether any turned.
pub(crate) fn turn_components(cx: &mut Cx, id: ObjectId) -> bool {
    let Some(mut merge) = cx.s().merge_of(id).cloned() else { return false };
    let cards = cx.cards();
    let can_turn = |card: CardId, transformed: bool| cards.backs[card.0 as usize].is_some() && !ops::instant_or_sorcery(cards.get(cards.face(card, !transformed)));
    let obj = *cx.s().obj(id);
    let own = obj.represented_double_faced(cards.backs[obj.card.0 as usize].is_some()) && can_turn(obj.card, obj.transformed);
    let mut any = own;
    for p in &mut merge.parts {
        let transformed = p.face != p.card;
        if can_turn(p.card, transformed) {
            p.face = cards.face(p.card, !transformed);
            any = true;
        }
    }
    if !any {
        return false;
    }
    if own {
        cx.st().update_obj(id, |o| o.transformed = !o.transformed);
        merge.own_face = cards.face(obj.card, !obj.transformed);
    }
    cx.st().set_merge(id, merge);
    true
}

/// The game as a rule of one of the merged permanent `source`'s other components reads it, where the rule is asked with the
/// object alone (a cost change, `CostChange`): `source`'s text is that component's (`Query::text_linked`, CR 607.1), so it
/// reads what its own card chose and exiled. Everything else is the game it wraps.
pub(crate) struct AsText<'a> {
    pub q: &'a dyn Query,
    pub source: ObjectId,
    pub link: Linked,
}

impl Query for AsText<'_> {
    fn can_attach_to(&self, attachment: ObjectId, host: ObjectId) -> bool {
        crate::attach::host_allows(self, attachment, host, true)
    }

    fn enchanting_forbidden(&self, host: ObjectId, aura: ObjectId) -> bool {
        self.q.enchanting_forbidden(host, aura)
    }

    fn can_untap(&self, permanent: ObjectId) -> bool {
        self.q.can_untap(permanent)
    }

    fn goaders(&self, creature: ObjectId) -> Vec<PlayerId> {
        self.q.goaders(creature)
    }

    fn has_granted_way(&self, card: ObjectId) -> bool {
        self.q.has_granted_way(card)
    }

    fn producible_types(&self, permanent: ObjectId) -> (ColorSet, bool) {
        self.q.producible_types(permanent)
    }

    fn linked_card(&self, card: CardId) -> CardId {
        self.q.linked_card(card)
    }

    fn names_its_source(&self, waiting: Waiting) -> bool {
        self.q.names_its_source(waiting)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        if source == self.source { self.link } else { self.q.text_linked(source) }
    }

    fn state(&self) -> &GameState {
        self.q.state()
    }

    fn events(&self) -> &[Record] {
        self.q.events()
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.q.card_id(def)
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.q.card(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        self.q.chars(id)
    }

    fn mana_value(&self, id: ObjectId) -> u32 {
        self.q.mana_value(id)
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        self.q.token_chars(card, copy)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        self.q.lost_all_abilities(id)
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        self.q.can_target(asker, target)
    }

    fn chosen_targets(&self) -> &[Target] {
        self.q.chosen_targets()
    }

    fn event_subject(&self) -> Target {
        self.q.event_subject()
    }

    fn event_data(&self) -> [u32; 2] {
        self.q.event_data()
    }

    fn slot_opponent(&self) -> Option<PlayerId> {
        self.q.slot_opponent()
    }

    fn gift_assumed(&self) -> bool {
        self.q.gift_assumed()
    }

    fn announced_x(&self) -> Option<u32> {
        self.q.announced_x()
    }

    fn obj(&self, id: ObjectId) -> &Obj {
        self.q.obj(id)
    }
}
