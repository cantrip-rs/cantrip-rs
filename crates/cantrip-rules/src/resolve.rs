//! Resolving the top of the stack (CR 608), and countering (CR 701.6).

use cantrip_api::*;

use crate::layers::{self, live};
use crate::{Cx, Flow, R, ops};

/// The card whose characteristics a spell on the stack has: its Adventure's or Omen's while it is cast as one (CR 715.3b,
/// 720.3b).
fn face(state: &GameState, spell: ObjectId) -> CardId {
    let obj = state.obj(spell);
    obj.copied_card().unwrap_or(obj.card)
}

fn target_specs(cards: &CardTable, state: &GameState, item: &StackItem) -> &'static [TargetSpec] {
    match item.kind {
        StackKind::Spell => {
            let def = cards.get(face(state, item.object));
            // A fused split spell's (CR 702.102b), and as it resolves each half's own, on the view of the item that half's
            // instructions are followed with (`run_fused`).
            match (def.split, item.way) {
                (Some((left, _)), WAY_SPLIT_LEFT) if def.fuse => left.spell_targets(),
                (Some((_, right)), WAY_SPLIT_RIGHT) if def.fuse => right.spell_targets(),
                _ => cards.spell_targets_of(def),
            }
        }
        _ => match cards.ability(item.ability) {
            AbilityDef::Activated(a) => a.targets,
            AbilityDef::Triggered(t) => t.targets,
            AbilityDef::Static(_) => &[],
        },
    }
}

/// The modes an item on the stack was put there with (CR 700.2), if it is modal.
pub fn modal_of(cards: &CardTable, state: &GameState, item: &StackItem) -> Option<&'static ModalDef> {
    match item.kind {
        StackKind::Spell => cards.get(face(state, item.object)).modal,
        StackKind::Activated => match cards.ability(item.ability) {
            AbilityDef::Activated(a) => a.modal,
            AbilityDef::Triggered(_) | AbilityDef::Static(_) => None,
        },
        StackKind::Triggered => match cards.ability(item.ability) {
            AbilityDef::Triggered(t) => t.modal,
            AbilityDef::Activated(_) | AbilityDef::Static(_) => None,
        },
    }
}

/// The slots a list of target specs stands for, for a spell or ability controlled by `controller` (CR 601.2c): one each,
/// except that a `TargetSpec::EACH_OPPONENT` is a slot of at most one target for each seat but the controller's, in seat
/// order, each with the opponent it is for. A player who has left keeps their seat, so the others keep their places.
/// Each slot also says the index of the spec it comes from, which is what a target chosen for it records
/// (`StackItem::spec_of`).
pub(crate) fn expand_specs(state: &GameState, controller: PlayerId, specs: &[TargetSpec]) -> Vec<(TargetSpec, Option<PlayerId>, u8)> {
    let mut out = Vec::with_capacity(specs.len());
    for (i, spec) in specs.iter().enumerate() {
        if spec.max == TargetSpec::EACH_OPPONENT {
            let each = TargetSpec { min: 0, max: 1, ..*spec };
            out.extend(state.seats().filter(|&p| p != controller).map(|p| (each, Some(p), i as u8)));
        } else {
            out.push((*spec, None, i as u8));
        }
    }
    out
}

/// The spec target `slot` of an item was chosen for, out of `specs` (CR 601.2c): the one it recorded (`StackItem::spec_of`),
/// wherever in the list a slot of several targets stands. The nth target of a `TargetSpec::EACH_OPPONENT` is a slot of at
/// most one target for the nth opponent in seat order (`expand_specs`).
fn recorded_spec(state: &GameState, item: &StackItem, specs: &[TargetSpec], slot: usize) -> Option<(TargetSpec, Option<PlayerId>)> {
    let at = state.spec(item, slot) as usize;
    let spec = specs.get(at)?;
    if spec.max != TargetSpec::EACH_OPPONENT {
        return Some((*spec, None));
    }
    let nth = slot - state.spec_targets(item, at).start;
    let opponent = state.seats().filter(|&p| p != item.controller).nth(nth);
    Some((TargetSpec { min: 0, max: 1, ..*spec }, opponent))
}

/// The spec of an item's target `slot`, and the opponent the slot is for if it is one of an each-opponent spec: the slot the
/// target was chosen for (`recorded_spec`), among the item's printed specs or, for a modal item, the specs of its chosen modes
/// in printed order (CR 700.2c, 601.2c).
pub(crate) fn slot_spec(cards: &CardTable, state: &GameState, item: &StackItem, slot: usize) -> Option<(TargetSpec, Option<PlayerId>)> {
    if let Some(modal) = modal_of(cards, state, item) {
        return recorded_spec(state, item, &crate::actions::mode_specs(modal, item.modes), slot);
    }
    let specs = target_specs(cards, state, item);
    // CR 702.47c: the slots of spliced text follow the spell's own, in the order the cards were spliced, and are numbered on
    // from them (`StackItem::spec_of`).
    if item.kind == StackKind::Spell && state.spec(item, slot) as usize >= specs.len() && item.data[0] > 0 {
        let mut all = specs.to_vec();
        for spliced in spliced(state, item) {
            all.extend_from_slice(cards.get(spliced).spell.map_or(&[][..], |s| s.targets));
        }
        return recorded_spec(state, item, &all, slot);
    }
    recorded_spec(state, item, specs, slot)
}

/// Every target spec a spell on the stack chose its targets for, numbered as `StackItem::spec_of` records them (CR 601.2c):
/// a modal spell's are those of the modes it was cast with, in printed order, a mode chosen twice twice (CR 700.2c-d), each
/// spec that has no words of its own labelled with its mode's text, so that whoever chooses its targets again knows which
/// mode they are for; any other spell's are those of the face it is on the stack as (CR 715.3b, 720.3b, 709.3b), an
/// Aura's its enchant ability (CR 303.4a), followed by those of the cards spliced onto it (CR 702.47c). A copy of a spell
/// has the same modes and spliced text (CR 707.10, 707.10c), so its targets are read the same way.
pub(crate) fn spell_specs(cards: &CardTable, state: &GameState, item: &StackItem) -> Vec<TargetSpec> {
    if let Some(modal) = modal_of(cards, state, item) {
        let mut specs = Vec::new();
        for (i, mode) in modal.modes.iter().enumerate() {
            for _ in 0..modal.times(item.modes, i) {
                specs.extend(mode.targets.iter().map(|s| if s.label.is_empty() { s.labelled(mode.text) } else { *s }));
            }
        }
        return specs;
    }
    let mut specs = target_specs(cards, state, item).to_vec();
    for spliced in spliced(state, item) {
        specs.extend_from_slice(cards.get(spliced).spell.map_or(&[][..], |s| s.targets));
    }
    specs
}

/// The cards spliced onto a spell (CR 702.47a-c), in the order they were spliced, any number of them: the first
/// `data[0]` objects its announcement chose (`StackItem::members`), as they were in the hand. A copy of the spell has the
/// same (CR 707.10c). None for an ability.
pub(crate) fn spliced(state: &GameState, item: &StackItem) -> Vec<CardId> {
    if item.kind != StackKind::Spell {
        return Vec::new();
    }
    let members = state.members(item.members);
    members[..(item.data[0] as usize).min(members.len())].iter().map(|&o| state.obj(o).card).collect()
}

/// Runs the chosen modes of a modal item in printed order (CR 700.2d), each as if it were a spell of its own with
/// its own targets in slot 0 onward. A mode whose targets have all become illegal is still carried out, as long as the
/// item resolves for another mode's legal target: only the parts of it that would act on an illegal target are not
/// done, and its other instructions are (CR 608.2b; the rulings on the Commands and on escalate), so each mode's effect
/// checks its own targets (`targets::object`, `Game::target_legal`).
pub fn run_modes(cx: &mut Cx, item: &StackItem, modal: &ModalDef) -> Result<(), Suspend> {
    let mut next = 0usize;
    let mut spec_at = 0usize;
    for (i, mode) in modal.modes.iter().enumerate() {
        // A mode chosen more than once is that many instances in sequence, each with targets of its own (CR 700.2d).
        for _ in 0..modal.times(item.modes, i) {
            // The targets recorded for this instance's slots (`StackItem::spec_of`), their slots renumbered from its first.
            let ends = spec_at + mode.targets.len();
            let n = (next..item.target_count as usize).take_while(|&t| (cx.s().spec(item, t) as usize) < ends).count();
            let mut sub = *item;
            sub.modes = modal.once(i);
            let of: Vec<u8> = (0..n).map(|t| cx.s().spec(item, next + t) - spec_at as u8).collect();
            let targets = cx.s().targets(item)[next..next + n].to_vec();
            let shares: Vec<u32> = (next..next + n).map(|t| cx.s().share(item, t)).collect();
            cx.st().set_item_targets(&mut sub, &targets, &of, &shares);
            spec_at = ends;
            next += n;
            // Each mode's instructions follow the last mode's (CR 608.2c), which the state triggers have seen done.
            cx.judge_if_due();
            (mode.effect)(cx, &sub)?;
        }
    }
    // An instruction printed ahead of the modes that they share is done once, after them, to all their targets (CR 700.2).
    if let Some(shared) = modal.shared {
        cx.judge_if_due();
        shared(cx, item)?;
    }
    Ok(())
}

/// Follows a fused split spell's instructions (CR 702.102d): the left half's, then the right half's, each on a view of the
/// item that is that half cast alone - its own targets in slot 0 onward, read by its own specs (`target_specs`, the item's
/// way naming the half). A half whose target has become illegal does nothing to it, as for any spell (CR 608.2b, the
/// ruling: "an illegal target can't perform any actions or have any actions performed on it").
fn run_fused(cx: &mut Cx, item: &StackItem, left: &'static CardDef, right: &'static CardDef) -> Result<(), Suspend> {
    let split = left.spell_targets().len();
    let count = item.target_count as usize;
    // The targets recorded for the left half's specs come first (`StackItem::spec_of`), then the right half's.
    let first = (0..count).take_while(|&t| (cx.s().spec(item, t) as usize) < split).count();
    for (way, half, from, to, base) in [(WAY_SPLIT_LEFT, left, 0, first, 0), (WAY_SPLIT_RIGHT, right, first, count, split)] {
        let Some(spell) = half.spell else { continue };
        let mut sub = *item;
        sub.way = way;
        let of: Vec<u8> = (from..to).map(|t| cx.s().spec(item, t) - base as u8).collect();
        // Shares are kept only where a divided total was announced.
        let shares = cx.s().shares(item);
        let shares = if shares.is_empty() { Vec::new() } else { shares[from..to].to_vec() };
        let targets = cx.s().targets(item)[from..to].to_vec();
        cx.st().set_item_targets(&mut sub, &targets, &of, &shares);
        // The right half's instructions follow the left half's (CR 608.2c), which the state triggers have seen done.
        cx.judge_if_due();
        (spell.effect)(cx, &sub)?;
    }
    Ok(())
}

/// Whether target `slot` of an item is still legal (CR 608.2b): the target is still what it was, the slot's filter
/// still takes it, nothing forbids the source to target it, and for a slot whose targets together are judged
/// (`TargetSpec::together`), the rule still allows the slot's targets together - each read as it is, or as it last was if
/// it has left (Run Away Together's ruling) - or none of them is legal (Nethroi, Apex of Death's ruling).
pub fn target_legal(cards: &CardTable, state: &GameState, events: &[Record], item: &StackItem, slot: usize) -> bool {
    let target = state.target(item, slot);
    // A slot that chose no target (an "up to" slot) has nothing to be legal (CR 601.2c).
    if target == Target::None {
        return false;
    }
    let Some((spec, slot_player)) = slot_spec(cards, state, item, slot) else {
        return false;
    };
    // Asked as the spell or ability it is (CR 607.5): one an effect gave its source reads only its own links.
    let by = Linked { object: item.object, text: cards.linked_card(item.ability.card), acquired: item.acquired };
    let asker = Asker { source: item.object, controller: item.controller, by: Some(by).filter(|&by| by != cards.linked_to(state, item.object)), spell: item.kind == StackKind::Spell };
    if let Target::Object(id) = target
        && !live(state, id)
    {
        return false;
    }
    let q = Reader { cards, state, events };
    // A slot of its own sees what the slots before it chose (CR 601.2c).
    let earlier = if item.target_count > 1 { &state.targets(item)[..slot] } else { &[][..] };
    (spec.filter)(&Given { q: &q, earlier, subject: item.subject, slot_player, gift: false, x: announced_x(item) }, &asker, target)
        && layers::can_target(cards, state, events, &asker, target)
        && (spec.together.is_none() || together_legal(&q, state, item, slot, &spec))
}

/// Whether the targets of the slot target `slot` of `item` was chosen for are still allowed together by `spec`'s rule
/// (`TargetSpec::together`, CR 608.2b), its numbers read with the count its targets were chosen with (`StackItem::counted`).
fn together_legal(q: &dyn Query, state: &GameState, item: &StackItem, slot: usize, spec: &TargetSpec) -> bool {
    let run = state.spec_targets(item, state.spec(item, slot) as usize);
    let targets: Vec<Target> = state.targets(item)[run].iter().copied().filter(|&t| t != Target::None).collect();
    select::targets_together(q, spec.together_at(u32::from(item.counted)).as_slice(), item.controller, item.object, &targets)
}

/// The X announced for a spell or an activated ability on the stack (CR 107.3a, 601.2b, 602.2b), its own (CR 107.3k): what
/// its targets are judged with (`Query::announced_x`). A triggered ability announces none.
pub fn announced_x(item: &StackItem) -> Option<u16> {
    (item.kind != StackKind::Triggered).then_some(item.x)
}

/// A `Query` that also says what the earlier target slots chose, for the filter of a later one.
pub struct Given<'a> {
    pub q: &'a dyn Query,
    pub earlier: &'a [Target],
    /// The subject of the event the ability triggered on.
    pub subject: Target,
    /// The opponent the slot is for, when it is one of a `TargetSpec::EACH_OPPONENT`.
    pub slot_player: Option<PlayerId>,
    /// The targets are judged as if the spell's gift were promised (`Query::gift_assumed`).
    pub gift: bool,
    /// The X announced for the spell or ability whose targets these are (`Query::announced_x`, `announced_x`).
    pub x: Option<u16>,
}

impl Query for Given<'_> {
    fn linked_card(&self, card: CardId) -> CardId {
        self.q.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.q.text_linked(source)
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
        self.earlier
    }

    fn event_subject(&self) -> Target {
        self.subject
    }

    fn slot_opponent(&self) -> Option<PlayerId> {
        self.slot_player
    }

    fn gift_assumed(&self) -> bool {
        self.gift
    }

    fn announced_x(&self) -> Option<u32> {
        self.x.map(u32::from)
    }

    /// As the game it wraps has it: a spell judged as announced has the X and kicker announced (`actions::Announcing`).
    fn obj(&self, id: ObjectId) -> &Obj {
        self.q.obj(id)
    }
}

/// A read-only `Query` over a state, for checks made outside a frame's run.
pub struct Reader<'a> {
    pub cards: &'a CardTable,
    pub state: &'a GameState,
    pub events: &'a [Record],
}

impl Query for Reader<'_> {
    fn linked_card(&self, card: CardId) -> CardId {
        self.cards.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.cards.linked_to(self.state, source)
    }

    fn state(&self) -> &GameState {
        self.state
    }

    fn events(&self) -> &[Record] {
        self.events
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.cards.id_of(def)
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.cards.get(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        layers::chars(self.cards, self.state, self.events, id)
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        layers::token_chars(self.cards, card, copy)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        layers::lost_all_abilities(self.cards, self.state, self.events, id)
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        layers::can_target(self.cards, self.state, self.events, asker, target)
    }
}

/// Where a spell goes as it leaves the stack other than to the battlefield: its owner's graveyard (CR 608.2n, 701.6a,
/// 608.2b), unless a replacement effect sends it elsewhere. Among them are the spell's own (`its_own_exile`) and, as it
/// resolves, the rules' "instead of putting it into its owner's graveyard as it resolves" for how it was cast: an Adventure's
/// exile (CR 715.3d), an Omen's shuffle into its owner's library (CR 720.3d) and rebound's exile (CR 702.88a). Each is a
/// replacement effect (CR 614.1a), ordered by the spell's controller with the others that would change the move (CR 616.1,
/// `replace`); what follows one - the card on an Adventure, rebound's delayed trigger - follows only if it applied.
///
/// `items` leave together, as one instruction makes them (CR 608.2c: "counter all spells", `counter_items`): their moves are one
/// batch, so an ability that triggers on "one or more" of them triggers once (CR 603.2c), and the copies among them cease to
/// exist at once. Abilities among them are already gone.
fn leave_stack(cx: &mut Cx, items: &[StackItem], reason: ZoneReason) {
    let resolving = reason == ZoneReason::Resolve;
    let mut ceasing = Vec::new();
    let mut moves = Vec::new();
    for item in items.iter().filter(|i| i.kind == StackKind::Spell) {
        let obj = *cx.s().obj(item.object);
        // A copy of a card leaves the stack for nowhere: it ceases to exist (CR 707.10a, 722.3c). A copy of an Omen is an Omen
        // too (CR 720.3c): resolving, its owner's library is still shuffled, though the copy, in no zone it can be in, is not
        // put there (CR 707.10a, 704.5e).
        if obj.card_copy {
            ceasing.push((Zone::STACK, item.object));
            if item.way == WAY_OMEN && resolving {
                crate::turn::shuffle_library(cx, obj.owner);
            }
            continue;
        }
        let def = cx.card(obj.card);
        // Paradigm (CR 702.192a): "exile this spell" as it resolves, an instruction of the spell; a spell that is countered or
        // fizzles is not exiled.
        if def.paradigm.is_some() && resolving {
            ops::move_to_for(cx, item.object, Zone::exile(obj.owner), reason);
            continue;
        }
        let moving = ops::Moving {
            adventure: resolving && item.way == WAY_ADVENTURE,
            omen: resolving && item.way == WAY_OMEN,
            // "If this spell was cast from your hand" (CR 702.88a).
            rebound: resolving && def.rebound && obj.cast_from == Some(ZoneKind::Hand),
            ..ops::Moving::to(item.object, Zone::graveyard(obj.owner))
        };
        moves.push(its_own_exile(cx, item, moving, reason));
    }
    if !ceasing.is_empty() {
        ops::cease_together(cx, &ceasing);
    }
    if !moves.is_empty() {
        ops::move_together(cx, &moves, reason);
    }
}

/// `moving`, the spell `item`'s move off the stack, with the spell's own replacement effects of that move (CR 614.1a), which
/// the core orders with the others that would change it by the spell's controller (CR 616.1, `replace`): flashback's
/// "exile this card instead of putting it anywhere else any time it would leave the stack" (CR 702.34a) - printed, given by a
/// permission or a permanent - harmonize's (CR 702.180a) and aftermath's (CR 702.127a), of a move anywhere but exile; and
/// "if that spell would be put into your graveyard, exile it instead", which the effect that had it cast made
/// (`StackItem::exile_instead`), of a move to the graveyard alone.
pub(crate) fn its_own_exile(cx: &Cx, item: &StackItem, moving: ops::Moving, reason: ZoneReason) -> ops::Moving {
    let card = cx.s().obj(item.object).card;
    let flashback = item.flashback || (item.way > 0 && cx.card(card).ways.get(item.way as usize - 1).is_some_and(|w| w.exile_after));
    // "Instead of putting it into your graveyard as it resolves" (Goliath Daydreamer) is of the resolution alone: a spell that is
    // countered does not resolve and goes to the graveyard (CR 701.6a).
    let exile_instead = if item.exile_instead == ExileInstead::Dream && reason != ZoneReason::Resolve { ExileInstead::Never } else { item.exile_instead };
    ops::Moving { flashback, exile_instead, ..moving }
}

pub fn resolve(cx: &mut Cx) -> R<Flow> {
    // The item this frame resolves (`Cx::resolving`), wherever it is: what it cast or copied as it resolved went above it.
    let Some(&item) = cx.resolving() else { return Ok(Flow::Done) };
    let on_stack = |cx: &Cx| cx.s().stack.iter().rposition(|i| i.serial == item.serial);

    // CR 608.2b: a spell or ability whose every target has become illegal does nothing.
    let count = item.target_count as usize;
    let chose_any = cx.s().targets(&item).iter().any(|&t| t != Target::None);
    let mut fizzles = chose_any && (0..count).all(|slot| !target_legal(cx.cards(), cx.s(), &cx.g.log, &item, slot));
    // CR 702.103e: a bestowed Aura spell whose target is illegal does not fizzle: it ceases to be bestowed and resolves as
    // the creature spell it is.
    if fizzles && item.kind == StackKind::Spell && item.way == WAY_BESTOW {
        cx.st().update_obj(item.object, |o| o.copy = None);
        fizzles = false;
    }

    match item.kind {
        StackKind::Spell => {
            let def = cx.card(face(cx.s(), item.object));
            if fizzles {
                if let Some(at) = on_stack(cx) {
                    cx.st().stack_remove(at);
                }
                leave_stack(cx, std::slice::from_ref(&item), ZoneReason::Counter);
                return Ok(Flow::Done);
            }
            // Gift (CR 702.174b, 702.174j): the chosen opponent gets it before any of the spell's other effects, and
            // only as the spell resolves. A spell that was countered or fizzled gave nothing. Whoever left the game
            // gets nothing (CR 800.4a). A permanent spell gives it when it enters (`cantrip_mechanics::gift`).
            if let (Some(gift), Some(to)) = (def.gift, item.gift)
                && (cx.chars(item.object).is(CardType::Instant) || cx.chars(item.object).is(CardType::Sorcery))
            {
                if !cx.s().player(to).lost {
                    (gift.give)(cx, to)?;
                }
                cx.emit(Event::GaveGift { player: item.controller, to, gift: gift.kind });
            }
            let fused = if item.way == WAY_FUSED && def.fuse { def.split } else { None };
            if def.spell.is_some() || fused.is_some() {
                match (def.modal, def.spell, fused) {
                    (_, _, Some((left, right))) => run_fused(cx, &item, left, right)?,
                    (Some(modal), _, None) => run_modes(cx, &item, modal)?,
                    (None, Some(spell), None) => (spell.effect)(cx, &item)?,
                    (None, None, None) => {}
                };
                // Paradigm's spell ability that makes its delayed trigger follows the card's own text (CR 702.192a); "exile
                // this spell" is `leave_stack`'s.
                if let Some(paradigm) = def.paradigm {
                    paradigm(cx, &item)?;
                }
                // The spell abilities spliced onto it follow its own (CR 702.47c).
                for spliced in spliced(cx.s(), &item) {
                    if let Some(added) = cx.card(spliced).spell {
                        (added.effect)(cx, &item)?;
                    }
                }
                // CR 608.2m: a spell that put itself somewhere as an instruction (Time Spiral exiles itself, Green Sun's
                // Zenith shuffles itself into its library) has left the stack already, and goes nowhere more.
                if let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == item.object) {
                    cx.st().stack_remove(at);
                    leave_stack(cx, std::slice::from_ref(&item), ZoneReason::Resolve);
                }
            } else if cx.s().obj(item.object).card_copy {
                // A copy of a permanent spell becomes a token as it resolves (CR 707.10f).
                if let Some(at) = on_stack(cx) {
                    ops::resolve_copy_as_token(cx, &item, at)?;
                }
            } else {
                // A permanent spell whose sneak cost was paid enters tapped and attacking what the creature returned to
                // pay it was attacking (CR 702.190b), which the cast kept in `subject`.
                let attacking = item.attacking;
                // A face-down spell (CR 708.4) enters with the same characteristics it had as a spell (CR 702.37c,
                // 702.168b): it is a face-down permanent as it arrives, so what replaces its entering judges that, and its
                // card's own "enters" replacements do not apply (CR 708.3), as for a card put onto the battlefield face down.
                if let Some(values) = cx.s().obj(item.object).face_down_values() {
                    cx.st().update_obj(item.object, |o| o.copy = Some(Copiable::of(values)));
                }
                // A permanent spell becomes a permanent (CR 608.3), its entering replacements applied first, while it
                // is still on the stack (CR 614.12).
                let mut entering = Entering {
                    object: item.object,
                    controller: item.controller,
                    tapped: attacking,
                    counters: [0; CounterKind::COUNT],
                    // A bestowed spell is the Aura it was cast as, until it ceases to be bestowed (CR 702.103b); a prototyped
                    // spell becomes a permanent with the same prototyped characteristics (CR 718.3b), which are its copiable
                    // values (CR 718.2a) until it leaves the battlefield (CR 718.4).
                    copy: if item.way == WAY_BESTOW || item.way == WAY_PROTOTYPE { cx.s().obj(item.object).copy } else { None },
                    prepared: false,
                    chosen_color: None,
                    chosen_subtype: None,
                    chosen_player: None,
                    chosen_card: None,
                    chosen_option: None,
                    chosen_number: None,
                    chosen_object: None,
                    // An Aura spell enters attached to what it targeted (CR 303.4f, 608.3b).
                    attach: if def.enchant.is_some() { item.target(0).object() } else { None },
                    attach_player: if def.enchant.is_some() { item.target(0).player() } else { None },
                    x: item.x,
                    // Cast as its back face, or transformed, it enters with that face up (CR 712.8c, 712.8f, 712.13).
                    transformed: item.way == WAY_BACK_FACE || item.way == WAY_TRANSFORMED,
                    protector: None,
                    instead: None,
                    becomes: None,
                    own: None,
                };
                ops::prepare_entry(cx, &mut entering)?;
                if let Some(at) = on_stack(cx) {
                    cx.st().stack_remove(at);
                }
                // CR 712.13a: one that would enter with an instant or sorcery face up - turned over as it enters, by its
                // daybound at night (CR 702.145b) - doesn't enter, and is put into its owner's graveyard instead (CR 304.4,
                // 307.4).
                if ops::instant_or_sorcery_entering(cx, item.object, entering.transformed) {
                    leave_stack(cx, std::slice::from_ref(&item), ZoneReason::Resolve);
                    let card = cx.s().obj(item.object).card;
                    cx.record(Event::Resolved { source: item.object, ability: AbilityRef::spell(card) });
                    return Ok(Flow::Done);
                }
                ops::move_together(
                    cx,
                    &[ops::Moving { entering: Some(entering), ..ops::Moving::to(item.object, Zone::BATTLEFIELD) }],
                    ZoneReason::Resolve,
                );
                if let Some(new) = cx.s().obj(item.object).became
                    && cx.s().obj(new).zone.is(ZoneKind::Battlefield)
                {
                    ops::carry_effects(cx, item.object, new);
                    if attacking {
                        ops::enter_attacking(cx, &[new], item.subject);
                    }
                    // "If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes
                    // sacrifices it at the beginning of the next cleanup step": a static ability that functions on the stack
                    // creates the rules' delayed triggered ability as the permanent is put onto the battlefield (CR 608.3g,
                    // 603.7a), about that permanent, controlled by the spell's controller (CR 603.7d) - if the spell was cast
                    // at such a time by its own permission (`cast`).
                    if def.untimely_cleanup_rider
                        && cx.s().obj(item.object).marks.untimely_by_own_flash
                    {
                        let since = cx.g.log.len() as u32;
                        let ability = AbilityRef { card: cx.cards().intrinsic, index: cantrip_api::intrinsic::UNTIMELY_SACRIFICE };
                        cx.st().delayed_push(cantrip_state::Delayed {
                            when: DelayedWhen::Step { step: Step::Cleanup, on_turn_of: None },
                            source: item.object,
                            controller: item.controller,
                            ability,
                            subject: Target::Object(new),
                            members: cantrip_state::Span::default(),
                            since,
                            acquired: Acquired::OWN,
                        });
                    }
                }
                // A creature spell paid for with mana that gives it haste has it until end of turn (CR 106.6).
                if cx.s().obj(item.object).marks.gains_haste
                    && let Some(new) = cx.s().obj(item.object).became
                    && cx.s().obj(new).zone.is(ZoneKind::Battlefield)
                {
                    Game::add_effect(cx, new, new, EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])), Duration::EndOfTurn);
                }
                // A granted way's "if you do, it gains ...": the permanent has the ability from then on.
                if let Some(ability) = cx.s().obj(item.object).marks.gains
                    && let Some(new) = cx.s().obj(item.object).became
                    && cx.s().obj(new).zone.is(ZoneKind::Battlefield)
                {
                    Game::add_effect(cx, new, new, EffectKind::AddAbility(ability), Duration::Indefinite);
                }
            }
            // It has resolved (CR 608.2, 608.3), for the history: paradigm's "the first time a spell you control with this
            // spell's name has resolved this game" (CR 702.192a).
            let card = cx.s().obj(item.object).card;
            cx.record(Event::Resolved { source: item.object, ability: AbilityRef::spell(card) });
        }
        StackKind::Activated | StackKind::Triggered => {
            // An intervening "if" no longer true: the ability does nothing (CR 603.4).
            let holds = item.kind != StackKind::Triggered || crate::settle::still_holds(cx, &item);
            if !fizzles && holds {
                match cx.cards().ability(item.ability) {
                    AbilityDef::Activated(ActivatedDef { modal: Some(modal), .. }) => run_modes(cx, &item, modal)?,
                    AbilityDef::Activated(a) => (a.effect)(cx, &item)?,
                    AbilityDef::Triggered(TriggeredDef { modal: Some(modal), .. }) => run_modes(cx, &item, modal)?,
                    AbilityDef::Triggered(t) => (t.effect)(cx, &item)?,
                    AbilityDef::Static(_) => unreachable!("a static ability is never on the stack"),
                }
                // It has resolved (CR 608.2m: "the first time this ability has resolved this turn").
                cx.record(Event::Resolved { source: item.object, ability: item.ability });
            }
            // It leaves the stack as it finishes resolving (CR 608.2n), found by its serial: what it did may have taken an
            // item below it off the stack (Ertai Resurrected countering an ability), moving it down. An ability that ended
            // the turn has left the stack already, with everything else on it (CR 724.1b).
            if let Some(now) = on_stack(cx) {
                cx.st().stack_remove(now);
            }
        }
    }
    Ok(Flow::Done)
}

/// Counters a spell on the stack (CR 701.6): it leaves the stack for its owner's graveyard, or exile.
/// Counters a spell and puts it somewhere other than its owner's graveyard (Remand's hand, Memory Lapse's library
/// top: CR 701.6a, the move is part of the countering). A spell that can't be countered stays.
pub fn counter_to(cx: &mut Cx, spell: ObjectId, to: CounterTo) {
    let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == spell) else {
        return;
    };
    if !can_be_countered(cx, spell) {
        return;
    }
    cx.emit(Event::Countered { spell });
    send_spell(cx, at, to, ZoneReason::Counter);
}

/// A spell leaves the stack without being countered (CR 400.3): "return target spell to its owner's hand" (Sink
/// into Stupor), "its owner puts it on their choice of the top or bottom of their library" (Subtlety). Nothing says
/// "counter", so it works on a spell that can't be countered and no one sees it countered.
pub fn remove_spell(cx: &mut Cx, spell: ObjectId, to: CounterTo) {
    if let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == spell) {
        send_spell(cx, at, to, ZoneReason::Other);
    }
}

/// Takes the spell at `at` off the stack and into the zone `to` names, its owner's.
fn send_spell(cx: &mut Cx, at: usize, to: CounterTo, reason: ZoneReason) {
    let item = cx.st().stack_remove(at);
    let obj = *cx.s().obj(item.object);
    if obj.card_copy {
        ops::cease_to_exist(cx, Zone::STACK, item.object);
        return;
    }
    let zone = match to {
        CounterTo::Hand => Zone::hand(obj.owner),
        CounterTo::LibraryTop | CounterTo::LibraryBottom => Zone::library(obj.owner),
        CounterTo::Exile => Zone::exile(obj.owner),
    };
    // Flashback's "instead of putting it anywhere else" (CR 702.34a) replaces a move to a hand or a library too, as the
    // replacement effects of the spell's own do (`its_own_exile`); a cast's "if that spell would be put into your
    // graveyard" does not.
    let new = ops::move_together(cx, &[its_own_exile(cx, &item, ops::Moving::to(item.object, zone), reason)], reason)[0];
    if to == CounterTo::LibraryBottom && cx.s().obj(new).zone.is(ZoneKind::Library) {
        cx.st().zone_remove(zone, new);
        cx.st().zone_push(zone, new);
    }
}

pub fn counter(cx: &mut Cx, spell: ObjectId) {
    if let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == spell) {
        counter_at(cx, &[at]);
    }
}

/// Counters a spell or an ability on the stack (CR 701.6): `hint` is where the item was when it was announced, tried
/// first so that two items of one source are told apart; a countered ability just ceases to exist.
pub fn counter_item(cx: &mut Cx, object: ObjectId, hint: usize) {
    counter_items(cx, &[(object, hint)]);
}

/// Counters spells and abilities on the stack with one instruction (CR 701.6, 608.2c), each found as `counter_item` finds it:
/// "counter all spells your opponents control and all abilities your opponents control" (Glen Elendra's Answer), Counterflux's
/// overloaded "counter each spell you don't control". Returns how many were countered.
pub fn counter_items(cx: &mut Cx, items: &[(ObjectId, usize)]) -> u32 {
    let stack = &cx.s().stack;
    let mut at: Vec<usize> = Vec::with_capacity(items.len());
    for &(object, hint) in items {
        let found = if stack.get(hint).is_some_and(|i| i.object == object) { Some(hint) } else { stack.iter().rposition(|i| i.object == object) };
        if let Some(found) = found.filter(|f| !at.contains(f)) {
            at.push(found);
        }
    }
    counter_at(cx, &at)
}

/// Whether a spell can be countered (CR 101.2, 701.6a): not one that "can't be countered", printed or given (Frenzied Baloth),
/// nor the one a "the next spell you cast this turn can't be countered" effect marked as it was cast (Mistrise Village).
fn can_be_countered(cx: &Cx, spell: ObjectId) -> bool {
    !cx.chars(spell).keywords.has(Keyword::CantBeCountered) && !cx.s().obj(spell).marks.cant_be_countered
}

/// Counters the items at `at` on the stack together (CR 701.6, 608.2c): what can't be countered stays (CR 101.2, 701.6a); a
/// countered ability ceases to exist; the countered spells are announced as one batch and leave the stack for their owners'
/// graveyards in one move (`leave_stack`), so an ability that triggers on "one or more" of either triggers once (CR 603.2c).
/// Returns how many were countered.
fn counter_at(cx: &mut Cx, at: &[usize]) -> u32 {
    let mut at: Vec<usize> = at
        .iter()
        .copied()
        .filter(|&a| {
            let item = cx.s().stack[a];
            item.kind != StackKind::Spell || can_be_countered(cx, item.object)
        })
        .collect();
    // Announced before they leave the stack: what triggers on a spell being countered looks back on the game as it was just
    // before (CR 603.10e), the spells still on the stack.
    let countered: Vec<Event> =
        at.iter().map(|&a| cx.s().stack[a]).filter(|i| i.kind == StackKind::Spell).map(|i| Event::Countered { spell: i.object }).collect();
    match countered.len() {
        0 => {}
        1 => cx.emit(countered[0]),
        _ => cx.emit_batch(&countered, None),
    }
    // Taken off from the top down, so the places below stay where they were found.
    at.sort_unstable_by(|a, b| b.cmp(a));
    let leaving: Vec<StackItem> = at.iter().map(|&a| cx.st().stack_remove(a)).collect();
    leave_stack(cx, &leaving, ZoneReason::Counter);
    leaving.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
        None
    }

    fn nothing(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
        Ok(())
    }

    fn anything(_: &dyn Query, _: &Asker, _: Target) -> bool {
        true
    }

    /// Thorin, Mountain-king's shape ("attach any number of target Equipment you control to target creature you control"): a
    /// slot of any number of targets before a slot of one, then an "up to one" slot.
    static SPECS: [TargetSpec; 3] = [
        up_to(TargetSpec::ANY_NUMBER, anything).labelled("any number"),
        target(anything).labelled("one"),
        up_to_one(anything).labelled("up to one"),
    ];
    static CARD: CardDef = CardDef {
        name: "Slot Order Test",
        abilities: &[triggered(when(EventMask::ZONE_CHANGE, never), nothing, "").targets(&SPECS)],
        ..CardDef::BLANK
    };

    fn labels(table: &CardTable, state: &GameState, item: &StackItem) -> Vec<&'static str> {
        (0..item.target_count as usize).map(|slot| slot_spec(table, state, item, slot).expect("a slot").0.label).collect()
    }

    /// CR 601.2c, 115.1d: each target is judged by the slot it was chosen for, wherever a slot of several targets stands in
    /// the list; the slot recorded with it (`StackItem::spec_of`) says which.
    #[test]
    fn a_slot_of_any_number_of_targets_may_come_before_a_slot_of_one() {
        let table = CardTable::new([&CARD]);
        let mut state = GameState::new(2, 0);
        let ability = AbilityRef { card: table.id_of(&CARD), index: 0 };
        let t = |n: u32| Target::Object(ObjectId(n));
        let item_of = |state: &mut GameState, targets: &[Target], of: &[u8]| {
            let mut item = crate::actions::stack_item(StackKind::Triggered, ObjectId(0), ability, PlayerId(0), &[]);
            state.set_item_targets(&mut item, targets, of, &[]);
            item
        };

        // Two for the first slot, one for the second, one for the third.
        let item = item_of(&mut state, &[t(1), t(2), t(3), t(4)], &[0, 0, 1, 2]);
        assert_eq!(labels(&table, &state, &item), ["any number", "any number", "one", "up to one"]);
        assert_eq!((state.spec_targets(&item, 0), state.spec_targets(&item, 1), state.spec_targets(&item, 2)), (0..2, 2..3, 3..4));

        // None for the first slot: the first target is the second slot's, not the first's.
        let item = item_of(&mut state, &[t(3)], &[1]);
        assert_eq!(labels(&table, &state, &item), ["one"]);
        assert_eq!((state.spec_targets(&item, 0), state.spec_targets(&item, 1), state.spec_targets(&item, 2)), (0..0, 0..1, 1..1));

        // Three for the first slot and the "up to one" left unchosen at the end, where no place is kept for it.
        let item = item_of(&mut state, &[t(1), t(2), t(5), t(3)], &[0, 0, 0, 1]);
        assert_eq!(labels(&table, &state, &item), ["any number", "any number", "any number", "one"]);
        assert_eq!(state.spec_targets(&item, 2), 4..4);
    }
}
