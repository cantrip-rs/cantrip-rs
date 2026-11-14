//! The operations everything else is built from: zone changes, draws, damage, life, counters. Each is one rules
//! procedure, each writes through the journal, and each announces what it did.

use cantrip_api::*;
use cantrip_state::{ChoiceRecord, Choices, Entered, Kept, PendingTrigger};

use crate::layers::{self, live};
use crate::{Cx, R, counters, replace, settle};

/// A player commits a crime (CR 700.13) as they cast a spell, activate an ability or put a triggered ability on the stack
/// that targets at least one opponent; at least one permanent, spell or ability an opponent controls; and/or at least one
/// card in an opponent's graveyard: called by those three procedures, once each with the targets chosen, after their
/// `BecameTarget` events. One crime is recorded however many targets qualify. Each target is judged as it is now, as the
/// object becomes cast, activated or put on the stack; a permanent a cost took away is judged by its row as it left. A
/// card in exile, a hand or a library is no crime, nor is anything of the player's own.
pub(crate) fn commit_crime(cx: &mut Cx, player: PlayerId, by: ObjectId, targets: &[Target]) {
    let s = cx.s();
    let opponent = |p: PlayerId| p != player && s.opponents(player).any(|o| o == p);
    let crime = targets.iter().any(|&t| match t {
        Target::Player(p) => opponent(p),
        Target::Object(id) => {
            let o = s.obj(id);
            match o.zone.kind() {
                ZoneKind::Battlefield | ZoneKind::Stack => opponent(o.controller),
                ZoneKind::Graveyard => opponent(o.zone.player),
                _ => false,
            }
        }
        Target::Ability(serial) => s.stack.iter().any(|i| i.serial == serial && i.kind != StackKind::Spell && opponent(i.controller)),
        Target::None => false,
    });
    if crime {
        cx.emit(Event::CrimeCommitted { player, by });
    }
}

/// Each object or player among `targets` becomes a target of `by` (CR 601.2c, 602.2b, 603.3d), once: one chosen for several
/// instances of the word "target" (CR 115.3) becomes the target of that spell or ability once, not once per instance, so a
/// ward or "whenever this becomes the target" ability triggers once for it. Called as a spell is cast, an ability activated
/// or put on the stack, a copy put on the stack (CR 707.10) and targets changed or chosen anew (CR 115.7), with that
/// procedure's targets; an empty slot (`Target::None`) is no target.
pub(crate) fn became_targets(cx: &mut Cx, targets: impl IntoIterator<Item = Target>, by: ObjectId, controller: PlayerId, ability: bool) {
    // The first few in place, any past them in a list made only when there are that many.
    let mut seen = [Target::None; cantrip_state::INLINE_TARGETS];
    let mut more: Vec<Target> = Vec::new();
    let mut n = 0;
    for target in targets {
        if target == Target::None || seen[..n].contains(&target) || more.contains(&target) {
            continue;
        }
        if n < seen.len() {
            seen[n] = target;
            n += 1;
        } else {
            more.push(target);
        }
        cx.emit(Event::BecameTarget { target, by, controller, ability });
    }
}

/// One object about to move, and how it arrives if it is entering the battlefield.
#[derive(Clone, Copy, Default)]
pub enum PutBy {
    /// The player performing this action or resolving effect, if any.
    #[default]
    Current,
    Player(PlayerId),
    /// A rules-created action without a player; never inherit an enclosing spell's player.
    Rules,
}

impl PutBy {
    fn player(self, cx: &Cx) -> Option<PlayerId> {
        match self { Self::Current => cx.acting_player(), Self::Player(p) => Some(p), Self::Rules => None }
    }
}

#[derive(Clone, Copy)]
pub struct Moving {
    pub id: ObjectId,
    pub to: Zone,
    pub entering: Option<Entering>,
    pub put_by: PutBy,
    /// Why this move happens when it is not why the rest of the batch does: a sacrifice and a return made at once (CR 101.4).
    pub reason: Option<ZoneReason>,
    /// The instruction moving it is the one that reveals it ("reveal it and put it into your hand", CR 701.20a): it is a
    /// card all players can see as it moves (`seen_by_all`). Set by the reveal-and-move verbs, never by a move that is not
    /// about the revealed card (a later draw).
    pub revealed: bool,
    /// It is discarded to pay a cost that specifies a characteristic of it (`ZoneChange::pays_characteristic`, CR 701.9c).
    pub pays_characteristic: bool,
    /// A spell leaving the stack, its own replacements of that move (`ZoneChange::flashback`, `ZoneChange::exile_instead`):
    /// set by the spell's leaving (`resolve::leave_stack`, `resolve::send_spell`).
    pub flashback: bool,
    pub exile_instead: ExileInstead,
    /// A spell resolving, the rules' own "instead of putting it into its owner's graveyard as it resolves" that it is subject
    /// to (`ZoneChange::adventure`, `omen`, `rebound`): set by `resolve::leave_stack`.
    pub adventure: bool,
    pub omen: bool,
    pub rebound: bool,
    /// The move is a destruction a regeneration shield may replace (`ZoneChange::destroy`): lethal damage's (CR 704.5g-h).
    /// A move made for `ZoneReason::Destroy` is one whatever this says.
    pub destroy: bool,
    /// Set only by the unattached-Aura SBA (CR 704.5m), for the old enchanted permanent that already left (400.7f).
    pub aura_host_left: Option<ObjectId>,
}

impl Moving {
    pub fn to(id: ObjectId, to: Zone) -> Moving {
        Moving { id, to, entering: None, put_by: PutBy::Current, reason: None, revealed: false, pays_characteristic: false, flashback: false, exile_instead: ExileInstead::Never, adventure: false, omen: false, rebound: false, destroy: false, aura_host_left: None }
    }
}

/// One move of a `move_together`, worked out before any of them is made (CR 614.12, 101.4): where it goes once every
/// replacement has applied, the new object it arrives as, and the permanent's last-known information if it is leaving.
struct Plan {
    m: Moving,
    reason: ZoneReason,
    old: Obj,
    from: Zone,
    to: Zone,
    requested_to: Zone,
    change: ZoneChange,
    madness: bool,
    entering: bool,
    new: Obj,
    /// The counters it arrives with (CR 122.6), once the replacements have applied, and the player putting them.
    counts: [i32; CounterKind::COUNT],
    counts_by: PlayerId,
    last_known: LastKnown,
    /// Every player could see it as it moved (`seen_by_all`), for a move into a hand or a library (CR 603.10a).
    seen_by_all: bool,
    /// A merged permanent leaving the battlefield (CR 730.3): each of its components, and where its card components go if it
    /// is a token a replacement effect for cards alone would have moved elsewhere (CR 730.3e).
    components: Option<(Vec<crate::merge::Component>, Option<(Zone, Option<Linked>)>)>,
}

/// Whether every player can see `id` as it moves now (CR 603.10a: "an object that all players can see"), the instruction
/// moving it revealing it if `revealed_by_move`: "reveal it and put it into your hand" (`Moving::revealed`, the
/// reveal-and-move verbs), "reveal it and shuffle it into its owner's library instead" (`ZoneChange::reveal`). An object in
/// a public zone can be seen by all (CR 400.2), and one exiled face up, or face down where every player in the game may
/// look at it (CR 406.3). A card in a hand or a library can be only while a reveal of it is in effect (CR 701.20a) - not
/// because it was revealed once, which ended (the history's `Event::Revealed` is what was shown, not what is). An effect's
/// reveal lasts only through the instruction about the card it revealed: the move that instruction makes is marked, and a
/// later one in the same effect (a draw that takes it) is not. Otherwise: the cost or announcement of a spell or activated ability
/// still on the stack revealed it (behold, splice); or its revealing triggered an ability that has not yet left the stack
/// (miracle, CR 702.94a). The card an ability is activated from in a hidden zone is revealed by the activation alone, one
/// event (CR 602.2a, `Event::Revealed`), and is not kept revealed while the ability is on the stack (RX-7,
/// `docs/rules-exceptions.md`). A continuous library-top reveal is derived through
/// `library_visibility`, including the announcement barrier of CR 401.5.
pub(crate) fn seen_by_all(cx: &Cx, id: ObjectId, revealed_by_move: bool) -> bool {
    seen_by_all_in(cx.g, id, revealed_by_move)
}

/// The cards in a hand or a library that a reveal in effect shows every player now (`seen_by_all`): those the cost or
/// announcement of a spell or an activated ability still on the stack revealed (behold, splice), and those whose revealing
/// triggered an ability not yet off the stack (CR 701.20a). Not the card an ability was activated from there: its reveal is
/// the activation's one event (CR 602.2a, RX-7). What a seat may see
/// (`cantrip_engine::visibility::observable`) holds them.
pub(crate) fn revealed_in_hidden_zones(g: &crate::Game) -> Vec<ObjectId> {
    let s = &g.state;
    let mut candidates: Vec<ObjectId> = Vec::new();
    for item in &s.stack {
        candidates.push(item.object);
        candidates.extend_from_slice(s.members(item.members));
    }
    candidates.extend(s.triggers.iter().map(|t| t.source));
    let mut out: Vec<ObjectId> = Vec::new();
    for id in candidates {
        let zone = s.obj(id).zone;
        if (zone.is(ZoneKind::Hand) || zone.is(ZoneKind::Library)) && !out.contains(&id) && seen_by_all_in(g, id, false) {
            out.push(id);
        }
    }
    out
}

/// `seen_by_all`, of a game not running a frame (`revealed_in_hidden_zones`).
fn seen_by_all_in(g: &crate::Game, id: ObjectId, revealed_by_move: bool) -> bool {
    let s = &g.state;
    let o = s.obj(id);
    match o.zone.kind() {
        ZoneKind::Battlefield | ZoneKind::Graveyard | ZoneKind::Stack | ZoneKind::Command => true,
        ZoneKind::Exile => !o.face_down || s.apnap().all(|p| o.seen_by & (1 << p.0) != 0),
        ZoneKind::Hand | ZoneKind::Library => {
            revealed_by_move
                || (o.zone.is(ZoneKind::Library)
                    && crate::library_visibility::reveals(g, o.owner)
                    && crate::library_visibility::top(g, o.owner) == Some(id))
                || (o.zone.is(ZoneKind::Hand) && g.revealed_hands() & (1 << o.zone.player.0) != 0)
                || s.stack.iter().any(|i| match i.kind {
                    // The cards its cost or announcement revealed are among the objects its cost chose (`StackItem::members`):
                    // a spell's spliced cards and those it beheld (CR 702.47a, 701.4a), revealed from its announcement
                    // until it leaves the stack (CR 701.20a) - while it is being cast (`being_cast`) and once it has become
                    // cast (`cast_known`, CR 601.2i). A copy of a spell or an ability was neither (CR 707.10), so it revealed
                    // nothing; a cast copy of a card was cast (CR 707.12). An activated ability's item is put on the stack
                    // as it is announced (CR 602.2a), and only a copy of one is `copied`. Its own source, activated from a
                    // hidden zone, is not among them: that reveal is the activation's event alone (RX-7).
                    StackKind::Spell => s.members(i.members).contains(&id) && (i.cast_known != u32::MAX || being_cast(g, i)),
                    StackKind::Activated => !i.copied && s.members(i.members).contains(&id),
                    // A copy (Strionic Resonator) was never triggered by the reveal (CR 707.10): only "that triggered
                    // ability" keeps the card revealed.
                    StackKind::Triggered => !i.copied && triggered_by_revealing(i.ability, i.object, id),
                })
                || s.triggers.iter().any(|t| triggered_by_revealing(t.ability, t.source, id))
        }
        // A card outside the game (CR 400.11) is seen by all only as the instruction bringing it in reveals it.
        ZoneKind::Sideboard => revealed_by_move,
        _ => false,
    }
}

/// Whether the spell `item` is being cast now, announced (CR 601.2a) and not yet cast (CR 601.2i): its card was moved to
/// the stack to be cast (`ZoneReason::Cast`) in the run of records just before the item was put there (`StackItem::since`),
/// which a copy of a spell, created on the stack (CR 707.10), never was. A cast given up is undone, item and all (CR 733.1).
fn being_cast(g: &crate::Game, item: &StackItem) -> bool {
    let log = &g.log;
    let end = (item.since as usize).min(log.len());
    let Some(last) = end.checked_sub(1) else { return false };
    let cause = log[last].cause;
    log[..end].iter().rev().take_while(|r| r.cause == cause).any(|r| {
        matches!(r.event, Event::ZoneChange { new, to, reason: ZoneReason::Cast, .. } if new == item.object && to.is(ZoneKind::Stack))
    })
}

/// Whether `ability`, of `source`, was triggered by revealing `card`, which stays revealed until the ability leaves the
/// stack, or isn't put on it the next time a player would receive priority (CR 701.20a). The only ability the engine has
/// that a reveal triggers is miracle's, by revealing its own card (CR 702.94a): no card's triggered ability listens for
/// `Event::Revealed` (`cantrip-cards/tests/no_reveal_triggers.rs` holds the card table to that). The first that does
/// records what it was triggered by and is asked of here.
fn triggered_by_revealing(ability: AbilityRef, source: ObjectId, card: ObjectId) -> bool {
    ability.is_miracle() && source == card
}

/// With `CANTRIP_VERIFY`: a card the resolving spell or ability revealed, put into a hand or a library by that resolution
/// with a move that is not marked revealed (`Moving::revealed`) and is no draw, is a reveal-and-move verb forgotten -
/// `reveal` then `return_to_hand` where the text is "reveal it and put it into your hand" (`Game::return_to_hand_revealed`)
/// - which would judge a card all players can see as hidden (CR 603.10a, 701.20a). Its history is the run of records the
/// resolution made (`Cx::cause`), an ability's earlier resolution cut off at its `Event::Resolved`.
pub(crate) fn verify_reveal_carried(cx: &Cx, id: ObjectId) {
    let Some(&cantrip_state::Frame { kind: cantrip_state::FrameKind::Resolve { .. }, .. }) = cx.s().agenda.get(cx.frame) else { return };
    let cause = cx.cause();
    for r in cx.g.log.iter().rev() {
        if r.cause != cause {
            return;
        }
        match r.event {
            Event::Revealed { card, .. } if card == id => panic!(
                "{} ({id:?}) was revealed by the effect putting it into a hand or a library, with a move not marked revealed: the \
                 instruction that reveals it moves it with a reveal-and-move verb (`Game::return_to_hand_revealed`, \
                 `move_together_revealed`, `bring_into_hand_revealed`, `discard_revealed`; CR 701.20a, 603.10a)",
                cx.g.name(id)
            ),
            Event::Resolved { source, ability } if matches!(cause, Cause::Ability { source: s, ability: a, .. } if source == s && ability == a) => return,
            _ => {}
        }
    }
}

/// Moves objects at once (CR 400.7: each arrives as a new object). Leaves-the-battlefield abilities are judged on
/// the board as it was before any of them moved (CR 603.10a); everything else on the board after. A library is
/// entered on top; cards put into a graveyard together are in the order their owner arranges (CR 404.3,
/// `arrange_graveyards`). Returns the new objects, in order.
pub fn move_together(cx: &mut Cx, moves: &[Moving], reason: ZoneReason) -> Vec<ObjectId> {
    move_together_seen(cx, moves, reason).0
}

/// `move_together`, which also returns the objects, by their old ids, put into a hand or a library that every player could
/// see as they moved (`seen_by_all`, decided as the moves were made): the rest put there were not revealed.
fn move_together_seen(cx: &mut Cx, moves: &[Moving], reason: ZoneReason) -> (Vec<ObjectId>, Vec<ObjectId>) {
    move_together_entering(cx, moves, reason, None)
}

/// An instruction can name the defender of the arriving creatures (CR 508.4).
fn move_together_entering(cx: &mut Cx, moves: &[Moving], reason: ZoneReason, attacking: Option<Target>) -> (Vec<ObjectId>, Vec<ObjectId>) {
    // A simultaneous move, including its replacement bookkeeping, is one rules
    // instruction (CR 614.6, 603.2c), not one instruction per saved permanent.
    let (created, seen, _) = cx.instruction(|cx| move_together_entering_event(cx, moves, reason, attacking, &[]), |_| true);
    (created, seen)
}

/// The moves of `move_together_entering`, and with them the changes of life `life` of the same simultaneous action
/// (`together`, CR 101.4), each its own occurrence: replaced and judged allowed against the game as it is before any of the
/// moves or changes is made (CR 616.1, `prepare_life_changes`), applied with the moves and announced in their batch (CR
/// 603.2c). The new objects, those seen by all as they moved, and the life lost.
fn move_together_entering_event(
    cx: &mut Cx,
    moves: &[Moving],
    reason: ZoneReason,
    attacking: Option<Target>,
    life: &[(PlayerId, i64)],
) -> (Vec<ObjectId>, Vec<ObjectId>, i32) {
    // An object moves once however many times the batch names it: a permanent a spell targets twice ("destroy target
    // artifact and target creature" at an artifact creature, CR 115.3) is destroyed once (CR 400.7). The first move kept.
    // And an object that has changed zones or ceased to exist since it was named is not moved at all: it became a new
    // object with no memory of the old one, and an instruction to move the old one does nothing (CR 400.7) - every verb
    // that moves objects comes here, so none can make a second copy of a card from a stale id.
    let distinct;
    let kept = |i: usize, m: &Moving| live(cx.s(), m.id) && !moves[..i].iter().any(|n| n.id == m.id);
    let moves = if moves.iter().enumerate().any(|(i, m)| !kept(i, m)) {
        distinct = moves.iter().enumerate().filter(|&(i, m)| kept(i, m)).map(|(_, m)| *m).collect::<Vec<_>>();
        &distinct[..]
    } else {
        moves
    };
    let before: Vec<ObjectId> = cx.s().battlefield().to_vec();
    // A permanent leaving, phased in or out (CR 702.26d: a phased-out permanent is still on the battlefield, and leaves it when
    // its owner leaves the game, CR 702.26n): what it held "until" it left comes back (`return_until_left`).
    let left_battlefield = moves.iter().any(|m| matches!(cx.s().obj(m.id).zone.kind(), ZoneKind::Battlefield | ZoneKind::PhasedOut));
    // The permanents that could look back, as they are just before the moves (CR 603.10a): only for moves a look-back
    // ability can trigger on - leaving the battlefield or a graveyard, or a public object put into a hand or library (one
    // from a hand or a library, public while a reveal of it is in effect, is known once its move is planned, below).
    let looks_back = moves.iter().any(|m| {
        let from = cx.s().obj(m.id).zone;
        from.is(ZoneKind::Battlefield)
            || from.is(ZoneKind::Graveyard)
            || ((m.to.is(ZoneKind::Hand) || m.to.is(ZoneKind::Library)) && !from.is(ZoneKind::Hand) && !from.is(ZoneKind::Library))
    });
    let then = if looks_back { crate::settle::Then::now(cx) } else { crate::settle::Then::none() };
    let mut events = Vec::with_capacity(moves.len());
    let mut shuffled = Vec::new();
    let mut created = Vec::with_capacity(moves.len());
    // CR 701.3d: every attachment these moves end, read before any is made: an attachment leaving the battlefield,
    // and every one attached to something leaving. Each pair once.
    for &a in &before {
        let Some(host) = cx.s().obj(a).attached_to else { continue };
        let leaving = |id: ObjectId| moves.iter().any(|m| m.id == id);
        if live(cx.s(), host)
            && cx.s().obj(host).zone.is(ZoneKind::Battlefield)
            && (leaving(a) || leaving(host))
            && !events.iter().any(|e| matches!(e, Event::Unattached { attachment, .. } if *attachment == a))
        {
            events.push(Event::Unattached { attachment: a, from: host });
        }
    }
    // CR 614.12, 101.4: what replaces each object's move, and what it arrives with, is worked out against the game as it
    // is before any of them moves - so objects entering together do not see one another on the battlefield (a creature
    // entering with Gev gets none of his counters) and those leaving together still see one another - and only then do
    // they all move. The last-known information of each permanent leaving is taken before any of them leaves (CR 603.10a).
    crate::hand_visibility::sync(cx);
    let hands_before = crate::hand_visibility::mask(cx, cx.cards());
    let mut plans: Vec<Plan> = Vec::with_capacity(moves.len());
    let mut regenerating: Vec<(ObjectId, bool)> = Vec::new();
    let mut shielded: Vec<ObjectId> = Vec::new();
    let mut replaced: Vec<ObjectId> = Vec::new();
    let together = moves.iter().any(|m| m.entering.is_some()) && moves.iter().any(|m| cx.s().obj(m.id).zone.is(ZoneKind::Battlefield));
    let leaving: Vec<ObjectId> =
        if together { moves.iter().map(|m| m.id).filter(|&id| cx.s().obj(id).zone.is(ZoneKind::Battlefield)).collect() } else { Vec::new() };
    // Saved and put back as this move is done, so a move made while it is worked out (a replacement's) leaves it as it was.
    let outer_leaving = std::mem::replace(&mut *cx.s().leaving_together.borrow_mut(), leaving);
    for m in moves {
        let reason = m.reason.unwrap_or(reason);
        let old = *cx.s().obj(m.id);
        let from = old.zone;
        let to = m.to;
        let mut m = *m;
        // A spell moved off the stack by any instruction carries its own replacements of that move (`resolve::its_own_exile`):
        // flashback's "instead of putting it anywhere else" (CR 702.34a) and a cast's "exile it instead" (CR 614.1a).
        if from.is(ZoneKind::Stack)
            && let Some(item) = cx.s().stack.iter().find(|i| i.kind == StackKind::Spell && i.object == m.id).copied()
        {
            m = crate::resolve::its_own_exile(cx, &item, m, reason);
        }
        // A card goes to its owner's own hand, library or graveyard, whoever puts it there (CR 400.3).
        let to = match to.kind() {
            ZoneKind::Library | ZoneKind::Hand | ZoneKind::Graveyard | ZoneKind::Exile => Zone::new(to.kind(), old.owner),
            _ => to,
        };
        // Replacement effects of the permanents in play, and of the card itself (CR 614, 616): they change where it goes,
        // how it arrives, or what it arrives with.
        let entry = m.entering.filter(|_| to.is(ZoneKind::Battlefield));
        let proposed = ZoneChange {
            object: m.id,
            from,
            to,
            reason,
            controller: entry.map_or(old.controller, |e| e.controller),
            put_by: to.is(ZoneKind::Battlefield).then(|| m.put_by.player(cx)).flatten(),
            tapped: entry.is_some_and(|e| e.tapped),
            attacking: false,
            counters: [0; CounterKind::COUNT],
            counters_by: entry.map_or(old.controller, |e| e.controller),
            reveal: false,
            shuffle: false,
            is_card: !old.token && !old.card_copy,
            madness: false,
            pays_characteristic: m.pays_characteristic,
            exiled_with: None,
            flashback: m.flashback,
            exile_instead: m.exile_instead,
            adventure: m.adventure,
            commander_components: [ObjectId::NONE; 8],
            omen: m.omen,
            rebound: m.rebound,
            on_adventure: false,
            rebounding: false,
            destroy: m.destroy || reason == ZoneReason::Destroy,
            regenerated: false,
            regeneration_shield: false,
            shielded: false,
            replaced: false,
        };
        let requested_to = proposed.to;
        // A permanent entering is judged as it would exist on the battlefield (CR 614.12), and its own replacements are
        // ordered with the others (CR 616.1). One its own replacement sent elsewhere instead does not enter at all.
        let change = match entry {
            Some(mut e) => {
                let change = entry_replacements(cx, &mut e, proposed);
                m.entering = Some(e).filter(|_| change.to.is(ZoneKind::Battlefield) && e.instead.is_none());
                change
            }
            None => replace::zone_change(cx, proposed),
        };
        // A regeneration shield replaced the destruction (CR 701.19a, 614.8): it never happens (CR 614.6), and the permanent
        // is regenerated instead, once every move is worked out.
        if change.regenerated {
            regenerating.push((m.id, change.regeneration_shield));
            continue;
        }
        // A shield counter replaced the destruction (CR 122.1c, 614.6): it never happens, and a shield counter is removed from the
        // permanent instead, once every move is worked out.
        if change.shielded {
            shielded.push(m.id);
            continue;
        }
        if change.replaced {
            replaced.push(m.id);
            continue;
        }
        let m = &m;
        let to = change.to;
        // Madness (CR 702.35a) exiled a discarded card in place of its graveyard, one of the replacements above: it was
        // still discarded, and the linked ability is queued below, once the exiled card exists - if no later effect sent
        // it on elsewhere, so that it was "exiled this way".
        let madness = change.madness && to.is(ZoneKind::Exile);
        // A merged permanent leaving the battlefield (CR 730.3): one permanent leaves, and each of its components goes to the
        // zone it goes to (CR 730.3d) - but a token one, whose components that are cards a replacement effect for cards alone
        // moves instead (CR 730.3e), judged as the move would be of a card.
        let components = if from.is(ZoneKind::Battlefield) || from.is(ZoneKind::PhasedOut) { crate::merge::components(cx, m.id) } else { None };
        let components = components.map(|all| {
            let card_to = (old.token && all.iter().any(|c| !c.token)).then(|| {
                let as_card = replace::zone_change(cx, ZoneChange { is_card: true, ..proposed });
                if as_card.regenerated || as_card.shielded || as_card.replaced { (to, change.exiled_with) } else { (as_card.to, as_card.exiled_with) }
            });
            (all, card_to.filter(|&(z, _)| z != to))
        });
        // Whatever it leaves, it is as it last existed there (CR 400.7, 608.2h, 113.7a): what an ability of it, or one it
        // created, reads once it is gone.
        let last_known = last_known_of(cx, m.id);
        let turn = cx.s().turn.number;
        let entering = to.is(ZoneKind::Battlefield);
        let from_stack = entering && from.is(ZoneKind::Stack);
        let mut new = Obj::new(old.card, old.owner, to);
        // An Adventure its controller exiled as it resolved (CR 715.3d), if no later effect sent it elsewhere: that player may
        // play the card for as long as it stays exiled.
        if change.on_adventure && to.is(ZoneKind::Exile) {
            new.on_adventure = Some(old.controller);
        }
        new.controller = if entering || to.is(ZoneKind::Stack) { old.controller } else { old.owner };
        new.token = old.token;
        new.commander = old.commander;
        new.commander_id = old.commander_id;
        new.commander_return_pending = old.commander && (to.is(ZoneKind::Graveyard) || to.is(ZoneKind::Exile));
        new.controlled_since = turn;
        // Given as it arrives, below.
        new.timestamp = 0;
        if from_stack {
            new.kicked = old.kicked;
            new.gift = old.gift;
            new.paid = old.paid;
            new.spent = old.spent;
            new.cast_way = old.cast_way;
            new.x = old.x;
        }
        new.card_copy = old.card_copy;
        new.marks = Marks {
            // CR 709.5d: a Room enters with the unlocked designation of the half that was cast as a spell, and with
            // neither if neither was.
            doors: if from_stack && cx.cards().get(old.card).is_room() { cast_door(old.cast_way) } else { 0 },
            ..Marks::default()
        };
        if from.is(ZoneKind::Graveyard) && !old.token {
        }
        // What the replacements left of its arrival: tapped, and the counters they give it, a void counter on a card
        // exiled in place of a graveyard as much as those of a permanent (CR 122.6).
        new.tapped = entering && change.tapped;
        // A card a replacement effect exiled is exiled with that effect's source as it arrives, where the effect's object has
        // an ability that refers to the cards exiled with it (CR 607.2b, 406.6): never "until" anything.
        if to.is(ZoneKind::Exile)
            && let Some(by) = change.exiled_with
        {
            new.exiled = Some(ExileLink::new(by, from, false));
        }
        if entering {
            new.controller = change.controller;
        }
        let mut counts = change.counters;
        let mut counts_by = change.counters_by;
        if let Some(e) = m.entering.filter(|_| entering) {
            for (n, with) in counts.iter_mut().zip(e.counters) {
                *n += with;
            }
            // Under the player its replacements left it entering under ("enters under your control instead", CR 614.1c).
            new.controller = change.controller;
            new.copy = e.copy;
            keep_chosen(cx, &e)(&mut new);
            new.marks.protector = e.protector;
            new.attached_to = e.attach;
            new.attached_player = e.attach_player;
            new.transformed = e.transformed;
        }
        // CR 708.3: an object turned face down before it entered is a face-down permanent as it arrives; its card's own
        // values are what it is turned face up as (CR 708.2a).
        // It has the face-down values in layer 1b (CR 613.2b), over no copy effect yet.
        // CR 708.4: so is a card turned face down before it is put on the stack, cast by morph or disguise (CR 702.37c,
        // 702.168b): a face-down spell. A face-down spell that resolves becomes a face-down permanent with the same values
        // (CR 702.37c, 702.168b), as the resolution turned it (`resolve`); one that goes anywhere else arrives face up
        // (CR 708.9), a new object (CR 400.7).
        if (entering || to.is(ZoneKind::Stack)) && !from.is(ZoneKind::Battlefield) && let Some(c) = old.copy.filter(|c| cx.card(c.card).face_down) {
            new.copy = None;
            new.face_down = true;
            new.marks.face_down_as = Some(c.card);
        }
        if entering {
            counters::intrinsic(cx.cards(), &new, m.entering.and_then(|e| e.read_ahead), &mut counts);
            // Counters it enters with are counters put on it (CR 122.6, 614.1a), by its controller (CR 122.6a):
            // replacements that change how many apply to them, the permanent judged as it would exist on the battlefield
            // (CR 614.12).
            let controller = new.controller;
            counts_by = controller;
            let with = counts;
            counts = match m.entering {
                // Judged as the replacements left its entry: under whom and whether tapped (CR 614.12).
                Some(e) => judged_entering(cx, &Entering { counters: with, controller: change.controller, tapped: change.tapped, ..e }, with.iter().any(|&n| n > 0), |cx| {
                    counters::entering(cx, m.id, controller, controller, with)
                }),
                None => counters::entering(cx, m.id, controller, controller, with),
            };
        }
        // Whether all players can see it as it moves (CR 603.10a), decided now, before anything moves.
        let seen_by_all = (to.is(ZoneKind::Hand) || to.is(ZoneKind::Library)) && seen_by_all(cx, m.id, change.reveal || m.revealed);
        if layers::verifying()
            && (to.is(ZoneKind::Hand) || to.is(ZoneKind::Library))
            && matches!(from.kind(), ZoneKind::Hand | ZoneKind::Library | ZoneKind::Sideboard)
            && !m.revealed
            && !change.reveal
            && reason != ZoneReason::Draw
        {
            verify_reveal_carried(cx, m.id);
        }
        // The permanent's move is its topmost component's (CR 730.2a, 730.3): the card or token it has been.
        if let Some(top) = components.as_ref().and_then(|(all, _)| all.iter().find(|c| c.top)) {
            new.card = top.card;
            new.token = top.token;
            new.transformed = top.transformed;
            new.commander_id = top.commander_id;
            new.commander = top.commander_id.is_some();
            new.commander_return_pending = new.commander && (to.is(ZoneKind::Graveyard) || to.is(ZoneKind::Exile));
        }
        plans.push(Plan { m: *m, reason, old, from, to, requested_to, change, madness, entering, new, counts, counts_by, last_known, seen_by_all, components });
    }
    *cx.s().leaving_together.borrow_mut() = outer_leaving;
    // The changes of life of the same action are replaced, and judged allowed, against the same game the moves were, before
    // any of them is made (CR 101.4, 616.1): a permanent leaving in this action still stops or replaces them.
    let mut life = prepare_life_changes(cx, life);
    retain_life_changes(cx, &mut life);
    // The permanents regenerated, shielded or otherwise replaced instead of moving stay (CR 701.19a, 614.6): no attachment
    // of theirs ends for their sake.
    if !regenerating.is_empty() || !shielded.is_empty() || !replaced.is_empty() {
        let moving = |id: &ObjectId| plans.iter().any(|p| p.m.id == *id);
        events.retain(|e| !matches!(e, Event::Unattached { attachment, from } if !moving(attachment) && !moving(from)));
        regenerate_instead(cx, &regenerating);
        for &id in &shielded {
            counters::remove(cx, id, CounterKind::Shield, 1);
        }
    }
    let mut rebounds: Vec<(ObjectId, PlayerId, CardId)> = Vec::new();
    // A card from a hand, a library or outside the game that all players can see as it is put into a hand or a library is
    // looked back on too (CR 603.10a), which the moves' planned destinations and reveals say.
    let then = if !then.taken() && plans.iter().any(|p| p.seen_by_all) {
        crate::settle::Then::now(cx)
    } else {
        then
    };
    let seen: Vec<ObjectId> = plans.iter().filter(|p| p.seen_by_all).map(|p| p.m.id).collect();
    // The cards put into a graveyard, by that graveyard's player, as they arrive: their owner may arrange them (CR 404.3).
    // A token or a copy of a card ceases to exist there (CR 111.7, 704.5d-e), so it is in no order anyone can ask.
    let mut binned: Vec<(PlayerId, ObjectId)> = Vec::new();
    let mut entered: Vec<ObjectId> = Vec::new();
    // The permanents a replacement makes arrive attacking (CR 614.1c, 508.4), put into the combat once they all are.
    let mut attacking_arrivals: Vec<ObjectId> = Vec::new();
    // The new objects of a merged permanent's components other than its topmost (CR 730.3): after every other, so the objects
    // returned line up with the moves asked for, and an effect that finds what the permanent became finds them too (CR
    // 730.3c, `merge::became_all`).
    let mut separated: Vec<ObjectId> = Vec::new();
    // The components put into one library together, the owner's to order (CR 730.3a).
    let mut into_library: Vec<(PlayerId, Vec<ObjectId>)> = Vec::new();
    // CR 400.7f follows only Auras that actually reach their owner's graveyard, and only a host that actually leaves
    // in this simultaneous batch (not one whose destruction was regenerated/replaced). Preserve that relation in the
    // history: adjacent same-cause records alone cannot distinguish separate instructions from simultaneous moves.
    let aura_hosts: Vec<(ObjectId, ObjectId)> = plans.iter().filter_map(|p| {
        if !p.from.is(ZoneKind::Battlefield) || p.to != Zone::graveyard(p.old.owner)
            || !p.last_known.subtypes.has(Subtype::Aura) { return None; }
        let host = p.old.attached_to?;
        let together = plans.iter().any(|h| h.m.id == host && h.from.is(ZoneKind::Battlefield) && !h.to.is(ZoneKind::Battlefield));
        (together || p.m.aura_host_left == Some(host)).then_some((p.m.id, host))
    }).collect();
    let turn = cx.s().turn.number;
    for Plan { m, reason, old, from, to, requested_to, change, madness, entering, mut new, counts, counts_by, last_known, components, .. } in plans {
        let m = &m;
        let row = cx.st().last_known_push(last_known);
        cx.st().zone_remove(from, m.id);
        // A basic land of a Limited event's supply (`GameState::outside_supply`) that leaves the sideboard, however it is
        // brought into the game (CR 400.11b), is replaced there by another, since a Limited sideboard holds any number of them.
        if from.is(ZoneKind::Sideboard) && cx.s().outside_supply.contains(&cx.s().obj(m.id).card) {
            let supplied = *cx.s().obj(m.id);
            let fresh = cx.st().push_obj(supplied);
            cx.st().zone_push(from, fresh);
        }
        // A spell that leaves the stack by any move other than resolving or being countered (exiled, bounced) does not
        // resolve: it is gone from the stack.
        if from.is(ZoneKind::Stack)
            && let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == m.id)
        {
            cx.st().stack_remove(at);
        }
        new.timestamp = cx.st().timestamp();
        let new_token = new.token;
        let new_id = cx.st().push_obj(new);
        // Its meld counterpart arrives with it, the one permanent (CR 712.14c): part of it from here on.
        let melded = m.entering.and_then(|e| e.melded).filter(|_| entering).map(|card| crate::merge::meld_with(cx, new_id, card));
        // What the effect putting it there says it is, it is as it arrives (CR 614.12), before anything sees it arrive.
        if entering && let Some(b) = m.entering.and_then(|e| e.becomes) {
            enter_as(cx, new_id, b);
        }
        if entering && m.entering.is_some_and(|e| e.riot_haste) {
            riot_haste(cx, new_id);
        }
        // The counters it arrives with are on it as it arrives (CR 614.1c), announced after its move.
        let mut put = Vec::new();
        counters::place_arrived(cx, new_id, counts, counts_by, &mut put);
        note_statics(cx, new_id);
        if to.is(ZoneKind::Library) {
            cx.st().zone_insert(to, 0, new_id);
        } else {
            cx.st().zone_push(to, new_id);
        }
        if to.is(ZoneKind::Graveyard) && !new_token && !old.card_copy {
            binned.push((to.player, new_id));
        }
        if entering && let Some(e) = &m.entering {
            emblem_as_it_arrives(cx, e, new_id);
        }
        // Each other component of a merged permanent goes there too, a new object of its own (CR 730.3), arriving from no zone
        // as a component, not as a permanent that left: what triggers on a card put into a zone sees it, and what triggers on
        // a permanent leaving sees the one permanent (CR 730.3, the release notes' "one creature died, three cards were put
        // into the graveyard"). Face down, it is revealed as it moves (CR 708.9).
        let mut part_events: Vec<Event> = Vec::new();
        if let Some((all, card_to)) = &components {
            let mut group = vec![new_id];
            for c in all.iter().filter(|c| !c.top) {
                let (mut to_c, exiled_with) = match card_to {
                    Some((z, by)) if !c.token => (*z, *by),
                    _ if to.is(ZoneKind::Command) && (requested_to.is(ZoneKind::Hand) || requested_to.is(ZoneKind::Library)) => (requested_to, change.exiled_with),
                    _ => (to, change.exiled_with),
                };
                if c.commander_id.is_some_and(|identity| change.commander_components.contains(&identity)) {
                    to_c = Zone::COMMAND;
                }
                let mut part = Obj::new(c.card, old.owner, to_c);
                // Exiled with the effect's source as the permanent is (CR 607.2b, 730.3d).
                if to_c.is(ZoneKind::Exile)
                    && let Some(by) = exiled_with
                {
                    part.exiled = Some(ExileLink::new(by, from, false));
                }
                part.token = c.token;
                part.commander_id = c.commander_id;
                part.commander = c.commander_id.is_some();
                part.commander_return_pending = part.commander && (to_c.is(ZoneKind::Graveyard) || to_c.is(ZoneKind::Exile));
                part.transformed = c.transformed;
                part.controlled_since = turn;
                part.timestamp = cx.st().timestamp();
                let part_id = cx.st().push_obj(part);
                if to_c.is(ZoneKind::Library) {
                    cx.st().zone_insert(to_c, 0, part_id);
                } else {
                    cx.st().zone_push(to_c, part_id);
                }
                if to_c.is(ZoneKind::Graveyard) && !c.token {
                    binned.push((to_c.player, part_id));
                }
                part_events.push(Event::ZoneChange { old: m.id, new: part_id, from: Zone::GONE, to: to_c, reason, part: true, put_by: None, enchanted_left: None, from_revealed_hand: false, to_revealed_hand: false });
                if c.face_down {
                    part_events.push(Event::Revealed { player: old.owner, card: part_id });
                }
                if to_c == to {
                    group.push(part_id);
                }
                separated.push(part_id);
            }
            if to.is(ZoneKind::Library) && !change.shuffle && group.len() > 1 {
                into_library.push((to.player, group));
            }
        }
        cx.st().update_obj(m.id, |o| {
            o.became = Some(new_id);
            o.last_known = Some(row);
        });
        // "Reveal it and shuffle it into its owner's library instead" (CR 701.20a, 701.24a).
        if change.reveal {
            cx.emit(Event::Revealed { player: old.owner, card: new_id });
        }
        // Shuffled now, announced after the move that put it there is: the history says it arrived, then was shuffled.
        if change.shuffle && to.is(ZoneKind::Library) {
            crate::turn::shuffle_in(cx, to);
            shuffled.push(to.player);
        }
        // Rebound exiled it as it resolved (CR 702.88a), if no later effect sent it elsewhere: its delayed triggered ability is
        // created once the move is recorded.
        if change.rebounding && to.is(ZoneKind::Exile) {
            rebounds.push((new_id, old.controller, old.card));
        }
        if madness {
            // "When you do" (CR 702.35a, 603.12): a reflexive trigger of the exiled card, its owner's.
            cx.st().trigger_push(PendingTrigger { event_amount: None,
                source: new_id,
                ability: AbilityRef::madness(old.card),
                controller: old.owner,
                subject: Target::None,
                data: [0; 2],
                members: cantrip_state::Span::default(),
                count: None,
                since: None,
                acquired: Acquired::OWN,
                words: None,
            });
        }

        // A permanent that leaves the battlefield leaves combat (CR 506.4).
        if from.is(ZoneKind::Battlefield) {
            remove_from_combat(cx, &[m.id]);
        }
        events.push(Event::ZoneChange { old: m.id, new: new_id, from, to, reason, part: false, put_by: change.put_by.filter(|_| to.is(ZoneKind::Battlefield)), enchanted_left: aura_hosts.iter().find_map(|&(a, h)| (a == m.id).then_some(h)), from_revealed_hand: from.is(ZoneKind::Hand) && hands_before & (1 << from.player.0) != 0, to_revealed_hand: false });
        events.extend(melded);
        events.append(&mut part_events);
        // CR 708.9: a face-down permanent or spell that moves to any other zone is revealed to all players by its owner as
        // it moves - where it arrives face up, as a card going anywhere but exile face down does.
        if old.face_down
            && matches!(from.kind(), ZoneKind::Battlefield | ZoneKind::PhasedOut | ZoneKind::Stack)
            && !entering
            && !cx.s().obj(new_id).face_down
        {
            events.push(Event::Revealed { player: old.owner, card: new_id });
        }
        // CR 303.4f, 301.5e: it entered attached to something, so it became attached to it.
        if entering && let Some(attached) = entered_attached(cx, new_id) {
            events.push(attached);
        }
        // CR 708.3: it entered face down, which the history keeps, as turning it face up later ends that (CR 708.8).
        if entering && cx.s().obj(new_id).face_down {
            events.push(Event::EnteredFaceDown { permanent: new_id, controller: cx.s().obj(new_id).controller });
        }
        // CR 709.5h: a door unlocked as the Room enters is unlocked all the same ("when you unlock this door" triggers).
        if entering && cx.s().obj(new_id).marks.doors != 0 {
            events.push(Event::DoorUnlocked { permanent: new_id, door: cx.s().obj(new_id).marks.doors, player: cx.s().obj(new_id).controller });
        }
        // The counters it arrived with were put on it (CR 122.6).
        events.extend(put);
        created.push(new_id);
        if entering {
            entered.push(new_id);
        }
        if entering && change.attacking {
            attacking_arrivals.push(new_id);
        }
        if m.entering.is_some_and(|e| e.prepared) && entering {
            become_prepared(cx, new_id);
        }
    }
    // A replacement that has it arrive attacking (CR 614.1c, 508.4): in the combat as it arrives, before anything of its
    // entry is announced, and attacking what its controller chooses (CR 508.4).
    if attacking.is_none() && !attacking_arrivals.is_empty() {
        enter_attacking_by_choice(cx, &attacking_arrivals);
    }
    if let Some(defender) = attacking {
        enter_attacking(cx, &entered, defender);
    }
    if binned.len() > 1 {
        arrange_graveyards(cx, &binned);
    }
    for (owner, group) in into_library {
        crate::merge::arrange_in_library(cx, owner, &group);
    }
    created.extend(separated);
    // What each permanent was as it entered (CR 603.6a, 614.12): judged once they are all there, as each one's continuous effects
    // reach the ones entering with it, and recorded with the moves so the history can say it however they change after.
    for permanent in entered {
        events.push(entered_event(cx, permanent));
    }
    let lost = apply_life_changes(cx, life, &mut events);
    let hands_after = crate::hand_visibility::mask(cx, cx.cards());
    for event in &mut events {
        if let Event::ZoneChange { to, to_revealed_hand, .. } = event {
            *to_revealed_hand = to.is(ZoneKind::Hand) && hands_after & (1 << to.player.0) != 0;
        }
    }
    cx.emit_batch(&events, Some(crate::settle::Before { battlefield: &before, then: &then, seen_by_all: &seen }));
    for player in shuffled {
        cx.record(Event::Shuffled { player });
    }
    // "At the beginning of your next upkeep, you may cast this card from exile without paying its mana cost" (CR 702.88a).
    for (card, controller, printed) in rebounds {
        let since = cx.g.log.len() as u32;
        cx.st().delayed_push(cantrip_state::Delayed {
            when: DelayedWhen::Step { step: Step::Upkeep, on_turn_of: Some(controller) },
            source: card,
            controller,
            ability: AbilityRef::rebound(printed),
            subject: Target::None,
            members: cantrip_state::Span::default(),
            since,
            acquired: Acquired::OWN,
            words: None,
        });
    }
    // A permanent left the battlefield: what was exiled or phased out until it left comes back now (CR 610.3, 610.4).
    if left_battlefield {
        return_until_left(cx);
    }
    (created, seen, lost)
}

/// "If an effect or rule puts two or more cards into the same graveyard at the same time, the owner of those cards may
/// arrange them in any order" (CR 404.3): every simultaneous move into graveyards comes through here (`move_together`), the
/// cards already in place in the order the move listed them, `binned` by the graveyard's player. Each owner with two or more
/// of them is asked, in APNAP order (CR 101.4), with the rules' own ordering question (`DestinationKind::GraveyardOrder`,
/// the cards listed top first, so that answering them as listed keeps the order they arrived in), and they are rearranged
/// as answered before the move is announced: nothing sees them in any other order.
///
/// Asked only while the order can be told apart: while some object of the game reads a graveyard's order
/// (`graveyard_order_read`). Without one, no order of these cards is distinguishable from any other by anything that can
/// happen in the game, so the order they arrived in is as good as any their owner could choose, and no game pays a prompt
/// for it - as several effects that each prevent all of some damage are not ordered (`replace::prevents_all`).
fn arrange_graveyards(cx: &mut Cx, binned: &[(PlayerId, ObjectId)]) {
    if !graveyard_order_read(cx) {
        return;
    }
    let players: Vec<PlayerId> = cx.s().apnap().collect();
    for owner in players {
        let arrived: Vec<ObjectId> = binned.iter().filter(|&&(p, _)| p == owner).map(|&(_, c)| c).collect();
        if arrived.len() < 2 {
            continue;
        }
        // Top first: the last to arrive is on top.
        let listed: Vec<ObjectId> = arrived.iter().rev().copied().collect();
        let source = match cx.cause() {
            Cause::Spell(spell) => spell,
            Cause::Ability { source, .. } => source,
            _ => listed[0],
        };
        let ask = RequestKind::DistributeCards {
            source,
            cards: listed.clone(),
            destinations: vec![Destination { kind: DestinationKind::GraveyardOrder, min: 0, max: u8::MAX, eligible: listed.clone() }],
            may_shuffle: false,
        };
        // Unanswered yet, the run goes on with the order they arrived in and is replayed with the answer (`Cx::ask_core`).
        let order = match cx.ask_rule(owner, ask) {
            Ok(Some(Answer::Placements { placements, .. })) => placements.into_iter().next().unwrap_or_default(),
            _ => continue,
        };
        if order == listed || order.len() != listed.len() {
            continue;
        }
        let zone = Zone::graveyard(owner);
        for &card in &arrived {
            cx.st().zone_remove(zone, card);
        }
        for &card in order.iter().rev() {
            cx.st().zone_push(zone, card);
        }
    }
}

/// Whether anything in the game can read the order of a graveyard (CR 404.2): an object, in any zone or outside the game,
/// that is - or once was, or is a copy of - a card whose text reads it (`CardDef::reads_graveyard_order`). Every object the
/// game has had keeps its row, and nothing enters a game but from its players' cards and what they create, so this is
/// every card that could ever look.
fn graveyard_order_read(cx: &Cx) -> bool {
    let cards = cx.cards();
    cx.s().objects.iter().any(|o| cards.get(o.card).reads_graveyard_order || o.copy.is_some_and(|c| cards.get(c.card).reads_graveyard_order))
}

/// Whether `attach` would attach `attachment` to `host` now (CR 701.3a-b): both are the permanents they were on the
/// battlefield, it is not already attached there, and the host is one it could legally be attached to (`host_allows`).
pub fn attachable(cx: &Cx, attachment: ObjectId, host: ObjectId) -> bool {
    live(cx.s(), attachment)
        && cx.s().obj(attachment).zone.is(ZoneKind::Battlefield)
        && cx.s().obj(attachment).attached_to != Some(host)
        && crate::attach::host_allows(cx, attachment, host, true)
        && cx.s().obj(host).zone.is(ZoneKind::Battlefield)
}

/// Attaches an Aura or Equipment to a permanent (CR 701.3a). Nothing happens when it could not be attached there
/// (CR 701.3b) or already is (`attachable`). Attaching reassigns its timestamp (CR 701.3c, 613.7e), and it has become
/// attached (`Event::Attached`), what this returns.
pub fn attach(cx: &mut Cx, attachment: ObjectId, host: ObjectId) -> bool {
    if !attachable(cx, attachment, host) {
        return false;
    }
    // Moving it off what it was on unattaches it from that first (CR 701.3d).
    if let Some(old) = cx.s().obj(attachment).attached_to
        && live(cx.s(), old)
        && cx.s().obj(old).zone.is(ZoneKind::Battlefield)
    {
        cx.emit(Event::Unattached { attachment, from: old });
    }
    let timestamp = cx.st().timestamp();
    cx.st().update_obj(attachment, |o| {
        o.attached_to = Some(host);
        o.timestamp = timestamp;
    });
    cx.emit(Event::Attached { attachment, to: Target::Object(host) });
    true
}

/// What a permanent that has just entered the battlefield entered attached to (CR 303.4f, 301.5e): its `Event::Attached`,
/// recorded after its `ZoneChange`.
fn entered_attached(cx: &Cx, id: ObjectId) -> Option<Event> {
    let o = cx.s().obj(id);
    let to = match (o.attached_to, o.attached_player) {
        (Some(host), _) => Target::Object(host),
        (None, Some(p)) => Target::Player(p),
        (None, None) => return None,
    };
    Some(Event::Attached { attachment: id, to })
}

/// Unattaches an Aura or Equipment (CR 701.3d); it stays where it is.
pub fn unattach(cx: &mut Cx, attachment: ObjectId) {
    let Some(host) = cx.s().obj(attachment).attached_to else { return };
    if !live(cx.s(), attachment) {
        return;
    }
    // A host that has already left was unattached from as it left, and announced then. Announced before it happens: what
    // triggers on it looks back on the game as it was just before (CR 603.10c), still attached.
    if live(cx.s(), host) && cx.s().obj(host).zone.is(ZoneKind::Battlefield) {
        cx.emit(Event::Unattached { attachment, from: host });
    }
    cx.st().update_obj(attachment, |o| o.attached_to = None);
    // The board changed after the event was announced: what "for as long as" durations (CR 611.2b) and control (CR 613.1b)
    // say of it is settled now, and the state triggers asked (CR 603.8), as an event would have.
    settle_at_once(cx);
    cx.state_changed();
}

/// Exiles cards with a permanent's ability (CR 607.2a, 610.3), linking each to it and to where it came from.
pub fn exile_with(cx: &mut Cx, cards: &[ObjectId], by: Linked, until_leaves: bool) -> Vec<ObjectId> {
    // CR 610.3b: the duration has ended before the exile began, so nothing is exiled. A permanent phased out has not left
    // (CR 702.26d).
    if until_leaves && !crate::phasing::still_on_battlefield(cx.s(), by.object) {
        return Vec::new();
    }
    // Only what is still the object it was is exiled (CR 400.7), so each new object is read with the zone it came from.
    let return_by = cx.acting_player();
    let cards = live_ones(cx, cards);
    let from: Vec<Zone> = cards.iter().map(|&c| cx.s().obj(c).zone).collect();
    let moves: Vec<Moving> = cards.iter().map(|&c| Moving::to(c, Zone::exile(cx.s().obj(c).owner))).collect();
    let new = move_together(cx, &moves, ZoneReason::Exile);
    // Each card a merged permanent became is exiled with it as well (CR 730.3c: "the same actions are taken upon each").
    for (&card, from) in cards.iter().zip(from) {
        for n in Query::became_all(&*cx, card) {
            if cx.s().obj(n).zone.is(ZoneKind::Exile) {
                cx.st().update_obj(n, |o| o.exiled = Some(ExileLink { return_by: until_leaves.then_some(return_by).flatten(), ..ExileLink::new(by, from, until_leaves) }));
            }
        }
    }
    new
}

/// Links cards already in exile to `by` (CR 607.2a): `Game::link_exiled`, for the exiled cards used to craft a permanent
/// (CR 702.167c). Each keeps the zone it came from; none is "until" anything.
pub fn link_exiled(cx: &mut Cx, cards: &[ObjectId], by: Linked) {
    for &c in cards {
        if live(cx.s(), c) && cx.s().obj(c).zone.is(ZoneKind::Exile) {
            let from = cx.s().obj(c).exiled.map_or(cx.s().obj(c).zone, |l| l.from);
            cx.st().update_obj(c, |o| o.exiled = Some(ExileLink::new(by, from, false)));
        }
    }
}

/// Exiles a spell card encoded on a creature (CR 702.99a, 702.99b): exiled linked to it, and marked as encoded there.
pub fn encode(cx: &mut Cx, card: ObjectId, creature: ObjectId) -> Option<ObjectId> {
    // Encoded on the creature, not exiled by an ability of its own: what is linked to it is the encoding, which no ability
    // of the creature reads but cipher's (`ExileLink::encoded`), named by the card encoded.
    let text = cx.g.cards.linked_card(cx.s().obj(card).card);
    let new = *exile_with(cx, &[card], Linked { object: creature, text, acquired: Acquired::OWN }, false).first()?;
    if !cx.s().obj(new).zone.is(ZoneKind::Exile) {
        return None;
    }
    cx.st().update_obj(new, |o| {
        if let Some(link) = &mut o.exiled {
            link.encoded = true;
        }
    });
    Some(new)
}

/// Exiles cards together face down (CR 406.3, 708.2): linked to `by` when an ability exiled them (CR 607.2a). A face-down
/// card in exile has no characteristics for a player who may not look at it; those who could see where it came from may
/// (CR 406.3): its owner for a card from a hand, everyone for one from a public zone, nobody for one from a library.
pub fn exile_face_down(cx: &mut Cx, cards: &[ObjectId], by: Option<Linked>) -> Vec<ObjectId> {
    // Only what is still the object it was is exiled (CR 400.7), so each new object is read with where it came from.
    let cards = &live_ones(cx, cards)[..];
    let from: Vec<Zone> = cards.iter().map(|&c| cx.s().obj(c).zone).collect();
    let owners: Vec<PlayerId> = cards.iter().map(|&c| cx.s().obj(c).owner).collect();
    let new = match by {
        Some(by) => exile_with(cx, cards, by, false),
        None => {
            let moves: Vec<Moving> = cards.iter().map(|&c| Moving::to(c, Zone::exile(cx.s().obj(c).owner))).collect();
            move_together(cx, &moves, ZoneReason::Exile)
        }
    };
    let everyone = cx.s().seats().fold(0u8, |mask, p| mask | 1 << p.0);
    for ((&n, from), owner) in new.iter().zip(from).zip(owners) {
        if cx.s().obj(n).zone.is(ZoneKind::Exile) {
            let seen_by = match from.kind() {
                ZoneKind::Hand => 1 << owner.0,
                ZoneKind::Library => 0,
                _ => everyone,
            };
            cx.st().update_obj(n, |o| {
                o.face_down = true;
                o.seen_by = seen_by;
            });
        }
    }
    new
}

/// `player` looks at face-down cards in exile (CR 406.3): they may look at them from now on.
pub fn look_at(cx: &mut Cx, player: PlayerId, cards: &[ObjectId]) {
    for &c in cards {
        if live(cx.s(), c) {
            cx.emit(Event::Looked { player, card: c });
        }
        if live(cx.s(), c) && cx.s().obj(c).face_down && cx.s().obj(c).zone.is(ZoneKind::Exile) {
            cx.st().update_obj(c, |o| o.seen_by |= 1 << player.0);
        }
    }
}

/// Puts an object onto the battlefield face down (CR 708.3; manifest, CR 701.40a; cloak, CR 701.58a): it is turned face
/// down before it enters, so what replaces its entering judges the face-down permanent `how` lists (a nameless 2/2
/// creature, with ward {2} if cloaked), and its card's own "enters" replacements do not apply. The permanent, if it entered
/// face down; a card kept from entering or sent elsewhere keeps its own characteristics (CR 701.40f, 701.58f).
pub fn put_onto_battlefield_face_down(cx: &mut Cx, id: ObjectId, controller: PlayerId, how: FaceDown, tapped: bool) -> R<Option<ObjectId>> {
    Ok(put_all_onto_battlefield_face_down(cx, &[id], controller, how, tapped)?.first().copied().flatten())
}

/// `put_onto_battlefield_face_down` for several objects at once (CR 110.2a): they enter together, each face down, so each
/// sees the others enter (CR 603.6a). For each object, the permanent it became, or None if it did not enter face down.
pub fn put_all_onto_battlefield_face_down(cx: &mut Cx, ids: &[ObjectId], controller: PlayerId, how: FaceDown, tapped: bool) -> R<Vec<Option<ObjectId>>> {
    let values = Copiable::of(cx.cards().face_down_values(how));
    // CR 708.2b: a face-down permanent can't be turned face down; an object that changed zones is no longer the one
    // chosen (CR 400.7).
    let moving: Vec<(ObjectId, Option<Copiable>)> = ids
        .iter()
        .copied()
        .filter(|&id| live(cx.s(), id) && !cx.s().obj(id).zone.is(ZoneKind::Battlefield))
        .map(|id| (id, cx.s().obj(id).copy))
        .collect();
    for &(id, _) in &moving {
        cx.st().update_obj(id, |o| o.copy = Some(values));
    }
    let objects: Vec<(ObjectId, PlayerId, bool)> = moving.iter().map(|&(id, _)| (id, controller, tapped)).collect();
    // Judged face down: a card a static ability keeps from entering as the face-down card it now is does not move
    // (CR 701.40f, 701.58f), so each is found by what it became, not by its place among those that moved.
    put_onto_battlefield(cx, &objects, ZoneReason::Other)?;
    let mut entered: Vec<Option<ObjectId>> = vec![None; ids.len()];
    for &(id, was) in &moving {
        let new = cx.s().obj(id).became.filter(|&new| cx.s().obj(new).zone.is(ZoneKind::Battlefield) && cx.s().obj(new).face_down);
        if new.is_none() && live(cx.s(), id) {
            cx.st().update_obj(id, |o| o.copy = was);
        }
        if let Some(at) = ids.iter().position(|&i| i == id) {
            entered[at] = new;
        }
    }
    Ok(entered)
}

/// `player` turns a face-down permanent face up (CR 701.40b, 708.2): it is the card it is, the same object (CR 708.8).
/// Nothing happens where a static ability says it can't be turned face up (CR 101.2, `CardDef::face_up_bans`).
/// `player` is the one who took the special action, or the controller of the effect that turns it up.
pub fn turn_face_up(cx: &mut Cx, id: ObjectId, player: PlayerId) {
    turn_face_up_as(cx, id, player, false);
}

/// `turn_face_up`, with a +1/+1 counter put on the permanent as it is turned face up where `megamorph` - its megamorph cost was
/// paid to turn it up (CR 702.37b) - applied while it is turned face up, not afterward (CR 708.11): what triggers on its being
/// turned face up sees the counter on it.
pub fn turn_face_up_as(cx: &mut Cx, id: ObjectId, player: PlayerId, megamorph: bool) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) || !cx.s().obj(id).face_down {
        return;
    }
    // "Can't be turned face up" beats the rule or effect that would turn it up (CR 101.2, 708.7): it stays face down, and
    // as nothing would turn it face up, a cloaked instant or sorcery card is not revealed either (CR 701.58g).
    if crate::static_rules::cant_be_turned_face_up(cx, cx.cards(), id) {
        return;
    }
    // CR 701.40g, 701.58g: a manifested or cloaked instant or sorcery card that would turn face up is revealed by its
    // controller and left face down, and nothing that triggers on turning face up does.
    // CR 730.2g: so is a face-down merged permanent that contains an instant or sorcery card.
    let spell_card = |card: CardId| {
        let def = cx.cards().get(card);
        def.types.has(CardType::Instant) || def.types.has(CardType::Sorcery)
    };
    let merged_spell = cx.s().merge_of(id).is_some_and(|m| m.parts.iter().any(|p| !p.token && spell_card(p.card)));
    if spell_card(cx.s().obj(id).card) || merged_spell {
        let controller = cx.s().obj(id).controller;
        cx.emit(Event::Revealed { player: controller, card: id });
        return;
    }
    // Turned up it is its own card under whatever copy effects are on it (CR 708.8, 613.2b): the face-down values of layer
    // 1b no longer apply over them.
    cx.st().update_obj(id, |o| {
        o.face_down = false;
        o.marks.face_down_as = None;
    });
    note_statics(cx, id);
    // Put by its own megamorph ability, the permanent's controller's (CR 122.6a), as it is turned face up.
    if megamorph {
        let controller = cx.s().obj(id).controller;
        counters::put(cx, id, CounterKind::PlusOne, 1, controller, counters::How::Effect);
    }
    cx.emit(Event::TurnedFaceUp { permanent: id, player });
}

/// CR 406.3a: a card exiled face down that is played is turned face up just before the player announces it (CR 601.2) or
/// plays it as a land (CR 305.1), so the spell or land it becomes is the card it is. (A card cast face down from exile
/// would be the exception, CR 708.4; nothing casts one face down from there.)
pub fn turn_up_to_play(cx: &mut Cx, card: ObjectId) {
    if cx.s().obj(card).face_down_card() {
        cx.st().update_obj(card, |o| o.face_down = false);
    }
}

/// The door whose half was cast, by the way it was cast (CR 709.3, 709.5d): none for anything but a split card's halves.
fn cast_door(way: u8) -> u8 {
    match way {
        WAY_SPLIT_LEFT => DOOR_LEFT,
        WAY_SPLIT_RIGHT => DOOR_RIGHT,
        _ => 0,
    }
}

/// Unlocks a door of a Room (CR 709.5f): the permanent is given the half's unlocked designation, the same object
/// (CR 709.5c), and "when you unlock this door" triggers (CR 709.5h). Nothing if it is not a Room on the battlefield or
/// the door is already unlocked.
pub fn unlock_door(cx: &mut Cx, id: ObjectId, door: u8, player: PlayerId) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) || cx.s().obj(id).marks.doors & door != 0 {
        return;
    }
    if !cx.card(cx.chars(id).card).is_room() {
        return;
    }
    cx.st().update_obj(id, |o| o.marks.doors |= door);
    cx.emit(Event::DoorUnlocked { permanent: id, door, player });
}

/// A card name, an object or a player chosen for a permanent as an ability of it resolves, or as it transforms (CR 607.2d,
/// 712.20): kept by the permanent as the object it is now, so a new object it becomes has none (CR 400.7), and a later
/// choice of the same kind by the same ability replaces an earlier one. Nothing if it is no longer on the battlefield.
pub fn remember_chosen(cx: &mut Cx, by: Linked, value: Kept) {
    let id = by.object;
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
        return;
    }
    // Kept with the ability that chose it (CR 607.2d): only abilities linked to that one refer to it.
    let mut choices = cx.s().obj(id).marks.choices;
    if choices.keep(by, value) {
        cx.st().update_obj(id, |o| o.marks.choices = choices);
    }
}

/// CR 610.3: cards exiled "until [a permanent] leaves the battlefield" return, as a one-shot effect that follows the
/// event, to the zone they were exiled from; a permanent comes back under its owner's control (610.3c), all of them in one
/// move (610.3d). Carried out the moment the permanent leaves (`return_until_left`). Whether any did.
pub fn return_exiled(cx: &mut Cx) -> R<bool> {
    let mut due: Vec<(ObjectId, ExileLink)> = Vec::new();
    for p in cx.s().seats() {
        for &id in cx.s().zone(Zone::exile(p)) {
            if let Some(link) = cx.s().obj(id).exiled
                && link.until_leaves
                // Phasing out is no leaving (CR 702.26d; Teferi's Protection's ruling on Banishing Light).
                && !crate::phasing::still_on_battlefield(cx.s(), link.by)
            {
                due.push((id, link));
            }
        }
    }
    if due.is_empty() {
        return Ok(false);
    }
    let owner = |cx: &Cx, id: ObjectId| cx.s().obj(id).owner;
    let to_battlefield: Vec<(ObjectId, PlayerId, bool)> =
        due.iter().filter(|(_, l)| l.from.is(ZoneKind::Battlefield)).map(|&(id, _)| (id, owner(cx, id), false)).collect();
    let elsewhere: Vec<Moving> = due
        .iter()
        .filter(|(_, l)| !l.from.is(ZoneKind::Battlefield))
        .map(|&(id, l)| Moving::to(id, Zone::new(l.from.kind(), owner(cx, id))))
        .collect();
    // One move (CR 610.3d): the cards going back to a hand or a graveyard and the permanents entering arrive together.
    let actors: Vec<_> = due.iter().map(|&(id, link)| (id, link.return_by.map_or(PutBy::Rules, PutBy::Player))).collect();
    put_onto_battlefield_into(cx, elsewhere, &to_battlefield, ZoneReason::Other, &[], None, None, None, &actors)?;
    // The return is one one-shot effect, created once (CR 610.3): the duration is over whether or not the card got where
    // it went. One that could not - an Aura with nothing to enchant (CR 303.4g), a move a replacement left in exile - stays
    // there as a card exiled with that permanent and nothing more, never returned again. Cleared once the moves are done,
    // so a return that stops for a question (an Aura's host, CR 303.4f) is the same return when it goes on.
    for &(id, _) in &due {
        if live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Exile) {
            cx.st().update_obj(id, |o| {
                if let Some(link) = &mut o.exiled {
                    link.until_leaves = false;
                }
            });
        }
    }
    Ok(true)
}

/// CR 610.3, 610.4: "A second one-shot effect is created immediately after the specified event": what was exiled "until [a
/// permanent] leaves the battlefield" returns (`return_exiled`), and what phased out until then phases in
/// (`phasing::phase_in_returning`), the moment that permanent has left - a move off the battlefield (`move_together`) or
/// its owner leaving the game (`leave_game`) - whatever is under way: a cost being paid (CR 601.2g-h), a spell or ability
/// resolving (CR 608.2). Neither is a triggered ability or a state-based action, so neither waits for a player to receive
/// priority or for the next check. A state-based check is one event (CR 704.3): its returns are held until all of it is
/// performed, the losers gone with it, and then made together, before the next check (CR 610.3d, 610.4d;
/// `settle::state_based_actions`). The returns go on until none is due: one that makes another such permanent leave (an
/// "as enters" sacrifice) is followed by the returns that leaving makes due.
///
/// A return can ask a question - the permanent an Aura returns attached to (CR 303.4f), an "as [this] enters" choice - and
/// the operation it follows has no suspension to give back. The question waits on the frame, and the run goes on and is
/// wound back to it and redone with the answer (`Game::run`), as one a replacement effect asks mid-move is; nothing more
/// is returned while it waits. Outside a run (a rules test's bare `Cx`, a concession between runs) no one could be asked,
/// so the returns are left to the next settling, which carries them out first, before the state triggers and the
/// state-based actions look at the board (`settle::settle`).
pub fn return_until_left(cx: &mut Cx) {
    if cx.s().returning.get() || cx.s().agenda.get(cx.frame).is_none_or(|f| f.pending.is_some()) {
        return;
    }
    // A question asked stops it: the run is redone with its answer, and the rest is returned then.
    while let Ok(true) = returns_due(cx) {}
}

/// The returns due now (CR 610.3, 610.4) carried out once, the exiled cards' (`return_exiled`) and the phased-out
/// permanents' (`phasing::phase_in_returning`): whether any was. The moves they make look for no returns of their own
/// (`GameState::returning`, held for the length of this and lowered however it ends, a host that catches a panic going
/// on with the game). `return_until_left` goes round it; settling asks it as a backstop (`settle::settle`).
pub fn returns_due(cx: &mut Cx) -> R<bool> {
    let held = cx.s().returning.replace(true);
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let returned = return_exiled(&mut *cx)?;
        Ok(crate::phasing::phase_in_returning(&mut *cx) || returned)
    }));
    cx.s().returning.set(held);
    match done {
        Ok(done) => done,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

pub fn move_to(cx: &mut Cx, id: ObjectId, to: Zone) -> ObjectId {
    move_to_for(cx, id, to, ZoneReason::Other)
}

/// `move_to`, for a reason. The new object; `id` itself when nothing moved - it had already changed zones or ceased to exist
/// (CR 400.7), or a replacement kept it where it is.
pub fn move_to_for(cx: &mut Cx, id: ObjectId, to: Zone, reason: ZoneReason) -> ObjectId {
    move_together(cx, &[Moving::to(id, to)], reason).first().copied().unwrap_or(id)
}

/// The entering replacements an object about to enter has of its own (CR 614.12), applied to `entering`.
///
/// Which of them apply is judged on the permanent as it would exist on the battlefield (`as_it_would_exist`): one that would
/// have lost all its abilities there (Humility, Dress Down) or the abilities of its rules text (CR 305.7: a nonbasic land
/// under Blood Moon, Blood Moon's ruling) has none of its own "enters tapped", "enters with" and "as this enters"
/// replacements, nor daybound's "enters transformed" - a Walking Ballista enters with no counters, a Clone copies nothing.
pub fn prepare_entry(cx: &mut Cx, entering: &mut Entering) -> R<()> {
    // CR 708.3: a face-down permanent has no abilities, so none of its card's "enters" replacements apply.
    if cx.s().obj(entering.object).copy.is_some_and(|c| cx.card(c.card).face_down) {
        return Ok(());
    }
    // Most positions have no static ability that could take an entering permanent's abilities: then it has its own. Its
    // own static abilities count among them (a "loses all abilities" or land-type-setting ability it would have there).
    let own = cx.cards().get(cx.cards().face(cx.s().obj(entering.object).card, entering.transformed));
    let may_lose = layers::abilities_may_be_lost(cx.cards(), cx.s())
        || own.statics.iter().any(|s| s.loses_all || s.sets_land_subtypes)
        || entering.copy.is_some_and(|c| cx.card(c.card).statics.iter().any(|s| s.loses_all || s.sets_land_subtypes))
        || entering.becomes.is_some_and(|b| b.what.loses_other_abilities);
    let lost = |cx: &mut Cx, entering: &Entering| may_lose && as_it_would_exist(cx, entering).lost;
    // A token made as a copy, its copy values over its card (a copy of a Clone, of Glasspool Mimic copying a creature):
    // what it enters as is that copy's, not its card's. (A bestowed Aura spell's copy values are its own card's Aura face.)
    let copied = cx.s().obj(entering.object).token && copy_over(cx, entering);
    // CR 616.1c: a replacement that has it enter as a copy applies first - its face's own (Clone's, `CardDef::enters_as_copy`),
    // the face that would be up as nothing has turned it over yet - unless a copy effect is over it already.
    let face = cx.cards().face(cx.s().obj(entering.object).card, entering.transformed);
    let mut copied_by = None;
    if !copy_over(cx, entering)
        && let Some(enters_as_copy) = cx.cards().get(face).enters_as_copy
        && !lost(cx, entering)
    {
        enters_as_copy(cx, entering)?;
        copied_by = Some(face);
    }
    // CR 616.1d, 702.145b: then, at night, daybound has a permanent represented by a double-faced card enter transformed -
    // judged on the copy it now enters as, if any. Daybound only ever turns it over: a face an instruction or CR 707.8a gave
    // it stays.
    if !entering.transformed && enters_by_daybound(cx, entering, may_lose) {
        entering.transformed = true;
    }
    choose_protector(cx, entering)?;
    // Its face's other replacements (CR 616.1e) - none where a copy effect is over it (its copy replacement made it a copy, it
    // was made as one, or it is a bestowed Aura, CR 702.103b), as its copy's apply instead, below. Applied with the other
    // replacements that modify how it enters, in the order its controller chooses (CR 616.1, `entry_replacements`).
    let card = cx.cards().face(cx.s().obj(entering.object).card, entering.transformed);
    if !copy_over(cx, entering)
        && cx.cards().get(card).enters.is_some()
        && !lost(cx, entering)
    {
        own_to_apply(entering, card);
    }
    // CR 707.9a: what it entered as a copy of has its own "enters with" and "enters tapped" to apply, as the copy
    // enters - its copy replacement too, which has nothing more to do unless it is another card's (a Clone copying a Clone
    // that entered as itself) - and a copy of a kicked permanent was never kicked itself (CR 702.33d). They are judged on
    // it as that copy (CR 614.12: "taking into account replacement effects that have already modified how it enters"). A
    // double-faced token's own copy values are its faces, whose own replacements are the face's, above (CR 707.8a).
    if copy_over(cx, entering)
        && let Some(copy) = entering.copy
        && (copied || Some(copy.card) != copied_by)
        && !lost(cx, entering)
    {
        let def = cx.cards().get(copy.card);
        if let Some(enters_as_copy) = def.enters_as_copy {
            enters_as_copy(cx, entering)?;
        }
        let card = entering.copy.map_or(copy.card, |c| c.card);
        if cx.cards().get(card).enters.is_some() {
            own_to_apply(entering, card);
        }
    }
    // CR 702.145b: the daybound of a permanent that its own replacement made a copy (a Clone-like double-faced card) is the
    // copy's - the example under CR 712.13a: a double-faced card entering at night as a copy of a daybound creature enters
    // transformed. Only ever turned over, never back.
    if !copied && !entering.transformed && copy_over(cx, entering) && enters_by_daybound(cx, entering, may_lose) {
        entering.transformed = true;
    }
    // Read ahead's choice is judged on what it now enters as: its face, or the copy it enters as. Riot's is not made here:
    // its replacement is ordered with the others that modify how it enters (`entry_replacements`, `apply_riot`).
    choose_read_ahead(cx, entering, may_lose)
}

/// A Saga's final chapter number (CR 714.2d): the greatest among the chapter abilities in `abilities`, 0 with none.
pub(crate) fn final_chapter<'a>(abilities: impl Iterator<Item = &'a AbilityDef>) -> u8 {
    abilities
        .filter_map(|a| match a {
            AbilityDef::Triggered(t) if t.chapter > 0 => Some(t.chapter),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

/// Read ahead (CR 702.155b, 714.3b): "As this Saga enters, choose a number between one and this Saga's final chapter
/// number" and "This Saga enters with the chosen number of lore counters on it" - replacement effects (CR 614.1c) whose
/// choice is made before it enters (CR 614.12a), by the player it enters under. Only a Saga that would have read ahead on
/// the battlefield chooses (CR 614.12): by its own text or a copy's, or given by a static ability that would apply to it
/// there (Barbara Wright's "Sagas you control have read ahead"), and not one that would have lost its abilities there. Its
/// final chapter number is that of the chapter abilities it would have (CR 714.2d); one with none has no number to choose.
/// The number chosen is the lore counters it enters with (`counters::intrinsic`), in place of a Saga's one (CR 714.3a).
fn choose_read_ahead(cx: &mut Cx, entering: &mut Entering, may_lose: bool) -> R<()> {
    // Its chapter abilities are its text's - its face's, or the copy's it enters as (no printed card gives a permanent a
    // chapter ability): with none it has no final chapter, and nothing to choose.
    let def = cx.cards().get(entering_card(cx, entering));
    if final_chapter(def.abilities.iter()) == 0 {
        return Ok(());
    }
    // Where no static ability could give it abilities or take them, and no copy effect or "it's a ..." could make it other
    // than its card, it is on the battlefield what its card says.
    let judged = may_lose || entering.copy.is_some() || entering.becomes.is_some() || layers::ability_grants_present(cx.cards(), cx.s());
    let last = if judged {
        let would = as_it_would_exist(cx, entering);
        if would.lost || !would.chars.has(Keyword::ReadAhead) || !would.chars.has_subtype(Subtype::Saga) {
            return Ok(());
        }
        final_chapter(would.chars.abilities().map(|a| cx.cards().ability(a)))
    } else {
        if !def.keywords.has(Keyword::ReadAhead) || !def.subtypes.has(Subtype::Saga) {
            return Ok(());
        }
        final_chapter(def.abilities.iter())
    };
    if last == 0 {
        return Ok(());
    }
    let ask = RequestKind::ChooseNumber {
        source: entering.object,
        least: 1,
        most: u32::from(last),
        heading: "Read ahead: choose the chapter it starts on (it enters with that many lore counters)",
    };
    // The core checks the answer is one offered (`validate`).
    entering.read_ahead = match cx.ask_core(entering.controller, ask, false)? {
        Answer::Announce(n) => u8::try_from(n).ok(),
        _ => None,
    };
    Ok(())
}

/// Whether riot could apply to the permanent `entering` would be (CR 702.136a, 614.12), now or once the other replacements
/// that modify how it enters have applied (CR 616.1f): its text prints riot (its face's, or the copy's it enters as), an
/// "it's a ..." or a static ability of its own could give it some, or a static ability on the battlefield could grant it
/// (Spider-Punk's "Other Spiders you control have riot" to a Roaming Throne whose choice makes it a Spider). The guard that
/// keeps an entry with no riot free of the judging.
fn riot_may_apply(cx: &Cx, entering: &Entering) -> bool {
    let def = cx.cards().get(entering_card(cx, entering));
    def.riot
        || entering.becomes.is_some()
        || def.statics.iter().any(|s| s.own && s.layer == Layer::Ability)
        || layers::ability_grants_present(cx.cards(), cx.s())
}

/// How many instances of riot the permanent `entering` would have on the battlefield (CR 702.136b, 614.12, `Chars::riot`), as
/// the replacements applied so far have made it: printed, by the text of a copy it enters as, or given by a static ability
/// that would apply to it there - none it would have lost there (CR 613.1f, 305.7).
fn riot_instances(cx: &mut Cx, entering: &Entering) -> u16 {
    let def = cx.cards().get(entering_card(cx, entering));
    // Where no static ability could give it abilities or take them, and no copy effect or "it's a ..." could make it other
    // than its card, it has the riot its card prints.
    let judged = entering.copy.is_some()
        || entering.becomes.is_some()
        || def.statics.iter().any(|s| s.layer == Layer::Ability || s.loses_all || s.sets_land_subtypes)
        || layers::abilities_may_be_lost(cx.cards(), cx.s())
        || layers::ability_grants_present(cx.cards(), cx.s());
    if !judged {
        return u16::from(def.riot);
    }
    let would = as_it_would_exist(cx, entering);
    if would.lost { 0 } else { would.chars.riot }
}

/// One instance of riot applied to `entering` (CR 702.136a): "You may have this permanent enter with an additional +1/+1
/// counter on it. If you don't, it gains haste" - a replacement effect (CR 614.1c) whose choice is made as it applies, before
/// it enters (CR 614.12a), by the player it enters under. Each instance is a replacement of its own, chosen separately
/// (CR 702.136b), and ordered with the other replacements that modify how it enters, its own "as this enters" among them
/// (CR 616.1, `replace::entry`): one that makes it get riot (Roaming Throne naming Spider under Spider-Punk) has riot apply
/// after it, and riot may apply before one that would take it away (Rhythm of the Wild's ruling, of Rescuer Sphinx). A
/// counter chosen is one more it enters with (`Entering::counters`); one that can't be put on it (Solemnity) is haste
/// instead (the riot cards' ruling), and haste chosen it gains as it enters (`Entering::riot_haste`, `riot_haste`). A
/// question with no answer yet leaves the entry as if the counter were chosen; the run is replayed with the answer
/// (`Cx::ask_core`).
pub(crate) fn apply_riot(cx: &mut Cx, entering: &mut Entering) {
    let rules = cx.cards().intrinsic;
    let modes = vec![Label::Option(rules, intrinsic::RIOT_COUNTER), Label::Option(rules, intrinsic::RIOT_HASTE)];
    let ask = RequestKind::ChooseModes {
        source: entering.object,
        modes,
        eligible: vec![intrinsic::RIOT_COUNTER, intrinsic::RIOT_HASTE],
        count: 1,
        at_most: 0,
        affordable: Vec::new(),
        additional_cost: false,
        repeat: false,
    };
    // The core checks the answer is one offered (`validate`).
    let chosen = match cx.ask_core(entering.controller, ask, false) {
        Ok(Answer::Modes(m)) => m.first().copied().unwrap_or(intrinsic::RIOT_COUNTER),
        _ => intrinsic::RIOT_COUNTER,
    };
    if chosen == intrinsic::RIOT_COUNTER
        && !crate::static_rules::counters_banned(&*cx, cx.cards(), entering.object, entering.controller, CounterKind::PlusOne)
    {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    } else {
        entering.riot_haste = true;
    }
}

/// "If you don't, it gains haste" (CR 702.136a): the permanent `id`, as it arrives, gains haste from then on, for as long as
/// it is that object (CR 611.2a, 400.7) - an effect of the riot that applied, which riot granted by a permanent that has left
/// since (Rhythm of the Wild's ruling) or riot lost later does not end. Its timestamp is its arrival's, so "loses all
/// abilities" later takes it (CR 613.7).
fn riot_haste(cx: &mut Cx, id: ObjectId) {
    let timestamp = cx.st().timestamp();
    let controller = cx.s().obj(id).controller;
    cx.st().effect_push(cantrip_state::ContinuousEffect {
        source: id,
        controller,
        timestamp,
        affected: id,
        kind: EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])),
        duration: Duration::Indefinite,
        words: Words::NONE,
    });
}

/// Notes `card`'s `CardDef::enters` as the entering permanent's own replacements still to apply (`Entering::own`): its
/// face's, or those of what it enters as a copy of (CR 707.9a) - `prepare_entry` notes the one text it enters with.
fn own_to_apply(entering: &mut Entering, card: CardId) {
    debug_assert!(entering.own.is_none(), "a permanent enters with the own replacements of one text");
    entering.own = Some(card);
}

/// Applies the entering permanent's own replacement (`Entering::own`, CR 614.1c-d): its card's `CardDef::enters`, to the
/// entry as the replacements applied so far have left it - under `controller`, tapped or not. Applied once: it is
/// emptied. One that asks a question with no answer yet leaves the entry as far as it got; the run goes on and is
/// replayed with the answer (`Cx::ask_core`), as for every question a zone change asks.
pub(crate) fn apply_own_entering(cx: &mut Cx, entering: &mut Entering, controller: PlayerId, tapped: bool) {
    let Some(card) = entering.own.take() else { return };
    let Some(enters) = cx.cards().get(card).enters else { return };
    entering.controller = controller;
    entering.tapped = tapped;
    let before = *entering;
    let _ = enters(cx, entering);
    if layers::verifying()
        && let Some(fault) = enters_tier_fault(&before, entering)
    {
        panic!("{}'s `CardDef::enters` {fault}", cx.cards().get(card).name);
    }
}

/// What `entering` chose as it entered (CR 614.12a), put on the permanent it is, kept with the text that made the choices
/// (`Choices`, CR 607.2d, 607.2f): the text it enters with (`entering_card`), whose own "as this enters" is the only one of
/// its own that modifies how it enters (`prepare_entry`: its face's, or what it enters as a copy of, CR 707.9a), printed or
/// of the copiable values it enters with, acquired as its own (CR 607.5).
fn keep_chosen(cx: &Cx, entering: &Entering) -> impl Fn(&mut Obj) + use<> {
    let e = *entering;
    let made = Entered {
        color: e.chosen_color,
        subtype: e.chosen_subtype,
        player: e.chosen_player,
        card: e.chosen_card,
        option: e.chosen_option,
        number: e.chosen_number,
        object: e.chosen_object,
    };
    let text = if made.any() { cx.cards().linked_card(entering_card(cx, entering)) } else { CardId(0) };
    let choices = Choices::entered(text, made);
    move |o: &mut Obj| o.marks.choices = choices
}

/// The replacement effects that modify how a permanent enters (CR 614.12) applied to its entry `proposed`: its own
/// (`Entering::own`) and every other that applies, each once, in the order its controller chooses where more than one
/// applies (CR 616.1: "the affected object's controller ... chooses one to apply"), each judged on it as it would exist
/// with those applied before it (CR 614.12, 616.1f). Spelunking's "lands you control enter untapped" and a land's own
/// "this land enters tapped": applied in the order its controller chooses, it enters tapped or untapped (Spelunking's
/// ruling). `entering` is left as they made it, but for where it goes and whether it enters tapped and under whom, which are
/// the returned zone change's; an entry its own replacement sent elsewhere (`Entering::instead`) goes there.
///
/// Each instance of riot it would have is one of them (CR 702.136b, `apply_riot`), offered while it would have more
/// instances than have applied.
///
/// Where nothing could apply but one kind - its own, its riot, or the others - there is nothing to order: they are applied
/// as they are, and nothing is judged but riot's instances.
fn entry_replacements(cx: &mut Cx, entering: &mut Entering, proposed: ZoneChange) -> ZoneChange {
    let own = entering.own.is_some();
    let riot = riot_may_apply(cx, entering);
    // A copy's own replacements that function from every zone are its, not its card's (`judged_entering`).
    let copy_replaces = copy_over(cx, entering) && cx.cards().replaces[entering_card(cx, entering).0 as usize];
    let others = copy_replaces || crate::replace::zone_change_possible(cx, &proposed);
    // The counters the effect putting it there gives it for what it is (`Entering::counters_if`) are judged on it as it would
    // exist as they apply, ordered with the rest (CR 614.12, 616.1): never the shortcut.
    if entering.counters_if.is_none() && !(own && (riot || others)) && !(riot && others) {
        let mut proposed = proposed;
        apply_own_entering(cx, entering, proposed.controller, proposed.tapped);
        proposed.controller = entering.controller;
        proposed.tapped = entering.tapped;
        // An entry its own replacement turned into a move elsewhere (CR 614.12): it does not enter at all.
        if let Some(instead) = entering.instead {
            return crate::replace::zone_change(cx, crate::replace::sent_instead(cx, proposed, instead));
        }
        // Riot alone: each instance applied in turn, judged again after the one before (CR 616.1f).
        if riot {
            let mut applied = 0;
            while riot_instances(cx, entering) > applied {
                apply_riot(cx, entering);
                applied += 1;
            }
        }
        return judged_entering(cx, entering, false, |cx| crate::replace::zone_change(cx, proposed));
    }
    // Judged as it would exist from the start, whatever its own replacements make of it (CR 614.12).
    let would = as_it_would_exist(cx, entering);
    let outer = cx.entering.replace(would);
    let change = crate::replace::entry(cx, proposed, entering);
    cx.entering = outer;
    change
}

/// A card's `enters` is ordered as one of the replacements any of which may be chosen (CR 616.1e). One that has it enter
/// under another player's control (CR 616.1b), as a copy (616.1c) or with its back face up (616.1d) is a replacement of an
/// earlier tier out of its place: its controller could choose another effect before it, which the rules don't allow. The
/// engine, not the card, decides which tier a replacement is in, so a verifying run stops on one (`apply_own_entering`): a
/// copy replacement is `CardDef::enters_as_copy`, and the others need a hook of their own tier, as it is. What `enters`
/// did out of its tier, from the entry before it to the entry after, if anything.
pub(crate) fn enters_tier_fault(before: &Entering, after: &Entering) -> Option<&'static str> {
    if after.copy != before.copy {
        Some("made it enter as a copy, or as another: a copy replacement is `CardDef::enters_as_copy`, applied before the others (CR 616.1c)")
    } else if after.controller != before.controller {
        Some("changed under whose control it enters: such a replacement is chosen before the others (CR 616.1b), and needs a hook of that tier")
    } else if after.transformed != before.transformed {
        Some("changed the face it enters with: such a replacement is chosen before the others (CR 616.1d), and needs a hook of that tier")
    } else {
        None
    }
}

/// Whether a copy effect is over the object `entering` would be (`Entering::copy`), other than a double-faced token's own
/// faces (CR 707.8a, `Obj::own_faces`).
fn copy_over(cx: &Cx, entering: &Entering) -> bool {
    entering.copy.is_some_and(|c| !cx.s().obj(entering.object).own_faces(&c))
}

/// The definition `entering` would have as it enters: the card a copy effect over it copies, else the face it enters
/// with (a double-faced token's own copy values are its faces', CR 707.8a).
fn entering_card(cx: &Cx, entering: &Entering) -> CardId {
    match entering.copy {
        Some(c) if !cx.s().obj(entering.object).own_faces(&c) => c.card,
        _ => cx.cards().face(cx.s().obj(entering.object).card, entering.transformed),
    }
}

/// CR 702.145b: whether, it being night, the permanent `entering` would be has daybound and is represented by a double-faced
/// card or token, so that it enters transformed: judged on it front face up as it would exist (CR 614.12) - its card's
/// daybound, or a copy effect's over it (the example under CR 712.13a), which a "loses all abilities" effect takes.
fn enters_by_daybound(cx: &mut Cx, entering: &Entering, may_lose: bool) -> bool {
    if !crate::day_night::night_and_double_faced(cx, entering.object) {
        return false;
    }
    if entering.copy.is_none() && !may_lose {
        return crate::day_night::enters_transformed(cx, entering.object);
    }
    let face_up = Entering { transformed: false, ..*entering };
    as_it_would_exist(cx, &face_up).chars.has(Keyword::Daybound)
}

/// A permanent about to enter the battlefield as it would exist there (CR 614.12): what the replacement effects that
/// modify how it enters are judged on.
#[derive(Clone, Copy)]
pub(crate) struct WouldExist {
    pub object: ObjectId,
    /// How it enters, as the replacements applied so far have made it: what it is judged again from when one changes it.
    pub entering: Entering,
    pub chars: Chars,
    /// Read before the hypothetical battlefield object is reverted: face,
    /// copy, unlocked Room doors and off-stack X all belong to that object.
    pub mana_value: u32,
    /// It would have lost all its abilities there (CR 613.1f), or a land the abilities of its rules text (CR 305.7).
    pub lost: bool,
}

/// The permanent `entering` would be on the battlefield (CR 614.12): "check the characteristics of the permanent as it
/// would exist on the battlefield, taking into account replacement effects that have already modified how it enters the
/// battlefield ..., continuous effects from the permanent's own static abilities that would apply to it once it's on the
/// battlefield, and continuous effects that already exist and would apply to the permanent." Worked out by putting it on the
/// battlefield for the moment of the question - under the player it enters under, with the face, the copy effect, the
/// choices and the counters the replacements applied so far gave it, and the timestamp of a permanent entering now
/// (CR 613.7d), the latest - and taking that back, so every layer judges it as it would be there, other objects' static
/// abilities and its own alike. Objects entering with it are not there (CR 614.12, 101.4). No resolved effect can name it
/// (CR 611.2c, 400.7), so only static abilities change it; callers ask only where one could change the answer.
pub(crate) fn as_it_would_exist(cx: &mut Cx, entering: &Entering) -> WouldExist {
    let id = entering.object;
    let mark = cx.s().mark();
    let from = cx.s().obj(id).zone;
    // The permanents leaving the battlefield in the same move (CR 101.4: Goblin Welder's sacrifice) are gone as it arrives.
    let leaving = cx.s().leaving_together.borrow().clone();
    for gone in leaving {
        if gone != id && cx.s().obj(gone).zone.is(ZoneKind::Battlefield) && cx.s().battlefield().contains(&gone) {
            cx.st().zone_remove(Zone::BATTLEFIELD, gone);
            cx.st().update_obj(gone, |o| o.zone = Zone::GONE);
        }
    }
    cx.st().zone_remove(from, id);
    let timestamp = cx.st().timestamp();
    // A Room spell enters with the door it was cast for unlocked (CR 709.5d).
    let door = if from.is(ZoneKind::Stack) && cx.cards().get(cx.s().obj(id).card).is_room() { cast_door(cx.s().obj(id).cast_way) } else { 0 };
    let chosen = keep_chosen(cx, entering);
    cx.st().update_obj(id, |o| {
        o.zone = Zone::BATTLEFIELD;
        o.controller = entering.controller;
        o.tapped = entering.tapped;
        o.marks.doors = door;
        o.marks.protector = entering.protector;
        o.transformed = entering.transformed;
        o.timestamp = timestamp;
        counters::hypothetical(o, &entering.counters);
        if entering.copy.is_some() {
            o.copy = entering.copy;
        }
        chosen(o);
        o.attached_to = entering.attach;
        o.attached_player = entering.attach_player;
    });
    cx.st().zone_push(Zone::BATTLEFIELD, id);
    // It arrives as a new object (CR 400.7): of the effects on it where it is, only those a permanent spell carries to the
    // permanent it becomes (CR 400.7a, `carry_effects`) would apply to it there.
    if cx.s().effects.iter().any(|e| e.affected == id) {
        let carried = |e: &cantrip_state::ContinuousEffect| cantrip_api::text_change_carries(from, e.kind);
        let keep: Vec<bool> = cx.s().effects.iter().map(|e| e.affected != id || carried(e)).collect();
        // Taken back below with the rest: no point of the game, so nothing for the state triggers.
        cx.st().effects_keep(&keep);
    }
    // What the effect putting it there says it is (`Entering::becomes`), it would be there.
    if let Some(b) = entering.becomes {
        enter_as(cx, id, b);
    }
    // The haste an instance of riot applied so far gives it (CR 702.136a, 614.12).
    if entering.riot_haste {
        riot_haste(cx, id);
    }
    note_statics(cx, id);
    let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, id);
    let mana_value = cantrip_api::mana_value_as_permanent(&|c| cx.card(c), cx.s().obj(id), &chars);
    let lost = layers::lost_all_abilities(cx.cards(), cx.s(), &cx.g.log, id);
    cx.st().revert(mark);
    WouldExist { object: id, entering: *entering, chars, mana_value, lost }
}

/// Runs `f` with `card` judged as the spell it would be, cast by `player` as `face` (its own card's face where `None`): a
/// spell is put on the stack before its modes and targets are announced (CR 601.2a-c), with the characteristics of the face
/// it is cast as alone (CR 709.3 a split card's half, 715.3 an Adventure, 712.8 a double-faced card's back, 720.3 an Omen)
/// and under its caster's control, so what may be targeted is judged on it there: with every effect that applies to it on
/// the stack - static abilities that read the zone or the controller among them - and none of those on the card where it is
/// (CR 400.7). Worked out by putting it there for the moment of the question, as the cast does, and taking that back.
/// The object is the card's own (no new object), so what `f` asks of `card` it asks of the spell.
pub(crate) fn as_cast<T>(cx: &mut Cx, card: ObjectId, face: Option<CardId>, player: PlayerId, f: impl FnOnce(&mut Cx) -> T) -> T {
    // The position is the same after: what the layers worked out for it is still good (`GameState::set_derived_aside`).
    let aside = cx.s().set_derived_aside();
    let mark = cx.s().mark();
    let from = cx.s().obj(card).zone;
    cx.st().zone_remove(from, card);
    cx.st().update_obj(card, |o| {
        o.zone = Zone::STACK;
        o.controller = player;
        o.cast_from = Some(from.kind());
        o.face_down = false;
        if let Some(face) = face {
            o.copy = Some(Copiable::of(face));
        }
    });
    cx.st().zone_push(Zone::STACK, card);
    if cx.s().effects.iter().any(|e| e.affected == card) {
        let keep: Vec<bool> = cx.s().effects.iter().map(|e| e.affected != card).collect();
        cx.st().effects_keep(&keep);
    }
    let ability = AbilityRef { card: face.unwrap_or(cx.s().obj(card).card), index: u8::MAX };
    cx.st().stack_push(crate::actions::stack_item(StackKind::Spell, card, ability, player, &[]));
    note_statics(cx, card);
    let done = f(cx);
    cx.st().revert(mark);
    cx.st().restore_derived(aside);
    done
}

/// Runs `f` with `entering` judged as it would exist on the battlefield (CR 614.12) by every effect that reads it
/// (`Query::chars`, `Query::lost_all_abilities`): the replacement effects that modify how it enters ("creatures your opponents
/// control enter tapped") and the rules that limit the counters it enters with (CR 122.6). Worked out only where some effect
/// could read it - a replacement effect that could apply, or counters to put on it - and where it could be other than its
/// card's own values where it is: a static ability that could apply to it there (its own or another's), a copy effect or a
/// face it enters with, an effect on it, or a Room (CR 709.5: on the battlefield only its unlocked doors).
///
/// Judged again after each replacement that changes how it enters (`rejudge_entering`).
fn judged_entering<T>(cx: &mut Cx, entering: &Entering, counters: bool, f: impl FnOnce(&mut Cx) -> T) -> T {
    let copy_replaces = copy_over(cx, entering) && cx.cards().replaces[entering_card(cx, entering).0 as usize];
    if !(copy_replaces || crate::replace::possible(cx, Some(entering.object)) || (counters && crate::static_rules::counter_bans_possible(cx.s(), cx.cards())))
    {
        return f(cx);
    }
    let id = entering.object;
    let obj = cx.s().obj(id);
    let differs = layers::static_sources_present(cx.cards(), cx.s())
        || cx.cards().statics[cx.cards().face(obj.card, entering.transformed).0 as usize]
        || entering.copy.is_some()
        || entering.becomes.is_some()
        || obj.copy.is_some()
        || entering.transformed != obj.transformed
        || cx.s().effects.iter().any(|e| e.affected == id)
        || cx.cards().get(obj.card).is_room();
    if !differs {
        return f(cx);
    }
    let would = as_it_would_exist(cx, entering);
    let outer = cx.entering.replace(would);
    let done = f(cx);
    cx.entering = outer;
    done
}

/// After a replacement has modified how a permanent enters (CR 614.12: "taking into account replacement effects that have
/// already modified how it enters"), the ones after it judge it as it would exist now: under the player it now enters
/// under, tapped or not; and one that sends it elsewhere (Containment Priest) leaves it no permanent to judge, so the rest
/// see the card it is. Called by `replace::apply` after each replacement of a zone change.
pub(crate) fn rejudge_entering(cx: &mut Cx, change: &ZoneChange) {
    let Some(would) = cx.entering.filter(|w| w.object == change.object) else { return };
    if !change.to.is(ZoneKind::Battlefield) {
        cx.entering = None;
        return;
    }
    if change.controller != would.entering.controller || change.tapped != would.entering.tapped {
        let entering = Entering { controller: change.controller, tapped: change.tapped, ..would.entering };
        cx.entering = None;
        cx.entering = Some(as_it_would_exist(cx, &entering));
    }
}

/// Who may protect a battle `controller` controls (CR 310.9a, 310.12a), in turn order: for a Siege each opponent still in
/// the game, for a battle of no type its controller alone.
pub fn protector_candidates(cx: &Cx, siege: bool, controller: PlayerId) -> Vec<PlayerId> {
    if siege {
        cx.s().opponents(controller).collect()
    } else if cx.s().player(controller).lost {
        Vec::new()
    } else {
        vec![controller]
    }
}

/// CR 310.9a, 310.12a: a battle's controller chooses its protector as it enters, out of the players its battle type
/// allows, asked only when there is more than one. A choice, not a target. A battle nobody can protect enters with
/// none, and CR 704.5x puts it into its owner's graveyard.
fn choose_protector(cx: &mut Cx, entering: &mut Entering) -> R<()> {
    let def = cx.cards().get(entering_card(cx, entering));
    if !def.is(CardType::Battle) {
        return Ok(());
    }
    let candidates = protector_candidates(cx, def.subtypes.has(Subtype::Siege), entering.controller);
    entering.protector = match candidates.len() {
        0 => None,
        1 => Some(candidates[0]),
        _ => {
            let ask = RequestKind::ChoosePlayer { source: entering.object, candidates: candidates.clone(), verb: "Protect", amount: 0, subject: None };
            match cx.ask_core(entering.controller, ask, false)? {
                Answer::Player(p) if candidates.contains(&p) => Some(p),
                _ => Some(candidates[0]),
            }
        }
    };
    Ok(())
}

/// Puts objects onto the battlefield (CR 110.2a): each one's entering replacements are applied, then they all enter
/// at once. An Aura put onto the battlefield without being cast chooses what it enchants as it enters (CR 303.4f),
/// and one with nothing it could enchant stays where it is (CR 303.4g).
pub fn put_onto_battlefield(cx: &mut Cx, objects: &[(ObjectId, PlayerId, bool)], reason: ZoneReason) -> R<Vec<ObjectId>> {
    put_onto_battlefield_with(cx, objects, reason, &[])
}

/// The counters an effect says a permanent enters with (CR 122.6, 614.1c): "return it with a -1/-1 counter on it".
pub fn entry_counters(with: &[(CounterKind, i32)]) -> [i32; CounterKind::COUNT] {
    let mut counters = [0i32; CounterKind::COUNT];
    for &(kind, n) in with {
        counters[kind as usize] += n;
    }
    counters
}

/// `put_onto_battlefield`, each entering with these counters as well (CR 122.6).
pub fn put_onto_battlefield_with(cx: &mut Cx, objects: &[(ObjectId, PlayerId, bool)], reason: ZoneReason, with: &[(CounterKind, i32)]) -> R<Vec<ObjectId>> {
    put_onto_battlefield_along(cx, Vec::new(), objects, reason, with, None)
}

/// `put_onto_battlefield_with`, in one move with these others (CR 101.4): they leave or arrive at the same moment, so what
/// watches either sees the board once, as it was before and after all of it.
pub fn put_onto_battlefield_along(
    cx: &mut Cx,
    moves: Vec<Moving>,
    objects: &[(ObjectId, PlayerId, bool)],
    reason: ZoneReason,
    with: &[(CounterKind, i32)],
    becomes: Option<Becomes>,
) -> R<Vec<ObjectId>> {
    put_onto_battlefield_into(cx, moves, objects, reason, with, becomes, None, None, &[])
}

/// `put_onto_battlefield` of one object, which enters with the counters the effect putting it there gives it for what it
/// is as it would exist on the battlefield ("if it's a creature", CR 614.1c, 614.12): a replacement effect applied among the
/// others that modify how it enters (`Entering::counters_if`, `replace::entry`).
pub fn put_onto_battlefield_with_counters_if(cx: &mut Cx, id: ObjectId, controller: PlayerId, tapped: bool, counters: CountersIf) -> R<Vec<ObjectId>> {
    put_onto_battlefield_into(cx, Vec::new(), &[(id, controller, tapped)], ZoneReason::Other, &[], None, Some(counters), None, &[])
}

/// A named player, separately from its entering controller, performs each put (CR 110.2a, 608.2c).
pub fn put_onto_battlefield_by(cx: &mut Cx, objects: &[(ObjectId, PlayerId, bool, PlayerId)], with: &[(CounterKind, i32)]) -> R<Vec<ObjectId>> {
    let rows: Vec<_> = objects.iter().map(|&(id, controller, tapped, _)| (id, controller, tapped)).collect();
    let actors: Vec<_> = objects.iter().map(|&(id, _, _, by)| (id, PutBy::Player(by))).collect();
    put_onto_battlefield_into(cx, Vec::new(), &rows, ZoneReason::Other, with, None, None, None, &actors)
}

fn put_onto_battlefield_into(
    cx: &mut Cx,
    mut moves: Vec<Moving>,
    objects: &[(ObjectId, PlayerId, bool)],
    reason: ZoneReason,
    with: &[(CounterKind, i32)],
    becomes: Option<Becomes>,
    counters_if: Option<CountersIf>,
    attacking: Option<Target>,
    putters: &[(ObjectId, PutBy)],
) -> R<Vec<ObjectId>> {

    // What leaves the battlefield in this move is gone as each entering permanent is judged (CR 101.4, 614.12).
    let leaving: Vec<ObjectId> = moves.iter().map(|m| m.id).filter(|&id| cx.s().obj(id).zone.is(ZoneKind::Battlefield)).collect();
    let outer_leaving = std::mem::replace(&mut *cx.s().leaving_together.borrow_mut(), leaving);
    let prepared = prepare_entries(cx, &mut moves, objects, with, becomes, counters_if);
    *cx.s().leaving_together.borrow_mut() = outer_leaving;
    prepared?;
    if !putters.is_empty() {
        for moving in &mut moves {
            if let Some((_, by)) = putters.iter().find(|(id, _)| *id == moving.id) { moving.put_by = *by; }
        }
    }
    crate::attach::choose_aura_hosts(cx, &mut moves)?;
    Ok(move_together_entering(cx, &moves, reason, attacking).0)
}

/// Whether `id` would enter the battlefield as an instant or sorcery card, which can't (CR 304.4, 307.4, 110.4, 400.4a): if
/// one would, it stays in the zone it is in instead. Judged by the face it would enter with - its back face if it enters
/// transformed (CR 712.14a: Esper Origins puts itself there as its Saga back face), its front face otherwise, so a modal
/// double-faced card put onto the battlefield whose front face is an instant or sorcery stays where it is (CR 712.14b) while
/// one played as its land back face enters (CR 712.12) - and a card turned face down to be manifested or cloaked as the
/// face-down creature it then is (CR 701.40a, 701.58a: an instant card may be manifested; turned face up it stays face
/// down, CR 701.40g). The card's own face, not what a copy or type-setting effect would make it as it enters: the example
/// of CR 712.13a, a sorcery face entering as a copy of a creature, still can't.
pub(crate) fn instant_or_sorcery_entering(cx: &Cx, id: ObjectId, transformed: bool) -> bool {
    let obj = cx.s().obj(id);
    if obj.copy.is_some_and(|c| cx.card(c.card).face_down) {
        return false;
    }
    instant_or_sorcery(cx.card(cx.cards().face(obj.card, transformed)))
}

/// An instant or sorcery card face (CR 304, 307), which no permanent can be (CR 110.4).
pub(crate) fn instant_or_sorcery(def: &CardDef) -> bool {
    def.types.has(CardType::Instant) || def.types.has(CardType::Sorcery)
}

/// Each of `objects` prepared to enter (`prepare_entry`) and added to `moves`.
fn prepare_entries(
    cx: &mut Cx,
    moves: &mut Vec<Moving>,
    objects: &[(ObjectId, PlayerId, bool)],
    with: &[(CounterKind, i32)],
    becomes: Option<Becomes>,
    counters_if: Option<CountersIf>,
) -> R<()> {
    let bans = crate::static_rules::enter_bans(cx);
    for &(id, controller, tapped) in objects {
        // A card that has changed zones since it was chosen is a new object, not the one to put onto the battlefield
        // (CR 400.7), as for the other moves of a chosen object.
        if !live(cx.s(), id) {
            continue;
        }
        // A card a static ability keeps from entering stays where it is (CR 101.2, 101.3): nothing of its entering is asked
        // or applied, as no entering happens.
        // Nor does an instant or sorcery card (CR 304.4, 307.4).
        if (!bans.is_empty() && crate::static_rules::enter_banned(cx, &bans, id)) || instant_or_sorcery_entering(cx, id, false) {
            continue;
        }
        let mut entering =
            Entering { object: id, controller, tapped, counters: entry_counters(with), copy: None, prepared: false, chosen_color: None, chosen_subtype: None, chosen_player: None, chosen_card: None, chosen_option: None, chosen_number: None, chosen_object: None, emblem: None, attach: None, attach_player: None, x: 0 , transformed: false, protector: None, read_ahead: None, riot_haste: false, instead: None, becomes, own: None, melded: None, counters_if };
        prepare_entry(cx, &mut entering)?;
        // One its own daybound turned over as it would enter (CR 702.145b) is judged as that face (CR 304.4, 307.4).
        if entering.transformed && instant_or_sorcery_entering(cx, id, true) {
            continue;
        }
        moves.push(Moving { entering: Some(entering), ..Moving::to(id, Zone::BATTLEFIELD) });
    }
    Ok(())
}

/// Sacrifices permanents and returns cards to the battlefield at the same moment (CR 101.4, 701.21): Goblin Welder's "simultaneously
/// sacrifices the artifact and returns the artifact card". One move, so the returning permanent's entering abilities see the
/// sacrificed one already gone, and the sacrificed one's leaving abilities see its replacement already there; each entering
/// replacement is applied as for any entry (CR 614.12). `player` sacrifices, and the sacrificed are moved for the reason
/// `Sacrifice`; one they can't sacrifice (`sacrificeable`) stays, and the cards are returned all the same (CR 101.2: only that
/// part of the effect does nothing).
pub fn sacrifice_and_return(cx: &mut Cx, player: PlayerId, sacrificed: &[ObjectId], returned: &[(ObjectId, PlayerId)]) -> R<()> {
    let moves: Vec<Moving> = sacrificed
        .iter()
        .copied()
        .filter(|&id| sacrificeable(cx, player, id))
        .map(|id| Moving { reason: Some(ZoneReason::Sacrifice), ..Moving::to(id, Zone::graveyard(cx.s().obj(id).owner)) })
        .collect();
    let arriving: Vec<(ObjectId, PlayerId, bool)> = returned.iter().map(|&(id, p)| (id, p, false)).collect();
    put_onto_battlefield_along(cx, moves, &arriving, ZoneReason::Other, &[], None)?;
    Ok(())
}

/// Puts one object onto the battlefield, with its back face up if it enters transformed (CR 712.14a), and entering with the
/// counters `with` names as well (CR 122.6, 614.1c).
pub fn put_onto_battlefield_as(
    cx: &mut Cx,
    id: ObjectId,
    controller: PlayerId,
    tapped: bool,
    transformed: bool,
    reason: ZoneReason,
    with: &[(CounterKind, i32)],
) -> R<ObjectId> {
    // Only a double-faced card has a back face to enter with: any other stays where it is (CR 712.14a).
    if transformed && cx.cards().backs[cx.s().obj(id).card.0 as usize].is_none() {
        return Ok(id);
    }
    // Nor does one a static ability keeps from entering (CR 101.2, 101.3), nor an instant or sorcery face (CR 304.4, 307.4).
    if crate::static_rules::cant_enter(cx, id) || instant_or_sorcery_entering(cx, id, transformed) {
        return Ok(id);
    }
    let mut entering = Entering {
        object: id,
        controller,
        tapped,
        counters: entry_counters(with),
        copy: None,
        prepared: false,
        chosen_color: None,
        chosen_subtype: None,
        chosen_player: None,
        chosen_card: None,
        chosen_option: None,
        chosen_number: None,
        chosen_object: None, emblem: None,
        attach: None, attach_player: None,
        x: 0,
        transformed,
        protector: None, read_ahead: None, riot_haste: false,
        instead: None,
        becomes: None,
        own: None,
        melded: None,
        counters_if: None,
    };
    prepare_entry(cx, &mut entering)?;
    // Turned over by its own daybound as it would enter (CR 702.145b), it is judged as that face (CR 304.4, 307.4).
    if entering.transformed && !transformed && instant_or_sorcery_entering(cx, id, true) {
        return Ok(id);
    }
    Ok(move_together(cx, &[Moving { entering: Some(entering), ..Moving::to(id, Zone::BATTLEFIELD) }], reason).first().copied().unwrap_or(id))
}

/// Meld (CR 701.42a, 712.14c): puts `first` and `second`, the two cards of a meld pair, onto the battlefield under
/// `controller`'s control with their back faces up and combined - one permanent, `first`'s card its own, entering as the
/// pair's combined back face, whose characteristics alone it has (CR 712.8g): its own "enters" replacements are that face's,
/// as a permanent entering as a copy has the copy's (CR 614.12). Only the two cards of one meld pair are melded: a token, a
/// copy of a card, or two cards that are no pair stay where they are (CR 701.42b-c), as do the two where a static ability
/// keeps the permanent from entering (CR 101.2). The melded permanent, if they melded.
pub fn meld(cx: &mut Cx, first: ObjectId, second: ObjectId, controller: PlayerId) -> R<Option<ObjectId>> {
    if first == second || !live(cx.s(), first) || !live(cx.s(), second) {
        return Ok(None);
    }
    let (a, b) = (*cx.s().obj(first), *cx.s().obj(second));
    let pair = |x: &Obj, y: &Obj| !x.token && !x.card_copy && cx.card(x.card).meld.is_some_and(|m| m.with == cx.card(y.card).name);
    let Some(into) = cx.card(a.card).meld.filter(|_| pair(&a, &b) && pair(&b, &a)).map(|m| cx.cards().id_of(m.into)) else {
        return Ok(None);
    };
    if crate::static_rules::cant_enter(cx, first) {
        return Ok(None);
    }
    let mut entering = Entering {
        object: first,
        controller,
        tapped: false,
        counters: [0; CounterKind::COUNT],
        copy: Some(Copiable::of(into)),
        prepared: false,
        chosen_color: None,
        chosen_subtype: None,
        chosen_player: None,
        chosen_card: None,
        chosen_option: None,
        chosen_number: None,
        chosen_object: None,
        emblem: None,
        attach: None, attach_player: None,
        x: 0,
        transformed: false,
        protector: None, read_ahead: None, riot_haste: false,
        instead: None,
        becomes: None,
        own: None,
        melded: Some(second),
        counters_if: None,
    };
    prepare_entry(cx, &mut entering)?;
    let entered = move_together(cx, &[Moving { entering: Some(entering), ..Moving::to(first, Zone::BATTLEFIELD) }], ZoneReason::Other);
    Ok(entered.first().copied().filter(|&id| cx.s().obj(id).zone.is(ZoneKind::Battlefield) && cx.s().merge_of(id).is_some()))
}

/// The continuous effects an effect putting `id` onto the battlefield gives it as it enters (`EntersAs`, CR 614.12, 613.1d,
/// 613.1f): its types (layer 4), then the abilities it loses and gains (layer 6), each at a timestamp of its own in that
/// order (CR 613.7a), for as long as it is this object (CR 611.2a, 400.7). Its source is the permanent itself, under its
/// controller: none of these reads either.
fn enter_as(cx: &mut Cx, id: ObjectId, b: Becomes) {
    let controller = cx.s().obj(id).controller;
    let mut kinds = Vec::new();
    // "It's an Aura enchantment": an enchantment alone, whose enchantment type is Aura alone (CR 205.1a).
    if b.what.aura {
        kinds.push(EffectKind::SetTypes(TypeSet::of(&[CardType::Enchantment])));
        kinds.push(EffectKind::LoseSubtypesOf(CardType::Enchantment));
        kinds.push(EffectKind::AddSubtype(Subtype::Aura));
    } else {
        if let Some(types) = b.what.types {
            kinds.push(EffectKind::SetTypes(types));
        }
        // "It's a Robot artifact creature" (CR 205.1b): the types gained, every other card type kept.
        if b.what.adds_types != TypeSet::of(&[]) {
            kinds.push(EffectKind::AddTypes(b.what.adds_types));
        }
        // "(They lose all other types and subtypes.)": then the subtypes it gains (CR 205.1a).
        kinds.extend(b.what.loses_subtypes_of.iter().map(|&t| EffectKind::LoseSubtypesOf(t)));
        kinds.extend(b.what.subtypes.iter().map(|&s| EffectKind::AddSubtype(b.words.subtype(s))));
    }
    if b.what.loses_other_abilities {
        kinds.push(EffectKind::LoseAllAbilities);
    }
    // The enchant ability its card describes; a card that describes none gives none.
    if b.what.aura && cx.card(b.card).becomes_aura_enchant.is_some() {
        kinds.push(EffectKind::Enchant(b.card));
    }
    // Only abilities its card lists: an index past its `delayed` list names no ability, and gives none.
    let listed = cx.card(b.card).delayed.len();
    kinds.extend(
        b.what.abilities.iter().filter(|&&i| usize::from(i) < listed).map(|&i| EffectKind::AddAbility(AbilityRef::delayed(b.card, i))),
    );
    // "It's a 3/3 ... creature with flying": the keywords it gains (layer 6), then its power and toughness (layer 7b).
    if b.what.keywords != Keywords::of(&[]) {
        kinds.push(EffectKind::AddKeywords(b.what.keywords));
    }
    if let Some((power, toughness)) = b.what.power_toughness {
        kinds.push(EffectKind::SetPt(power, toughness));
    }
    for kind in kinds {
        let timestamp = cx.st().timestamp();
        // An ability it gains is the text's that puts it onto the battlefield so (CR 611.2c, 612.1).
        let words = if matches!(kind, EffectKind::AddAbility(_)) { b.words } else { Words::NONE };
        cx.st().effect_push(cantrip_state::ContinuousEffect {
            source: id,
            controller,
            timestamp,
            affected: id,
            kind,
            duration: Duration::Indefinite,
            words,
        });
        // A static ability given to it that affects other objects makes it a source of static abilities.
        if let EffectKind::AddAbility(ability) = kind
            && matches!(cx.cards().ability(ability), AbilityDef::Static(s) if !s.own)
        {
            cx.st().see_statics();
        }
    }
}

/// Puts one object onto the battlefield as an effect says it is (`Game::put_onto_battlefield_as`, CR 614.12): the batch of
/// one. The permanent, or None if it did not enter.
pub fn put_onto_battlefield_becoming(cx: &mut Cx, id: ObjectId, controller: PlayerId, becomes: Becomes) -> R<Option<ObjectId>> {
    Ok(put_onto_battlefield_becoming_all(cx, &[(id, controller, false)], &[], becomes)?.first().copied())
}

/// Puts objects onto the battlefield together as an effect says they are (`Game::put_all_onto_battlefield_becoming`, CR 110.2a,
/// 614.12): each one's entering replacements judged on it as that (`as_it_would_exist`), as an Aura choosing what it enchants as
/// it enters and staying where it is with nothing to enchant (CR 303.4f-g), and all arriving as that in one move, each with the counters `with` names (CR 122.6, 614.1c). A card a
/// static ability keeps from entering stays where it is (CR 101.2, 101.3), judged as the card it is there, not as what the
/// effect would make it (Grafdigger's Cage's ruling); nor does an instant or sorcery card, whatever the effect says it is
/// (CR 304.4, 307.4). The permanents that entered.
pub fn put_onto_battlefield_becoming_all(cx: &mut Cx, objects: &[(ObjectId, PlayerId, bool)], with: &[(CounterKind, i32)], becomes: Becomes) -> R<Vec<ObjectId>> {
    let moved = put_onto_battlefield_along(cx, Vec::new(), objects, ZoneReason::Other, with, Some(becomes))?;
    Ok(moved.into_iter().filter(|&n| live(cx.s(), n) && cx.s().obj(n).zone.is(ZoneKind::Battlefield)).collect())
}

/// Puts one object onto the battlefield attached to `host` (`Game::put_onto_battlefield_attached`): the batch of one
/// (`put_all_onto_battlefield_attached`). The permanent, or None if it did not enter.
pub fn put_onto_battlefield_attached(cx: &mut Cx, id: ObjectId, controller: PlayerId, host: ObjectId) -> R<Option<ObjectId>> {
    Ok(put_all_onto_battlefield_attached(cx, &[(id, controller, host)])?.first().copied().flatten())
}

/// Puts objects onto the battlefield together, each under the player given and attached to the host given
/// (`Game::put_all_onto_battlefield_attached`, CR 303.4f, 303.4i, 301.5e, 101.4): the effect names what each Aura enchants, so
/// nothing is chosen (the choice of `choose_aura_hosts` is for an effect that does not), and they all arrive in one move, one
/// event (CR 603.6a: a "one or more" ability sees them once).
///
/// Each one's entering replacements see it attached to its host, as it would exist on the battlefield (CR 614.12), and each is
/// judged so under its controller against the game as it is before any of them moves: an Aura its host is not a legal object
/// for - by its enchant abilities, because it has protection from the Aura, because it is gone (CR 400.7), or because it is
/// itself one of the objects entering - stays where it is (CR 303.4i), while the others enter. What the ones entering together
/// do to one another's hosts once there (an Aura that gives its creature protection from another) is the state-based actions'
/// to judge (CR 704.5m). An Equipment enters attached only if it could equip its host, unattached otherwise (CR 301.5e), and
/// anything else enters unattached (CR 303.4h). A card a static ability keeps from entering stays where it is (CR 101.2,
/// 101.3), as does an instant or sorcery card (CR 304.4, 307.4). For each object in order, the permanent, or None if it did not
/// enter.
pub fn put_all_onto_battlefield_attached(cx: &mut Cx, objects: &[(ObjectId, PlayerId, ObjectId)]) -> R<Vec<Option<ObjectId>>> {
    let bans = crate::static_rules::enter_bans(cx);
    let mut prepared: Vec<(usize, Entering)> = Vec::with_capacity(objects.len());
    for (i, &(id, controller, host)) in objects.iter().enumerate() {
        // An object moves once however many times it is named (CR 400.7), with the first host named.
        if prepared.iter().any(|(_, e)| e.object == id) {
            continue;
        }
        let Some(mut entering) = entering_attached(cx, &bans, id, controller, host) else { continue };
        prepare_entry(cx, &mut entering)?;
        if entering.transformed && instant_or_sorcery_entering(cx, id, true) {
            continue;
        }
        prepared.push((i, entering));
    }
    let entering_ids: Vec<ObjectId> = prepared.iter().map(|(_, e)| e.object).collect();
    let mut moves: Vec<Moving> = Vec::with_capacity(prepared.len());
    let mut moved: Vec<usize> = Vec::with_capacity(prepared.len());
    for (i, mut entering) in prepared {
        match attached_entry(cx, &entering, &entering_ids) {
            AttachedEntry::Stays => continue,
            AttachedEntry::Unattached => entering.attach = None,
            AttachedEntry::Attached => {}
        }
        moves.push(Moving { entering: Some(entering), ..Moving::to(entering.object, Zone::BATTLEFIELD) });
        moved.push(i);
    }
    let mut out = vec![None; objects.len()];
    if moves.is_empty() {
        return Ok(out);
    }
    let new = move_together(cx, &moves, ZoneReason::Other);
    for (&i, &n) in moved.iter().zip(&new) {
        if live(cx.s(), n) && cx.s().obj(n).zone.is(ZoneKind::Battlefield) {
            out[i] = Some(n);
        }
    }
    Ok(out)
}

/// How an object put onto the battlefield attached to a host the effect names enters (CR 303.4i, 301.5e, 303.4h).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AttachedEntry {
    /// Attached to its host.
    Attached,
    /// Unattached: an Equipment that could not equip its host, or anything that is neither Aura nor Equipment.
    Unattached,
    /// Not at all: an Aura its host is not a legal object for stays where it is.
    Stays,
}

/// `id` about to be put onto the battlefield under `controller` attached to `host`, before its own entering replacements
/// are applied (`prepare_entry`): None if it can't be put there at all - it is no longer the object it was (CR 400.7), one of
/// `bans` (`static_rules::enter_bans`) keeps it where it is (CR 101.2, 101.3), or it is an instant or sorcery card (CR 304.4,
/// 307.4). Shared by the move (`put_all_onto_battlefield_attached`) and the question asked before it (`could_enter_attached`).
fn entering_attached(cx: &Cx, bans: &[(ObjectId, PlayerId, &'static CardDef, Option<Linked>)], id: ObjectId, controller: PlayerId, host: ObjectId) -> Option<Entering> {
    if !live(cx.s(), id) || (!bans.is_empty() && crate::static_rules::enter_banned(cx, bans, id)) || instant_or_sorcery_entering(cx, id, false) {
        return None;
    }
    Some(Entering {
        object: id,
        controller,
        tapped: false,
        counters: [0; CounterKind::COUNT],
        copy: None,
        prepared: false,
        chosen_color: None,
        chosen_subtype: None,
        chosen_player: None,
        chosen_card: None,
        chosen_option: None,
        chosen_number: None,
        chosen_object: None,
        emblem: None,
        attach: Some(host),
        attach_player: None,
        x: 0,
        transformed: false,
        protector: None,
        read_ahead: None,
        riot_haste: false,
        instead: None,
        becomes: None,
        own: None,
        melded: None,
        counters_if: None,
    })
}

/// How `entering` (attached to its host, `Entering::attach`) would enter, judged on it as it would exist on the battlefield
/// (CR 614.12) - seeing it attached to its host - against the game as it is before it moves, `entering_ids` the objects
/// entering with it (`attach::enters_attachable`), or attached to a player (`Entering::attach_player`: an Aura spell resolving,
/// CR 608.3c; `attach::enters_attachable_to_player`). The one judgement of the move (`put_all_onto_battlefield_attached`), of
/// the question asked before it (`could_enter_attached`) and of an Aura spell resolving (CR 303.4i, `resolve`).
pub(crate) fn attached_entry(cx: &mut Cx, entering: &Entering, entering_ids: &[ObjectId]) -> AttachedEntry {
    if entering.attach.is_none() && entering.attach_player.is_none() {
        return AttachedEntry::Unattached;
    }
    let prospective = as_it_would_exist(cx, entering);
    let it = prospective.chars;
    let attachable = match (entering.attach, entering.attach_player) {
        (Some(host), _) => crate::attach::enters_attachable(cx, entering.object, &it, entering.controller, prospective.mana_value, host, entering_ids),
        (None, Some(player)) => crate::attach::enters_attachable_to_player(cx, entering.object, &it, entering.controller, player),
        (None, None) => false,
    };
    match (attachable, it.has_subtype(Subtype::Aura)) {
        (true, _) => AttachedEntry::Attached,
        (false, true) => AttachedEntry::Stays,
        (false, false) => AttachedEntry::Unattached,
    }
}

/// Whether `object`, from wherever it is, could be put onto the battlefield under `controller` attached to `host` now
/// (`Game::could_enter_attached`): what `put_onto_battlefield_attached` would do with it, asked before it is chosen. It can
/// be put there (`entering_attached`), and as it would exist there it could be attached to `host` (`attached_entry`): an
/// Aura one its enchant abilities allow (CR 303.4f, 702.5a), that has no protection from it (CR 702.16c), that is not the
/// Aura itself, and while the Aura would be no creature (CR 303.4d); hexproof does not matter, as it is not targeted. Judged
/// as the move judges it, with nothing entering beside it. The one step of the move it leaves out is the object's own
/// entering replacements (`prepare_entry`), whose choices (CR 614.12a: a Clone's copy, read ahead, a battle's protector) are
/// made as it moves and can't be asked before it is chosen.
pub fn could_enter_attached(cx: &mut Cx, object: ObjectId, controller: PlayerId, host: ObjectId) -> bool {
    let bans = crate::static_rules::enter_bans(cx);
    entering_attached(cx, &bans, object, controller, host)
        .is_some_and(|entering| attached_entry(cx, &entering, &[object]) == AttachedEntry::Attached)
}

/// Transforms a permanent (CR 701.27a): its other face is up. The same object, so nothing about it is new (CR 712.18).
/// Only a double-faced card on the battlefield transforms; a copy of one does not (CR 701.27c).
///
/// A permanent with daybound or nightbound "can't transform except due to its daybound (nightbound) ability"
/// (CR 702.145b, 702.145e): day and night turn it over (`turn_over`), no effect does.
///
/// `by` is the spell or ability whose instruction this is, when the caller names it (`Game::transform_by`); otherwise the
/// one resolving (`Cx::resolving`), so the rule below holds for every caller.
pub fn transform(cx: &mut Cx, id: ObjectId, by: Option<&StackItem>) -> R<()> {
    let by = by.copied().or_else(|| cx.resolving().copied());
    if !live(cx.s(), id)
        || crate::day_night::role(cx.chars(id).keywords).is_some()
        || by.is_some_and(|item| transformed_since(cx, &item, id))
    {
        return Ok(());
    }
    turn_over(cx, id).map(|_| ())
}

/// CR 701.27f: "If an activated or triggered ability of a permanent that isn't a delayed triggered ability of that
/// permanent tries to transform it, the permanent does so only if it hasn't transformed or converted since the ability
/// was put onto the stack. If a delayed triggered ability of a permanent tries to transform that permanent, the permanent
/// does so only if it hasn't transformed or converted since that delayed triggered ability was created." The resolving
/// ability's `StackItem::since` is the history's head at that moment (`Delayed::since` for a delayed one), so the question
/// is whether the history has recorded the permanent transforming from there on. An ability of another object, and a
/// spell, transform it whatever has happened (Tetzin's "transform up to one other target double-faced artifact"). The same
/// object only: one that left and came back is a new object (CR 400.7), and the instruction finds nothing to transform.
/// Aang, at the Crossroads's delayed ability (its 2025-10-02 ruling); the activated transforms of the Incubator token,
/// Ulvenwald Oddity, Aang, Swift Savior, Norman Osborn and Temple of the Dead; Dowsing Device's and Cecil, Dark Knight's
/// triggered ones.
fn transformed_since(cx: &Cx, item: &StackItem, id: ObjectId) -> bool {
    item.kind != StackKind::Spell
        && item.object == id
        && History::with_receipts(&cx.g.log, cx.s().turn.number, &cx.s().cost_receipts).all()[(item.since as usize).min(cx.g.log.len())..]
            .iter()
            .any(|r| matches!(r.event, Event::Transformed { object } if object == id))
}

/// Turns a permanent over (CR 701.27a), once whatever is doing it has established that it may.
///
/// CR 712.20: "If a double-faced card would have an 'As [this permanent] transforms . . .' ability after it transforms or
/// converts, that ability is applied while that permanent is transforming or converting, not afterward." So the ability
/// is read from the permanent as it is once turned over - the face now up (`CardDef::as_transforms`), unless it has lost
/// all its abilities there (CR 613.1f) - and applied here, before the transformation's event, by whatever turned it
/// over. Entering the battlefield transformed is no transformation (CR 712.14a) and never comes here.
///
/// Only a permanent represented by a double-faced card or token transforms (CR 701.27a, 701.27c, 712.9): a card with a
/// copy effect on it is still that card (Glasspool Mimic as a copy of a daybound creature), and stays that copy with its
/// other face up (CR 707.2); a token made as a copy of a double-faced permanent is double-faced, each face under its
/// exceptions (CR 707.8a); a token made of one face alone is not, nor is a face-down permanent. Nor does one transform
/// whose other face is an instant or sorcery face (CR 701.27d, 712.10). Whether it turned over.
pub fn turn_over(cx: &mut Cx, id: ObjectId) -> R<bool> {
    if !live(cx.s(), id) {
        return Ok(false);
    }
    let obj = cx.s().obj(id);
    // A merged permanent is no double-faced permanent, but transforming it turns each of its double-faced components that can
    // transform (CR 730.2i).
    if cx.s().merge_of(id).is_some() {
        if !obj.zone.is(ZoneKind::Battlefield) || obj.marks.face_down_as.is_some() || !crate::merge::turn_components(cx, id) {
            return Ok(false);
        }
    } else {
        if !obj.zone.is(ZoneKind::Battlefield)
            || obj.marks.face_down_as.is_some()
            || !obj.represented_double_faced(cx.cards().backs[obj.card.0 as usize].is_some())
            || instant_or_sorcery(cx.card(cx.cards().face(obj.card, !obj.transformed)))
        {
            return Ok(false);
        }
        cx.st().update_obj(id, |o| o.transformed = !o.transformed);
    }
    note_statics(cx, id);
    // The face now up, or what a copy effect makes it (CR 707.2), and a merged permanent's other components' faces (CR
    // 702.140e, 730.2i), each asked as its own card's.
    let texts: Vec<(CardId, Option<Linked>)> = cx.cards().texts_linked(cx.s(), id).collect();
    for (face, by) in texts {
        if let Some(as_transforms) = cx.cards().get(face).as_transforms
            && !cx.lost_all_abilities(id)
        {
            let controller = cx.s().obj(id).controller;
            as_transforms(cx, &Asker { instance: None, source: id, controller, by, spell: false })?;
        }
    }
    cx.emit(Event::Transformed { object: id });
    Ok(true)
}

/// Creates an emblem (CR 114.2): an object in the command zone, owned by the player, that nothing moves. `subtype` is the
/// creature type its text says "chosen" (CR 607.2d), kept as a permanent keeps what it chose as it entered (`Choices`). `maker`
/// is the ability that makes it, where its text names that object (`Kept::Maker`).
pub fn create_emblem(cx: &mut Cx, def: CardId, player: PlayerId, subtype: Option<Subtype>, maker: Option<Linked>) -> ObjectId {
    // The emblem's abilities are the creating text's (CR 114.2, 612.4): words that text changed are changed in them. An
    // ability's text is as it was put on the stack (`Cx::own_text`, CR 113.7a).
    let words = match cx.cause() {
        Cause::Spell(source) | Cause::Ability { source, .. } => Query::words(&*cx, source),
        _ => Words::NONE,
    };
    emblem_with_words(cx, def, player, words, subtype, maker)
}

/// The emblem `entering` gets its controller as it arrives as `permanent` (`Entering::emblem`, CR 614.12, 114.2): the creating
/// text is the permanent's own - the text it entered with, acquired as its own (CR 607.5, 707.9a), and its words - and
/// `permanent`, the new object it is by now, is the object it names (CR 400.7).
fn emblem_as_it_arrives(cx: &mut Cx, entering: &Entering, permanent: ObjectId) {
    let Some(def) = entering.emblem else { return };
    let maker = Linked { object: permanent, text: cx.cards().linked_card(entering_card(cx, entering)), acquired: Acquired::OWN };
    let words = Query::words(&*cx, permanent);
    let player = cx.s().obj(permanent).controller;
    emblem_with_words(cx, def, player, words, None, Some(maker));
}

fn emblem_with_words(cx: &mut Cx, def: CardId, player: PlayerId, words: Words, subtype: Option<Subtype>, maker: Option<Linked>) -> ObjectId {
    let copy = (!words.is_printed()).then(|| Copiable { words, ..Copiable::of(def) });
    let turn = cx.s().turn.number;
    let timestamp = cx.st().timestamp();
    let mut obj = Obj::new(def, player, Zone::COMMAND);
    obj.controlled_since = turn;
    obj.timestamp = timestamp;
    obj.copy = copy;
    if subtype.is_some() {
        obj.marks.choices = Choices::entered(cx.cards().linked_card(def), Entered { subtype, ..Entered::default() });
    }
    if let Some(by) = maker {
        // An emblem's first keeping: there is always room (`CHOSEN_KEPT`).
        obj.marks.choices.keep(by, Kept::Maker(by.object));
    }
    let id = cx.st().push_obj(obj);
    cx.st().zone_push(Zone::COMMAND, id);
    note_statics(cx, id);
    id
}

/// The copiable values of the tokens a creation makes (CR 707.2): one set for every token, or each token's own.
#[derive(Clone, Copy)]
enum Copies<'a> {
    All(Option<Copiable>),
    Each(&'a [Copiable]),
}

impl Copies<'_> {
    fn of(&self, token: usize) -> Option<Copiable> {
        match self {
            Copies::All(copy) => *copy,
            Copies::Each(each) => each.get(token).copied(),
        }
    }
}

/// Creates tokens that are copies of different permanents as one instruction (CR 707.2, 111.1, 603.6a): each is a copy of
/// its own copiable values, all enter together and what watches them sees one event (CR 603.2c), so each token sees the
/// others enter (the Face Yourself ruling). `copies` are the values and who controls the token.
pub fn create_token_copies_together(cx: &mut Cx, copies: &[(Copiable, PlayerId)]) -> R<Vec<ObjectId>> {
    let tokens: Vec<(CardId, PlayerId)> = copies.iter().map(|&(values, controller)| (values.card, controller)).collect();
    let values: Vec<Copiable> = copies.iter().map(|&(values, _)| values).collect();
    create_tokens_onto(cx, &tokens, false, Copies::Each(&values), &[], Words::NONE, None, None, None)
}

/// One token for each of `entries`, created together as one instruction (CR 101.4, 111.1) and each entering attacking
/// what its own entry names (CR 508.4): "for each opponent, create a token ... attacking that player". They are one event,
/// so what watches tokens entering sees them together (CR 603.2c), and each is attacking as its entry is matched
/// (CR 603.6a); none was declared as an attacker (CR 506.3, 508.4), so nothing that triggers on attackers being declared
/// sees it. A copy of `copy`'s values when there is one (CR 707.2: Mardu Siegebreaker; Shredder, Shadow Master; myriad),
/// otherwise `card` with the words of the text making it (CR 612.4: Adeline, Resplendent Cathar; Hero of Bladehold;
/// Stampede Surfer). A replacement changes the one event (CR 614.1a), and the tokens it makes are created with what the
/// effect specifies, attacking included (the Chatterfang and Divine Visitation rulings): a further token made for an entry
/// (Doubling Season's) attacks what that entry allows (`EntersAttacking::others`), and one the replacement adds of its own
/// (Quina's Frog, Chatterfang's Squirrels) what its controller chooses (`enter_attacking_each`).
pub fn create_tokens_attacking_each(cx: &mut Cx, card: CardId, copy: Option<Copiable>, controller: PlayerId, tapped: bool, entries: &[EntersAttacking]) -> R<Vec<ObjectId>> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let words = creating_words(cx, copy);
    let card = copy.map_or(card, |values| values.card);
    let tokens = vec![(card, controller); entries.len()];
    create_tokens_onto(cx, &tokens, tapped, Copies::All(copy), &[], words, None, None, Some(AttackEntry::Each(entries)))
}

/// The words of the text creating tokens (CR 612.4, 612.2a): a token is what the text that makes it says, so a changed text
/// makes tokens of the colors and creature types it names now - an ability's text as it was put on the stack
/// (`Cx::own_text`, CR 113.7a). A copy is its copiable values, and takes none.
fn creating_words(cx: &Cx, copy: Option<Copiable>) -> Words {
    match cx.cause() {
        Cause::Spell(source) | Cause::Ability { source, .. } if copy.is_none() => Query::words(cx, source),
        _ => Words::NONE,
    }
}

/// Creates tokens that are copies of a permanent (CR 707.2, 111.1): the permanent's copiable values, its own copy
/// effect's exceptions included.
pub fn create_token_copies(cx: &mut Cx, values: Copiable, controller: PlayerId, count: u32) -> R<Vec<ObjectId>> {
    create_tokens_as(cx, values.card, controller, count, false, Some(values), &[])
}

/// `create_token_copies`, tapped as they enter when `tapped` (CR 110.5b, 614.1c): myriad's "token copy that's tapped and
/// attacking" (CR 702.116a) is created tapped, not tapped afterwards.
pub fn create_token_copies_tapped(cx: &mut Cx, values: Copiable, controller: PlayerId, count: u32, tapped: bool) -> R<Vec<ObjectId>> {
    create_tokens_as(cx, values.card, controller, count, tapped, Some(values), &[])
}

/// Thousand-Faced Shadow and Flamerush Rider make token copies entering tapped and attacking (CR 508.4, 707.2).
/// The explicit defender, or each token's controller's choice, is established before ETB matching (CR 603.6a).
pub fn create_token_copies_attacking(cx: &mut Cx, values: Copiable, controller: PlayerId, count: u32, tapped: bool, defender: Option<Target>) -> R<Vec<ObjectId>> {
    let tokens = vec![(values.card, controller); count as usize];
    let attack = defender.map_or(AttackEntry::Choose, AttackEntry::Defender);
    create_tokens_onto(cx, &tokens, tapped, Copies::All(Some(values)), &[], Words::NONE, None, None, Some(attack))
}

/// An object that ceases to exist where it is, in no zone it can be in (CR 111.7, 111.8, 707.10a, 704.5d, 800.4a), as it
/// last existed there (CR 608.2h).
pub fn cease_to_exist(cx: &mut Cx, zone: Zone, id: ObjectId) {
    cease_together(cx, &[(zone, id)]);
}

/// Objects that cease to exist at once (CR 800.4a: a leaving player's; CR 704.3: tokens and copies the state-based actions
/// end together): each one's last-known information is taken before any of them is gone, so none misses the effects of
/// another (a Glorious Anthem leaving with the creatures it pumped).
pub fn cease_together(cx: &mut Cx, objects: &[(Zone, ObjectId)]) {
    let rows: Vec<LastKnown> = objects.iter().map(|&(_, id)| last_known_of(cx, id)).collect();
    for (&(zone, id), row) in objects.iter().zip(rows) {
        let row = cx.st().last_known_push(row);
        cx.st().zone_remove(zone, id);
        cx.st().update_obj(id, |o| {
            o.zone = Zone::GONE;
            o.last_known = Some(row);
        });
    }
}

/// `id` as it is now, for last-known information once it has ceased to be this object (CR 400.7, 608.2h, 113.7a): it
/// changed zones, whatever zone it left, or ceased to exist. Its characteristics and words (CR 612) are what an ability of
/// it, or a delayed ability it created, reads after (High Tide whose "Island" Magical Hack made "Swamp" watches Swamps;
/// CR 603.7d). Nothing hidden reaches a player through it: it is read only where the rules read the object itself.
pub fn last_known_of(cx: &mut Cx, id: ObjectId) -> LastKnown {
    let chars = layers::source_chars(cx.cards(), cx.s(), &cx.g.log, id);
    last_known_from(cx, id, &chars)
}

/// The history's record of `permanent` as it entered (CR 603.6a, 614.12), asked once every permanent entering with it is
/// there: its controller and characteristics then, kept in a last-known row (`Event::Entered::known`) so
/// `history::Entered::chars` says them however it changes after, as an attack's row does for the attacker.
fn entered_event(cx: &mut Cx, permanent: ObjectId) -> Event {
    let chars = cx.chars(permanent);
    let row = last_known_from(cx, permanent, &chars);
    let (controller, types) = (row.controller, row.types);
    let known = cx.st().last_known_push(row);
    Event::Entered { permanent, controller, types, known }
}

/// `last_known_of` with `id`'s characteristics `c` already worked out.
pub fn last_known_from(cx: &mut Cx, id: ObjectId, c: &Chars) -> LastKnown {
    // Every leaving object's row is prepared before the simultaneous move erases any pairs (CR 603.10a).
    // The member arena is journalled and cloned with the position, unlike a pointer into the undo journal.
    let known = cx.s().last_known(id).or_else(|| cx.s().phased_last_known(id))
        .map(|last| (last.blocked_creatures, last.blocking_creatures));
    let (blocked_creatures, blocking_creatures) = if let Some(known) = known {
        // A departed source can deal damage later (CR 113.7a). Its event snapshot must retain the same source LKI.
        known
    } else if cx.s().blocks.is_empty() {
        // Normal casting and noncombat events pay no arena writes or allocations.
        (Default::default(), Default::default())
    } else {
        let blocked: Vec<_> = cx.s().blocks.iter().filter(|b| b.blocker == id).map(|b| b.attacker).collect();
        let blocking: Vec<_> = cx.s().blocks.iter().filter(|b| b.attacker == id).map(|b| b.blocker).collect();
        (cx.st().members_push(&blocked), cx.st().members_push(&blocking))
    };
    LastKnown {
        id,
        commander_ids: crate::merge::commander_ids(cx.s(), id),
        card: c.card,
        controller: cx.controller(id),
        name: c.name,
        name_set: c.name_set,
        supertypes: c.supertypes,
        no_cost: c.no_cost,
        types: c.types,
        subtypes: c.subtypes,
        colors: c.colors,
        power: c.power,
        toughness: c.toughness,
        base_power: c.base_power,
        keywords: c.keywords,
        toxic: c.toxic,
        // Every quality, whichever Auras an instance spares (`Chars::as_known`).
        protection: c.protection_colors(),
        protection_colorless: c.protection_colorless,
        protection_players: c.protection_players,
        protection_types: c.protection_card_types(),
        protection_subtypes: c.protection_subtypes,
        protection_mana_value: c.protection_mana_value,
        spell: c.spell,
        protection_colored_spells: c.protection_colored_spells,
        protection_everything: c.protection_everything,
        hexproof_from: c.hexproof_from,
        must_attack: c.must_attack,
        attacking: cx.s().attackers.iter().any(|a| a.id == id),
        blocked_creatures,
        blocking_creatures,
        mana_value: cantrip_api::mana_value_of(&|card| cx.cards().get(card), cx.s(), id, c),
        abilities: c.abilities,
        words: cantrip_api::words(cx.s(), id),
    }
}

/// Creates tokens (CR 111.2): each a new object on the battlefield, from no zone.
/// The effects of spells and abilities that change a permanent spell's characteristics or controller go on applying to the
/// permanent it becomes (CR 400.7a): a spell whose text was changed is a permanent with that text, and a creature spell cast
/// with suspend that gained haste is a creature with haste (CR 702.62a). Each is an effect of a layer (`layers::effect_layer`)
/// or a change of control; it keeps its timestamp and duration.
pub fn carry_effects(cx: &mut Cx, spell: ObjectId, permanent: ObjectId) {
    let carries = |e: &cantrip_state::ContinuousEffect| {
        e.affected == spell && (layers::effect_layer(e.kind).is_some() || matches!(e.kind, EffectKind::Control(_)))
    };
    let carried: Vec<cantrip_state::ContinuousEffect> = cx.s().effects.iter().filter(|e| carries(e)).copied().collect();
    if carried.is_empty() {
        return;
    }
    let keep: Vec<bool> = cx.s().effects.iter().map(|e| !carries(e)).collect();
    let effects_before = cx.s().effects.len();
    cx.st().effects_keep(&keep);
    // An effect that ends changes the game with no event (CR 603.8).
    if cx.s().effects.len() < effects_before {
        cx.state_changed();
    }
    for e in carried {
        cx.st().effect_push(cantrip_state::ContinuousEffect { affected: permanent, ..e });
    }
}

pub fn create_tokens(cx: &mut Cx, card: CardId, controller: PlayerId, count: u32, tapped: bool) -> R<Vec<ObjectId>> {
    create_tokens_as(cx, card, controller, count, tapped, None, &[])
}

fn create_tokens_as(
    cx: &mut Cx,
    card: CardId,
    controller: PlayerId,
    count: u32,
    tapped: bool,
    copy: Option<Copiable>,
    with: &[(CounterKind, i32)],
) -> R<Vec<ObjectId>> {
    let tokens: Vec<(CardId, PlayerId)> = (0..count).map(|_| (card, controller)).collect();
    create_tokens_batch(cx, &tokens, tapped, copy, with)
}

/// The card, copy values and face up of a token made with `card` and `copy` (CR 707.8a). A copy of a double-faced
/// permanent or card (`Copiable::faces`) is a double-faced token whose card is that front face, entering with the face up
/// that was: copying one of its own faces, its copy values are kept only for their exceptions, which apply to either face
/// (`Obj::own_faces`); copying what a copy effect over it made it (Glasspool Mimic as a copy of a creature), they are that
/// copy effect, over both faces, as on the card. A copy of one face of a card that has two - a copy of a Clone that copied
/// one - is single-faced (`Obj::one_face`). Any other copy is kept only for its exceptions. The token's card, its copy
/// values, whether it enters back face up, and whether it is single-faced.
fn token_faces(cards: &CardTable, card: CardId, copy: Option<Copiable>) -> (CardId, Option<Copiable>, bool, bool) {
    let Some(c) = copy else { return (card, None, false, false) };
    match c.faces() {
        Some(front) if c.card == front || cards.backs[front.0 as usize] == Some(c.card) => {
            let own = Copiable { card: front, ..c }.own_faces_of(front);
            (front, own.has_exceptions().then_some(own), c.card != front, false)
        }
        Some(front) => (front, Some(c.from_faces(None, false)), c.back_up(), false),
        None => {
            let c = c.from_faces(None, false);
            (c.card, c.has_exceptions().then_some(c), false, cards.backs[c.card.0 as usize].is_some())
        }
    }
}

/// Token copy values, including text changes in the creating instruction (CR 612.4, 707.2): the colors and subtypes the
/// changed text names, and the name of a token named for its subtypes (CR 111.4, 612.2a).
fn token_copy(cards: &CardTable, card: CardId, copy: Option<Copiable>, words: Words) -> Option<Copiable> {
    if words.is_printed() {
        return copy;
    }
    cantrip_api::token_values(cards.get(card), card, words)
}

/// Whether an instruction names a defender, one for each token, or leaves its choice to the entering creature's controller
/// (CR 508.4).
#[derive(Clone, Copy)]
enum AttackEntry<'a> {
    Defender(Target),
    /// The token asked for at each position attacks what its entry names ("for each opponent, create a token ... attacking
    /// that player").
    Each(&'a [EntersAttacking]),
    Choose,
}

/// Creates tokens of several definitions as one instruction (CR 111.1, 101.4): all enter at once, and what watches
/// them sees one event (CR 603.2c).
pub fn create_tokens_batch(cx: &mut Cx, tokens: &[(CardId, PlayerId)], tapped: bool, copy: Option<Copiable>, with: &[(CounterKind, i32)]) -> R<Vec<ObjectId>> {
    create_tokens_batch_attacking(cx, tokens, tapped, copy, with, None)
}

fn create_tokens_batch_attacking(cx: &mut Cx, tokens: &[(CardId, PlayerId)], tapped: bool, copy: Option<Copiable>, with: &[(CounterKind, i32)], attacking: Option<Target>) -> R<Vec<ObjectId>> {
    let words = creating_words(cx, copy);
    create_tokens_worded(cx, tokens, tapped, copy, with, words, None, attacking)
}

/// Creates tokens whose text has `words` (CR 612.4): `create_tokens_batch` with the words given, not the creating text's.
/// `spell` is the copy of a permanent spell, as it resolves, the token is the permanent of (CR 707.10f), if it is: such a token is not
/// created (CR 111.13) but enters from the stack as the spell resolves, so its `ZoneChange` is `old: spell, from: Stack,
/// reason: Resolve`, as any permanent spell's is, and "if you cast it" (`cast_of(old)`, CR 707.12, 603.4) finds the copy's cast.
/// An Aura spell's token enters attached to what the spell targeted, wherever that is (CR 303.4f, 608.3b, 707.10c): a
/// creature card in a graveyard for Animate Dead (CR 303.4a), as the Aura card itself would.
#[allow(clippy::too_many_arguments)]
pub fn create_tokens_worded(
    cx: &mut Cx,
    tokens: &[(CardId, PlayerId)],
    tapped: bool,
    copy: Option<Copiable>,
    with: &[(CounterKind, i32)],
    words: Words,
    spell: Option<&StackItem>,
    attacking: Option<Target>,
) -> R<Vec<ObjectId>> {
    create_tokens_onto(cx, tokens, tapped, Copies::All(copy), with, words, spell, None, attacking.map(AttackEntry::Defender))
}

/// Creates a token "attached to" a permanent (CR 111.1, 303.4): `Game::create_token_attached`, with the words of the text
/// making it (CR 612.4).
pub fn create_token_attached(cx: &mut Cx, card: CardId, controller: PlayerId, host: ObjectId) -> R<Vec<ObjectId>> {
    let words = match cx.cause() {
        Cause::Spell(source) | Cause::Ability { source, .. } => Query::words(&*cx, source),
        _ => Words::NONE,
    };
    create_tokens_onto(cx, &[(card, controller)], false, Copies::All(None), &[], words, None, Some(host), None)
}

/// `create_tokens_worded`, each token an effect says is created attached to `host` entering so (CR 303.4): an Aura only
/// if it could legally enchant `host` as it enters, and not created otherwise (CR 303.4i); an Equipment if it could
/// equip it, and unattached otherwise (CR 301.5e); a Fortification follows the same rule for a land (CR 301.6);
/// anything else enters unattached (CR 303.4h). The choice is the effect's, not a target (hexproof does not matter).
#[allow(clippy::too_many_arguments)]
fn create_tokens_onto(
    cx: &mut Cx,
    tokens: &[(CardId, PlayerId)],
    tapped: bool,
    copies: Copies,
    with: &[(CounterKind, i32)],
    words: Words,
    spell: Option<&StackItem>,
    host: Option<ObjectId>,
    defender: Option<AttackEntry<'_>>,
) -> R<Vec<ObjectId>> {
    // Replacements inspect copy exceptions and changed text too (CR 614.12).
    let proposed: Vec<(CardId, Option<Copiable>, PlayerId)> = tokens
        .iter()
        .enumerate()
        .map(|(i, &(card, controller))| (card, token_copy(cx.cards(), card, copies.of(i), words), controller))
        .collect();
    // CR 614.1a: how many of each there are is decided by what would replace the creation, before any of them is made. The
    // token a copy of a permanent spell becomes as it resolves is not created (CR 111.13, 707.10f), so nothing replaces it.
    let replaced = if spell.is_some() { None } else { replace::token_batch(cx, &proposed) };
    let tokens = replaced.as_ref().map_or(&proposed[..], |(tokens, _)| &tokens[..]);
    // Which token asked for each one made is one of "those tokens" for, if any (`replace::token_batch`).
    let stands_for = |place: usize| replaced.as_ref().map_or(Some(place), |(_, stands)| stands[place]);
    let before: Vec<ObjectId> = cx.s().battlefield().to_vec();
    // Tokens entering trigger no look-back ability (CR 603.10a).
    let then = crate::settle::Then::none();
    let mut events = Vec::new();
    let mut created = Vec::new();
    // The tokens a replacement has arrive attacking (CR 614.1c, 508.4).
    let mut attacking: Vec<ObjectId> = Vec::new();
    // CR 614.12, 101.4: tokens created together enter together. Each one's entering choices and replacements are worked
    // out before any of them is on the battlefield, so none sees another (Giada counts the Angels already there).
    let mut arriving: Vec<(ObjectId, PlayerId, Entering, Option<PlayerId>, usize)> = Vec::with_capacity(tokens.len());
    // The token asked for that each token made is one of "those tokens" for, once a replacement has changed the creation.
    let mut places: Vec<Option<usize>> = Vec::with_capacity(tokens.len());
    for (place, &(card, copy, controller)) in tokens.iter().enumerate() {
        let (card, copy, back_up, one_face) = token_faces(cx.cards(), card, copy);
        let turn = cx.s().turn.number;
        let timestamp = cx.st().timestamp();
        let mut token = Obj::new(card, controller, Zone::GONE);
        token.copy = copy;
        token.transformed = back_up;
        token.one_face = one_face;
        token.token = true;
        token.controlled_since = turn;
        token.timestamp = timestamp;
        // The token a copy of a permanent spell becomes was promised the gift the spell was (CR 707.10, ruling 2024-07-26).
        token.gift = spell.and_then(|s| s.gift);
        let id = cx.st().push_obj(token);
        let aura = spell.filter(|_| cx.card(card).enchant.is_some());
        let mut entering = Entering { object: id, controller, tapped, counters: entry_counters(with), copy, prepared: false, chosen_color: None, chosen_subtype: None, chosen_player: None, chosen_card: None, chosen_option: None, chosen_number: None, chosen_object: None, emblem: None, attach: aura.and_then(|i| i.target(0).object()), attach_player: aura.and_then(|i| i.target(0).player()), x: 0 , transformed: back_up, protector: None, read_ahead: None, riot_haste: false, instead: None, becomes: None, own: None, melded: None, counters_if: None };
        prepare_entry(cx, &mut entering)?;
        // A token that would be a copy of an instant or sorcery card is not created (CR 111.5), nor one with such a face up
        // (CR 712.13a: a copy of a double-faced permanent spell turned over as it would enter).
        let face = entering_card(cx, &entering);
        if instant_or_sorcery(cx.card(face)) {
            continue;
        }
        // The token is judged as it would exist on the battlefield (CR 614.12): a token copy as what it copies.
        let (copy, transformed) = (entering.copy, entering.transformed);
        cx.st().update_obj(id, |o| {
            o.copy = copy;
            o.transformed = transformed;
        });
        // A copy of an Aura spell whose target is still legal becomes a token only if, as it would exist, it could legally
        // enchant what the spell targeted; otherwise it isn't created (CR 303.4i, 608.3c, 707.10f) - "can't be enchanted by
        // other Auras" or an Aura that would be a creature (CR 303.4d), which no target check asks.
        if aura.is_some() && attached_entry(cx, &entering, &[id]) == AttachedEntry::Stays {
            continue;
        }
        // "Create a token attached to [a permanent]" (CR 303.4h-i, 301.5e): an Aura that could not enchant it is not
        // created; an Equipment that could not equip it is created unattached; anything else enters unattached.
        if let Some(host) = host {
            // CR 614.12: attachment legality is determined as the token would
            // exist on the battlefield, including entry changes and continuous effects.
            let prospective = as_it_would_exist(cx, &entering);
            let chars = prospective.chars;
            let allowed = cx.s().obj(host).zone.is(ZoneKind::Battlefield)
                && crate::attach::host_allows_as(&*cx, id, &chars, entering.controller, || prospective.mana_value, host, true);
            if chars.has_subtype(Subtype::Aura) {
                if !allowed {
                    continue;
                }
                entering.attach = Some(host);
            } else if (chars.has_subtype(Subtype::Equipment) || chars.has_subtype(Subtype::Fortification)) && allowed {
                entering.attach = Some(host);
            }
        }
        // A token enters the battlefield as it is created (CR 111.1): the replacements of a permanent's entry apply to it
        // as to any other's (CR 614.1c-d), "enters tapped" and the counters one gives as it arrives.
        let proposed = ZoneChange {
            object: id,
            from: if spell.is_some() { Zone::STACK } else { Zone::GONE },
            to: Zone::BATTLEFIELD,
            reason: if spell.is_some() { ZoneReason::Resolve } else { ZoneReason::Created },
            controller,
            put_by: Some(spell.map_or(controller, |spell| spell.controller)),
            tapped: entering.tapped,
            attacking: false,
            counters: [0; CounterKind::COUNT],
            counters_by: controller,
            reveal: false,
            shuffle: false,
            is_card: false,
            madness: false,
            pays_characteristic: false,
            exiled_with: None,
            flashback: false,
            exile_instead: ExileInstead::Never,
            adventure: false,
            commander_components: [ObjectId::NONE; 8],
            omen: false,
            rebound: false,
            on_adventure: false,
            rebounding: false,
            destroy: false,
            regenerated: false,
            regeneration_shield: false,
            shielded: false,
            replaced: false,
        };
        let change = entry_replacements(cx, &mut entering, proposed);
        // An effect that sends it elsewhere instead (CR 614.1a): a token anywhere but the battlefield ceases to exist
        // (CR 111.7), so it is never created: an entry its own replacement sends elsewhere instead (`Entering::instead`)
        // as much as one another effect does.
        if !change.to.is(ZoneKind::Battlefield) {
            continue;
        }
        entering.tapped = change.tapped;
        entering.controller = change.controller;
        let controller = change.controller;
        for (n, with) in entering.counters.iter_mut().zip(change.counters) {
            *n += with;
        }
        // A token copy of a planeswalker, a battle or a Saga enters with its loyalty, defense or lore counters too
        // (CR 306.5b, 310.4b, 714.3a), put on by its controller like the rest (CR 122.6a).
        let mut counts = entering.counters;
        counters::intrinsic(cx.cards(), cx.s().obj(id), entering.read_ahead, &mut counts);
        entering.counters = counts;
        entering.counters =
            judged_entering(cx, &entering, counts.iter().any(|&n| n > 0), |cx| counters::entering(cx, id, controller, controller, counts));
        if change.attacking {
            attacking.push(id);
        }
        arriving.push((id, controller, entering, change.put_by, place));
    }
    for (id, controller, entering, put_by, place) in arriving {
        let chosen = keep_chosen(cx, &entering);
        cx.st().update_obj(id, |o| {
            o.zone = Zone::BATTLEFIELD;
            o.controller = entering.controller;
            o.tapped = entering.tapped;
            o.copy = entering.copy;
            // With the face up it entered with: a double-faced token copy of a transformed permanent (CR 707.8a), or one
            // turned over by daybound at night (CR 702.145b).
            o.transformed = entering.transformed;
            // What the token chose as it entered (CR 614.12): a copy of Painter's Servant picks its own color.
            chosen(o);
            // An Aura enters attached to what it targeted (CR 303.4f, 608.3b).
            o.attached_to = entering.attach;
            o.attached_player = entering.attach_player;
        });
        if entering.riot_haste {
            riot_haste(cx, id);
        }
        let mut put = Vec::new();
        counters::place_arrived(cx, id, entering.counters, controller, &mut put);
        // Once every counter it enters with is on it, a hone counter among them (CR 122.1j).
        note_statics(cx, id);
        cx.st().zone_push(Zone::BATTLEFIELD, id);
        emblem_as_it_arrives(cx, &entering, id);
        events.push(match spell {
            Some(spell) => Event::ZoneChange { old: spell.object, new: id, from: Zone::STACK, to: Zone::BATTLEFIELD, part: false, reason: ZoneReason::Resolve, put_by, enchanted_left: None, from_revealed_hand: false, to_revealed_hand: false },
            None => Event::ZoneChange { old: id, new: id, from: Zone::GONE, to: Zone::BATTLEFIELD, part: false, reason: ZoneReason::Created, put_by, enchanted_left: None, from_revealed_hand: false, to_revealed_hand: false },
        });
        // CR 303.4f, 301.5e: created attached to something, it became attached to it.
        events.extend(entered_attached(cx, id));
        events.extend(put);
        created.push(id);
        places.push(stands_for(place));
    }
    // A replacement that has it arrive attacking (CR 614.1c, 508.4): in the combat as it arrives, as a permanent put there
    // by a move is (`move_together`).
    // An instruction and a replacement can both say attacking. Each arrival
    // chooses only once; an instruction that names a defender needs no choice.
    match defender {
        Some(AttackEntry::Defender(defender)) => enter_attacking(cx, &created, defender),
        Some(AttackEntry::Each(entries)) => enter_attacking_each(cx, &created, &places, entries),
        Some(AttackEntry::Choose) => enter_attacking_by_choice(cx, &created),
        None if !attacking.is_empty() => enter_attacking_by_choice(cx, &attacking),
        None => {}
    }
    // What each token was as it entered (CR 603.6a, 614.12): judged once they are all there, as the permanents that enter by
    // a move are, so the history says it of tokens too.
    for &permanent in &created {
        events.push(entered_event(cx, permanent));
    }
    cx.emit_batch(&events, Some(crate::settle::Before { battlefield: &before, then: &then, seen_by_all: &[] }));
    Ok(created)
}

/// A copy of a card (CR 707.12) in the zone the card is in, owned and controlled by `controller`, who is to cast it: a
/// card_copy object, not a card, which ceases to exist anywhere but the stack it was cast onto and where it began.
pub fn copy_card(cx: &mut Cx, card: ObjectId, controller: PlayerId) -> ObjectId {
    let zone = cx.s().obj(card).zone;
    copy_card_into(cx, card, controller, zone)
}

/// A copy of an object (CR 707.12) created in `zone`, owned and controlled by `controller`: `copy_card` for a copy made
/// somewhere other than where the object is, or of one that has left its zone (paradigm's "create a copy of this object in
/// exile", CR 702.192a). It copies the object's copiable values as it last existed (CR 707.2, 608.2h): the exceptions of
/// the copy effects on it, its face-down values while it is a face-down permanent, and the face that is up (CR 707.8),
/// as a token copy does (`create_token_copies`).
pub fn copy_card_into(cx: &mut Cx, card: ObjectId, controller: PlayerId, zone: Zone) -> ObjectId {
    let values = cantrip_api::Query::copiable_values(&*cx, card);
    let turn = cx.s().turn.number;
    let timestamp = cx.st().timestamp();
    let mut obj = Obj::new(values.card, controller, zone);
    obj.copy = values.has_exceptions().then_some(values);
    (obj.card_copy, obj.controlled_since, obj.timestamp) = (true, turn, timestamp);
    let copy = cx.st().push_obj(obj);
    cx.st().zone_push(zone, copy);
    copy
}

/// Becomes prepared (CR 722.3a/c): the permanent gains the designation and its controller creates its prepare copy.
pub fn become_prepared(cx: &mut Cx, permanent: ObjectId) {
    if !live(cx.s(), permanent) || !cx.s().obj(permanent).zone.is(ZoneKind::Battlefield) || cx.s().obj(permanent).prepared {
        return;
    }
    let card = layers::chars(cx.cards(), cx.s(), &cx.g.log, permanent).card;
    if cx.cards().get(card).prepare.is_none() {
        return;
    }
    cx.st().update_obj(permanent, |o| o.prepared = true);
    prepared_copy(cx, permanent);
}

/// Becomes unprepared (CR 722.3b): the designation is removed even if the permanent no longer has a prepare spell or
/// its abilities. Infinite Coursework instructs this; every preparation card also loses the designation when its
/// prepare copy becomes cast (CR 722.3c). Expire the old copy's exception permanently without ending it before the next
/// state-based check (CR 704.5e): becoming prepared again creates a new copy, never revives the old one.
pub fn become_unprepared(cx: &mut Cx, permanent: ObjectId) {
    if !live(cx.s(), permanent) || !cx.s().obj(permanent).zone.is(ZoneKind::Battlefield) || !cx.s().obj(permanent).prepared {
        return;
    }
    cx.st().update_obj(permanent, |o| o.prepared = false);
    expire_prepared_copies(cx, permanent);
    cx.state_changed();
}

/// The "for as long as" exemption of a permanent's prepare copies ends permanently when it becomes unprepared or
/// phases out (CR 722.3c, 702.26f): their link to it is severed, so a later preparation or phasing in revives none of
/// them, and the next state-based check ends them (CR 704.5e).
pub(crate) fn expire_prepared_copies(cx: &mut Cx, permanent: ObjectId) {
    let copies: Vec<ObjectId> = cx
        .s()
        .seats()
        .flat_map(|p| cx.s().zone(Zone::exile(p)))
        .copied()
        .filter(|&id| cx.s().obj(id).card_copy && cx.s().obj(id).prepared_from == Some(permanent))
        .collect();
    for copy in copies {
        cx.st().update_obj(copy, |o| o.prepared_from = None);
    }
}

/// The copy of a prepared permanent's prepare spell its controller creates in exile (CR 722.3c), as it becomes prepared
/// or phases in prepared: its prepare spell's characteristics alone, whatever else the permanent's copy effects say.
pub fn prepared_copy(cx: &mut Cx, permanent: ObjectId) {
    let card = layers::chars(cx.cards(), cx.s(), &cx.g.log, permanent).card;
    let Some(spell) = cx.cards().get(card).prepare else { return };
    let spell = cx.cards().id_of(spell);
    let controller = cx.s().obj(permanent).controller;
    let turn = cx.s().turn.number;
    let timestamp = cx.st().timestamp();
    let mut obj = Obj::new(spell, controller, Zone::exile(controller));
    (obj.controlled_since, obj.timestamp, obj.card_copy, obj.prepared_from) = (turn, timestamp, true, Some(permanent));
    let copy = cx.st().push_obj(obj);
    cx.st().zone_push(Zone::exile(controller), copy);
}

/// Copies a spell (CR 707.10): its modes, targets, X and costs are copied, and with `new_targets` the copier may keep
/// or change each target before the copy is put on the stack (CR 707.10c, 115.7d-e). It is not cast, so nothing that
/// watches casts sees it; it ceases to exist when it leaves the stack. Every target of the copy, kept or new, becomes
/// a target of it. Nothing if the spell has no stack or last-known stack record, or `copier` has left the game.
/// A copy of a permanent spell becomes a token as it resolves (CR 707.10f): `resolve_copy_as_token`.
pub fn copy_spell(cx: &mut Cx, spell: ObjectId, copier: PlayerId, new_targets: bool) -> R<Option<ObjectId>> {
    copy_spell_as(cx, spell, copier, new_targets, None)
}

/// `copy_spell`, the copy having `values` as its copiable values where they are given: the spell's own
/// (`Query::copiable_values`) with the exceptions of the copy effect (CR 707.9, 707.9b: "the copy isn't legendary and has
/// starting loyalty X"). They are the copy's from the moment it is created, on the stack and as the token a copy of a
/// permanent spell becomes (CR 707.10f), and a copy of it copies them (CR 707.9b).
pub fn copy_spell_as(cx: &mut Cx, spell: ObjectId, copier: PlayerId, new_targets: bool, values: Option<Copiable>) -> R<Option<ObjectId>> {
    // A spell that has left the stack is copied as it last was on it (CR 707.10, 608.2h): the copy keeps its targets,
    // X and modes.
    let mut item = match cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == spell) {
        Some(at) => cx.s().stack[at],
        None => match cx.s().left_stack(spell) {
            Some(&left) => left,
            None => return Ok(None),
        },
    };
    // "This spell can't be copied" (Choreographed Sparks): the 'can't' beats the copy effect (CR 101.2, 707.10), which does nothing.
    if cx.chars(spell).keywords.has(Keyword::CantBeCopied) {
        return Ok(None);
    }
    if cx.s().player(copier).lost {
        return Ok(None);
    }
    let original = *cx.s().obj(spell);
    // An instant or sorcery, or a permanent spell (a copy of one becomes a token as it resolves, CR 707.10f).

    let turn = cx.s().turn.number;
    let timestamp = cx.st().timestamp();
    let mut obj = original;
    (obj.owner, obj.controller, obj.zone, obj.timestamp, obj.controlled_since) = (copier, copier, Zone::STACK, timestamp, turn);
    // A spell copy is not the designated physical commander card.
    obj.commander = false;
    obj.commander_id = None;
    obj.commander_return_pending = false;
    // A copy was paid for by no one: no mana spent (CR 707.10), so none of what mana does to the spell it pays for
    // (CR 106.6: Cavern of Souls' "can't be countered", Arena of Glory's haste) is the copy's.
    obj.spent = ManaPool::default();
    // Nor was it cast (CR 707.10): it was cast from no zone (Sevinne's Reclamation's copy, not cast from a graveyard,
    // does not copy itself), at no time (Necromancy's "if you cast it any time a sorcery couldn't have been cast"),
    // and by no way a permanent gave, so it gains nothing such a way gives (Serra Paragon's). What was chosen and
    // paid as the original was cast is the copy's too: X, kicks, other additional costs, gift and the alternative
    // cost (`x`, `kicked`, `paid`, `gift`, `cast_way`), with its copiable values (`card`, `copy`, CR 707.2) and the
    // face it is (`transformed`, `face_down`). Its object was never cast, so the history has no cast of it either.
    obj.cast_from = None;
    obj.marks = Marks { cant_be_countered: false, gains_haste: false, untimely_by_own_flash: false, gains: None, ..original.marks };
    (obj.card_copy, obj.prepared_from, obj.prepared, obj.became) = (true, None, false, None);
    // A new object, with no counters (CR 707.2: counters are not copied).
    obj.clear_counters();
    // A copy of a face-down spell (cast by morph or disguise, CR 708.4) copies its copiable values, the face-down values
    // (CR 707.2, 708.2), and not its face-down status: a face-up spell that is that nameless 2/2, with no card behind it
    // to look at or turn face up (CR 707.10; the Clone example of CR 707.2), and the token it becomes is the same.
    if let Some(face_down) = original.face_down_values() {
        (obj.card, obj.copy, obj.face_down) = (face_down, None, false);
        obj.marks.face_down_as = None;
    }
    if let Some(values) = values {
        obj.copy = Some(values);
    }
    let copy = cx.st().push_obj(obj);
    item.object = copy;
    item.controller = copier;
    item.serial = 0;
    // A copy is put onto the stack as it is created (CR 707.10, 701.27f).
    item.since = cx.g.log.len() as u32;
    // A copy is not cast (CR 707.10).
    item.cast_known = u32::MAX;
    if new_targets {
        item = retarget(cx, item, spell, copier)?;
    }
    // The copy is put onto the stack only after its targets are chosen (CR 707.10c).
    item.since = cx.g.log.len() as u32;
    cx.st().zone_push(Zone::STACK, copy);
    cx.st().stack_push(item);
    let item = *cx.s().stack.last().expect("the copy was just put on the stack");
    let targets = cx.s().targets(&item).to_vec();
    became_targets(cx, targets, copy, copier, false);
    cx.emit(Event::Copied { copy, original: spell, player: copier });
    Ok(Some(copy))
}

/// Copies the activated or triggered ability on the stack that `ability` names (`Target::Ability`, its serial) for
/// `copier` (CR 707.10): another instance of it, put onto the stack above everything, with the original's source (the
/// Lithoform Engine ruling), targets, modes, X, what it triggered on and the objects its costs were paid with (CR 707.10:
/// "the objects used to pay the costs of the original"). It was not activated or triggered (CR 707.10), so nothing that
/// triggers on activations sees it, and no cost is paid for it. An activated ability that has left the stack, countered or
/// resolved, is copied as it last was there (the Rings of Brighthearth ruling: the copy is made "even if that ability is
/// countered"); what copies a triggered ability targets it, and copies nothing once it is gone. With `new_targets` the
/// copier may choose new targets (CR 707.10c) as for a spell (`retarget`). Rings of Brighthearth, Kurkesh, Illusionist's
/// Bracers, Battlemage's Bracers and Locus of Enlightenment copy activated abilities; Lithoform Engine, Return the Favor and
/// Strionic Resonator target one. The copy's serial, or nothing if no such ability is to be found or `copier` has left the
/// game.
pub fn copy_ability(cx: &mut Cx, ability: Target, copier: PlayerId, new_targets: bool) -> R<Option<u32>> {
    let Target::Ability(serial) = ability else { return Ok(None) };
    let mut item = match cx.s().stack.iter().position(|i| i.kind != StackKind::Spell && i.serial == serial) {
        Some(at) => cx.s().stack[at],
        None => match cx.s().left_ability(serial) {
            Some(&left) => left,
            None => return Ok(None),
        },
    };
    // "This ability can't be copied" (Gogo, Master of Mimicry): the 'can't' beats the copy effect (CR 101.2, 707.10), which does
    // nothing.
    if matches!(cx.cards().ability(item.ability), AbilityDef::Activated(a) if a.cant_be_copied) {
        return Ok(None);
    }
    if cx.s().player(copier).lost {
        return Ok(None);
    }
    item.controller = copier;
    item.serial = 0;
    item.copied = true;
    // A copy is put onto the stack as it is created (CR 707.10).
    item.since = cx.g.log.len() as u32;
    item.tapped_since = item.since;
    // Reserve the copy's identity without putting an incompletely retargeted item on the stack. Cost queries made
    // while choosing targets follow the original activation, and the original ability remains a distinct candidate.
    item.serial = cx.st().timestamp();
    cx.record(Event::AbilityCopied { copy: item.serial, original: serial });

    if new_targets {
        item = retarget(cx, item, item.object, copier)?;
    }
    item.since = cx.g.log.len() as u32;
    item.tapped_since = item.since;
    cx.st().stack_push(item);
    let item = *cx.s().stack.last().expect("the ability copy was just put on the stack");
    // Each object or player it targets becomes a target once, however many of its slots name it (CR 115.2).
    let targets = cx.s().targets(&item).to_vec();
    became_targets(cx, targets, item.object, copier, true);
    Ok(Some(item.serial))
}

/// "You may choose new targets" (CR 115.7d-e, 707.10c): for each target occurrence, either keep its original
/// identity or choose a different target. The completed set, not an intermediate set, must be legal; an original
/// target that was already illegal may remain unchanged. `asked_about` names the original object in the prompt.
/// Copies are not put on the stack until this returns; an existing spell is updated only after it returns.
fn retarget(cx: &mut Cx, mut item: StackItem, asked_about: ObjectId, chooser: PlayerId) -> R<StackItem> {
    let specs = crate::resolve::spell_specs(cx.cards(), cx.s(), &item);
    // The chooser decides; the spell or ability's controller supplies "you"
    // and determines whose hexproof is relevant (CR 115.7d).
    let asker = cx.item_asker(&item, item.controller);
    let original = cx.s().targets(&item).to_vec();
    let originally_legal: Vec<bool> = (0..original.len())
        .map(|at| crate::resolve::target_legal(cx.cards(), cx.s(), &cx.g.log, &item, at))
        .collect();
    let (of, shares) = (cx.s().spec_of(&item).to_vec(), cx.s().shares(&item).to_vec());
    // Each empty answer retains that occurrence, including an illegal or gone
    // object. Never put such an object into a target-candidate UI (CR 115.7d).
    let mut retry = false;
    loop {
        let mut targets = original.clone();
        for (at, spec) in specs.iter().enumerate() {
            let run = cx.s().spec_targets(&item, at);
            let parts: Vec<(std::ops::Range<usize>, TargetSpec, Option<PlayerId>)> = if spec.max == TargetSpec::EACH_OPPONENT {
                let each = TargetSpec { min: 0, max: 1, ..*spec };
                let opponents: Vec<PlayerId> = cx.s().seats().filter(|&p| p != item.controller).collect();
                run.clone().zip(opponents).map(|(t, p)| (t..t + 1, each, Some(p))).collect()
            } else { vec![(run, *spec, None)] };
            for (run, spec, slot_player) in parts {
                for index in run.clone() {
                    let current = original[index];
                    if current == Target::None { continue; }
                    let earlier = if item.target_count > 1 { &targets[..run.start] } else { &[][..] };
                    let mut candidates = crate::actions::item_target_candidates(cx, &asker, &spec, earlier, &item, slot_player);
                    candidates.retain(|candidate| *candidate != current && !targets[run.start..index].contains(candidate));
                    if candidates.is_empty() { continue; }
                    let label = if retry {
                        "Those targets can't be chosen together. Choose a new target or none to keep the current one."
                    } else {
                        "Choose a new target or none to keep the current one."
                    };
                    let ask = RequestKind::ChooseTarget { replacing: Some((index as u32, current)), label, slot_player, source: asked_about,
                        purpose: Purpose::Targets, candidates, optional: true, least: 0, count: 1,
                        any_number: false, together: None };
                    if let Answer::Targets(chosen) = cx.ask_core(chooser, ask, false)?
                        && let Some(&target) = chosen.first()
                    {
                        targets[index] = target;
                    }
                }
            }
        }
        if targets == original { break; }
        // Only the completed assignment is judged (CR 115.7e). The changed
        // targets must be legal; an unchanged target already illegal may stay,
        // but one that was legal may not become illegal because of a change.
        let mark = cx.s().mark();
        let mut trial = item;
        cx.st().set_item_targets(&mut trial, &targets, &of, &shares);
        let distinct = (0..targets.len()).all(|at| targets[at] == Target::None ||
            (0..at).all(|other| of[other] != of[at] || targets[other] != targets[at]));
        let legal = distinct && (0..targets.len()).all(|at| {
            targets[at] == Target::None ||
                (targets[at] == original[at] && !originally_legal[at]) ||
                crate::resolve::target_legal(cx.cards(), cx.s(), &cx.g.log, &trial, at)
        });
        cx.st().revert(mark);
        if legal {
            cx.st().set_item_targets(&mut item, &targets, &of, &shares);
            break;
        }
        // The player can always leave every original target alone. Ask again
        // when the chosen final combination cannot legally coexist.
        retry = true;
    }
    Ok(item)
}

/// Whether target `slot` of `item`, its targets being `targets`, changed to `to` keeps the targets of its slot as the rule of
/// its targets together allows (`TargetSpec::together`, CR 115.7a, 601.2c), with the count they were chosen with; true
/// for a slot with none.
fn fits_together(cx: &Cx, item: &StackItem, targets: &[Target], slot: usize, spec: &TargetSpec, to: Target) -> bool {
    if spec.together.is_none() {
        return true;
    }
    let run = cx.s().spec_targets(item, cx.s().spec(item, slot) as usize);
    let set: Vec<Target> =
        run.map(|i| if i == slot { to } else { targets.get(i).copied().unwrap_or(Target::None) }).filter(|&t| t != Target::None).collect();
    select::targets_together(&crate::resolve::as_item(cx, item), spec.together_at(u32::from(item.counted)).as_slice(), item.controller, item.object, &set)
}

/// `chooser` may choose new targets for a spell on the stack (CR 115.7d): Sudden Substitution's "then the spell's controller
/// may choose new targets for it". Nothing for anything but a spell on the stack. Each new target becomes a target.
pub fn choose_new_targets(cx: &mut Cx, spell: ObjectId, chooser: PlayerId) -> R<()> {
    let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == spell) else {
        return Ok(());
    };
    let before = cx.s().targets(&cx.s().stack[at]).to_vec();
    let after = retarget(cx, cx.s().stack[at], spell, chooser)?;
    cx.st().stack_set(at, after);
    let new: Vec<Target> = cx.s().targets(&after).iter().copied().filter(|t| !before.contains(t)).collect();
    became_targets(cx, new, spell, after.controller, false);
    Ok(())
}

/// "Change the target of target spell or ability" (CR 115.7a), a spell or an ability on the stack: each target must be changed
/// to another legal target (its current one is never offered), and must be if it can be. If any target has no other legal
/// target, none changes (CR 115.7a, 115.7e). A target with exactly one other choice is changed without asking. Legality is the
/// spell's or ability's own (its source and controller); `chooser` is who answers. Each new target becomes a target.
pub fn change_target(cx: &mut Cx, of: Target, chooser: PlayerId) -> R<()> {
    let at = match of {
        Target::Object(o) => cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == o),
        Target::Ability(serial) => cx.s().stack.iter().position(|i| i.kind != StackKind::Spell && i.serial == serial),
        _ => None,
    };
    let Some(at) = at else { return Ok(()) };
    let mut item = cx.s().stack[at];
    let count = item.target_count as usize;
    // Its targets are judged as the spell or ability it is (CR 607.5).
    let asker = cx.item_asker(&item, item.controller);
    let all = cx.s().targets(&item).to_vec();
    // Nobody is asked anything when some target has nowhere else to go, since then nothing changes at all.
    let mut options: Vec<Vec<Target>> = Vec::with_capacity(count);
    for slot in 0..count {
        // A slot that chose no target has none to change.
        if all[slot] == Target::None {
            options.push(Vec::new());
            continue;
        }
        let Some((spec, slot_player)) = crate::resolve::slot_spec(cx.cards(), cx.s(), &item, slot) else { return Ok(()) };
        let others: Vec<Target> =
            crate::actions::item_target_candidates(cx, &asker, &spec, &all[..slot], &item, slot_player)
                .into_iter()
                .filter(|&t| t != all[slot] && fits_together(cx, &item, &all, slot, &spec, t))
                .collect();
        if others.is_empty() {
            return Ok(());
        }
        options.push(others);
    }
    let mut changed = all.clone();
    for (slot, others) in options.into_iter().enumerate() {
        // A slot whose targets together are judged judges each new target with the ones already changed (CR 115.7a, 601.2c).
        let others: Vec<Target> = match crate::resolve::slot_spec(cx.cards(), cx.s(), &item, slot) {
            Some((spec, _)) if spec.together.is_some() => others.into_iter().filter(|&t| fits_together(cx, &item, &changed, slot, &spec, t)).collect(),
            _ => others,
        };
        if others.is_empty() {
            continue;
        }
        let new = if others.len() == 1 {
            others[0]
        } else {
            let ask = RequestKind::ChooseTarget { replacing: None, label: "", slot_player: None, together: None,
                source: item.object,
                purpose: Purpose::Targets,
                candidates: others.clone(),
                optional: false,
                least: 1,
                count: 1,
                any_number: false,
            };
            match cx.ask_core(chooser, ask, false)? {
                Answer::Targets(chosen) if chosen.len() == 1 && others.contains(&chosen[0]) => chosen[0],
                _ => others[0],
            }
        };
        changed[slot] = new;
    }
    let (of, shares) = (cx.s().spec_of(&item).to_vec(), cx.s().shares(&item).to_vec());
    cx.st().set_item_targets(&mut item, &changed, &of, &shares);
    cx.st().stack_set(at, item);
    became_targets(cx, changed, item.object, item.controller, item.kind != StackKind::Spell);
    Ok(())
}

/// "Change a target of target spell or ability to [object]" (CR 115.7b, 115.7a): one of the targets of the spell or ability on
/// the stack (a `Target::Object` spell or a `Target::Ability`) becomes `to`, if `to` is a legal target for it: legal for that
/// target's own instance of "target" as the item's controller and source would judge it, not already chosen for that instance
/// (CR 115.3), within its slot's total (CR 601.2c), and leaving every other target that was legal still legal (the
/// Spellskite rulings; CR 115.7e judges the final set). A target that already is `to` is not changed, and no other is changed
/// in its place. With more than one target that may be changed, `chooser` chooses which as this resolves; with none, nothing
/// happens - nor does it if `to` has left the battlefield. The division of a divided total stays as it was (CR 115.7f), and
/// `to` becomes a target (CR 115.10).
pub fn change_a_target_to(cx: &mut Cx, of: Target, to: ObjectId, chooser: PlayerId) -> R<()> {
    let at = match of {
        Target::Object(o) => cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == o),
        Target::Ability(serial) => cx.s().stack.iter().position(|i| i.kind != StackKind::Spell && i.serial == serial),
        _ => None,
    };
    let Some(at) = at else { return Ok(()) };
    if !live(cx.s(), to) || !cx.s().obj(to).zone.is(ZoneKind::Battlefield) {
        return Ok(());
    }
    let mut item = cx.s().stack[at];
    let count = item.target_count as usize;
    // Its targets are judged as the spell or ability it is (CR 607.5).
    let asker = cx.item_asker(&item, item.controller);
    let new = Target::Object(to);
    let all = cx.s().targets(&item).to_vec();
    let spec_of = cx.s().spec_of(&item).to_vec();
    // Whether target `slot` of the item, its targets being `list`, is legal for its instance of "target", given the targets
    // before it.
    let legal = |cx: &Cx, list: &[Target], slot: usize| -> bool {
        let Some((spec, slot_player)) = crate::resolve::slot_spec(cx.cards(), cx.s(), &item, slot) else { return false };
        crate::actions::item_target_candidates(cx, &asker, &spec, &list[..slot], &item, slot_player).contains(&list[slot])
            && fits_together(cx, &item, list, slot, &spec, list[slot])
    };
    let changeable: Vec<usize> = (0..count)
        .filter(|&slot| {
            let current = all[slot];
            if current == Target::None || current == new {
                return false;
            }
            // CR 115.3: not for an instance of "target" that already has it.
            if (0..count).any(|j| j != slot && spec_of[j] == spec_of[slot] && all[j] == new) {
                return false;
            }
            let mut changed = all.clone();
            changed[slot] = new;
            legal(cx, &changed, slot) && (slot + 1..count).all(|k| all[k] == Target::None || !legal(cx, &all, k) || legal(cx, &changed, k))
        })
        .collect();
    let candidates: Vec<Target> = changeable.iter().map(|&s| all[s]).collect();
    // The same object held by two instances of "target" (CR 115.3) is two different targets to change, which may do different
    // things (Spellskite's ruling, 2020-08-07): the choice is then of the instance, each named by its slot's text.
    let shared = candidates.iter().enumerate().any(|(i, t)| candidates[..i].contains(t));
    let slot = match changeable.as_slice() {
        [] => return Ok(()),
        [only] => *only,
        _ if shared => {
            const NTH: [&str; 6] = ["Its first target", "Its second target", "Its third target", "Its fourth target", "Its fifth target", "Its sixth target"];
            let modes: Vec<Label> = changeable
                .iter()
                .map(|&slot| {
                    let text = crate::resolve::slot_spec(cx.cards(), cx.s(), &item, slot).map_or("", |(spec, _)| spec.label);
                    Label::TargetSlot(if text.is_empty() { NTH.get(slot).copied().unwrap_or("Another of its targets") } else { text }, all[slot])
                })
                .collect();
            let eligible = (0..modes.len() as u8).collect();
            let ask = RequestKind::ChooseModes { source: to, modes, eligible, count: 1, at_most: 0, affordable: Vec::new(), additional_cost: false, repeat: false };
            match cx.ask_core(chooser, ask, false)? {
                Answer::Modes(chosen) => changeable[chosen.first().map_or(0, |&m| m as usize).min(changeable.len() - 1)],
                _ => changeable[0],
            }
        }
        _ => {
            let ask = RequestKind::ChooseTarget { replacing: None,
                label: "The target to change to this creature",
                slot_player: None,
                together: None,
                source: to,
                purpose: Purpose::ChangeTargetTo,
                candidates: candidates.clone(),
                optional: false,
                least: 1,
                count: 1,
                any_number: false,
            };
            match cx.ask_core(chooser, ask, false)? {
                Answer::Targets(chosen) if chosen.len() == 1 => changeable[candidates.iter().position(|&t| t == chosen[0]).unwrap_or(0)],
                _ => changeable[0],
            }
        }
    };
    cx.st().set_item_target(&mut item, slot, new);
    cx.st().stack_set(at, item);
    became_targets(cx, [new], item.object, item.controller, item.kind != StackKind::Spell);
    Ok(())
}

/// A spell changes controllers on the stack (CR 112.2): only who controls it moves, so it resolves for its new controller
/// and "you" in it means them. Nothing it was cast with changes.
pub fn change_spell_controller(cx: &mut Cx, spell: ObjectId, player: PlayerId) {
    let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == spell) else {
        return;
    };
    let mut item = cx.s().stack[at];
    if item.controller == player || cx.s().player(player).lost {
        return;
    }
    item.controller = player;
    cx.st().stack_set(at, item);
    cx.st().update_obj(spell, |o| o.controller = player);
    // A "for as long as [its caster] controls it" ends the moment they don't (CR 611.2b: suspend's haste, CR 702.62a), not
    // only when the state-based actions are next checked.
    settle_at_once(cx);
    cx.state_changed();
}

/// A spell on the stack is exiled instead of put into its owner's graveyard as it leaves the stack (CR 614.1a): the
/// replacement of that move is set on its stack item (`StackItem::exile_instead`), as when the effect that had it cast set it.
pub fn set_spell_exile_instead(cx: &mut Cx, spell: ObjectId, instead: ExileInstead) {
    let Some(at) = cx.s().stack.iter().position(|i| i.kind == StackKind::Spell && i.object == spell) else {
        return;
    };
    let mut item = cx.s().stack[at];
    item.exile_instead = instead;
    cx.st().stack_set(at, item);
}

/// A copy of a permanent spell resolves (CR 707.10f): the copy is put onto the battlefield as a token that is a copy of
/// the spell's permanent. An Aura token enters attached to what the spell targeted (CR 303.4f, 608.3b, 707.10c); the copy itself
/// ceases to exist on the stack (CR 707.10a).
pub fn resolve_copy_as_token(cx: &mut Cx, item: &StackItem, at: usize) -> R<()> {
    let spell = *cx.s().obj(item.object);
    cx.st().stack_remove(at);
    cease_to_exist(cx, Zone::STACK, item.object);
    // A copy of a Room spell keeps the choice of door and the characteristics of both (CR 709.5b); the token it becomes is a
    // Room that enters with that door unlocked (CR 709.5d, 707.10f). Its copiable values are the whole card's, with the
    // exceptions the spell's had (CR 707.9b, 707.10): the door it was cast as is only the face it was on the stack.
    let door = if cx.cards().get(spell.card).is_room() { cast_door(spell.cast_way) } else { 0 };
    let values = match spell.copy {
        Some(copy) if door != 0 => Copiable { card: spell.card, ..copy },
        Some(copy) => copy,
        None => Copiable::of(spell.card),
    };
    let words = if values.has_exceptions() { Words::NONE } else { cantrip_api::words(cx.s(), item.object) };
    let copy = if values.has_exceptions() { Some(values) } else { None };
    // A copy keeps the alternative cost paid for the original (CR 707.10): a copy of a spell cast for sneak becomes a token
    // that enters tapped and attacking what the original's returned creature attacked (CR 702.190b), unless the rules keep
    // it out of combat (`enter_attacking`, CR 506.3b: its controller is not the attacking player).
    let attacking = item.attacking;
    let tokens = create_tokens_worded(cx, &[(values.card, item.controller)], attacking, copy, &[], words, Some(item), attacking.then_some(item.subject))?;
    // The spell became that token (CR 707.10f), as a card's spell becomes its permanent: what the copy was cast for, which a
    // copy copies, is found through the copy's `Copied` event (`sneak::was_cast_for_sneak`).
    if let Some(&token) = tokens.first() {
        cx.st().update_obj(item.object, |o| o.became = Some(token));
        // What spells and abilities did to the permanent spell goes on applying to the token it becomes (CR 400.7a): Spider-Verse's
        // haste for the copy it makes.
        carry_effects(cx, item.object, token);
    }
    for token in tokens.into_iter().filter(|_| door != 0) {
        unlock_door(cx, token, door, item.controller);
    }
    Ok(())
}

/// Draws a card (CR 121): from an empty library, nothing is drawn and the player will lose (CR 704.5b).
pub fn draw(cx: &mut Cx, player: PlayerId) {
    // CR 121.2b, 614.17c: a card the player can't draw is not drawn, and no replacement effect replaces the draw that can't
    // happen (Spirit of the Labyrinth's ruling), so neither does a player who can't draw from an empty library lose for it.
    if crate::static_rules::cant_draw(&*cx, cx.cards(), player, 1) {
        return;
    }
    // CR 121.6: a replacement applies even with no cards in the library to draw.
    if replace::draw_replaced(cx, player) {
        return;
    }
    let Some(&top) = cx.s().zone(Zone::library(player)).first() else {
        cx.st().update_player(player, |p| p.drew_from_empty = true);
        return;
    };
    let card = move_to_for(cx, top, Zone::hand(player), ZoneReason::Draw);
    // Miracle (CR 702.94a): "you may reveal this card from your hand as you draw it" if it is the first card drawn this
    // turn, before anything else happens; only if it is revealed does the linked ability trigger. The window is the
    // drawing player's alone, so nobody else learns of a "no". The card has its own miracle and any a permanent gives
    // cards in its controller's hand (Lorehold, the Historian); with more than one, the player says which it is revealed
    // with, and only that one's linked ability triggers (CR 607.5).
    if cx.s().turn.number > 0 && cx.s().obj(card).zone == Zone::hand(player) {
        let miracles = crate::static_rules::miracles(&*cx, cx.cards(), card);
        if let Some(&first) = miracles.first()
            && crate::loops::settled_read(cx, |h| h.drawn(player) == 0, |&first| !first)
            && matches!(cx.ask_core(player, RequestKind::May { source: card, purpose: Purpose::RevealForMiracle(only_miracle(cx, card, &miracles)) }, false), Ok(Answer::Yes))
            && let Some(miracle) = which_miracle(cx, player, card, &miracles, first)
            && cx.s().obj(card).zone == Zone::hand(player)
        {
            cx.emit(Event::Revealed { player, card });
            let ability = AbilityRef::miracle(cx.s().obj(card).card);
            cx.st().trigger_push(PendingTrigger { event_amount: None,
                source: card,
                ability,
                controller: player,
                subject: Target::None,
                data: miracle.data(),
                members: cantrip_state::Span::default(),
                count: None,
                since: None,
                acquired: Acquired::OWN,
                words: None,
            });
        }
    }
    cx.emit(Event::Drew { player, card });
}

/// A card's miracle ability, named for the question whether to reveal it, when it has only one (CR 702.94a).
fn only_miracle(cx: &Cx, card: ObjectId, miracles: &[cantrip_api::Miracle]) -> Option<MiracleOf> {
    match miracles {
        [one] => Some(one.of(cx.s().obj(card).card)),
        _ => None,
    }
}

/// The miracle ability a card being revealed as it is drawn is revealed with (CR 702.94a, 607.5): its only one, or the one
/// its player chooses.
fn which_miracle(cx: &mut Cx, player: PlayerId, card: ObjectId, miracles: &[cantrip_api::Miracle], first: cantrip_api::Miracle) -> Option<cantrip_api::Miracle> {
    if miracles.len() == 1 {
        return Some(first);
    }
    let own = cx.s().obj(card).card;
    let ask = RequestKind::ChooseModes {
        source: card,
        modes: miracles.iter().map(|m| Label::Miracle(m.of(own))).collect(),
        eligible: (0..miracles.len() as u8).collect(),
        count: 1,
        at_most: 0,
        affordable: Vec::new(),
        additional_cost: false,
        repeat: false,
    };
    match cx.ask_core(player, ask, false) {
        Ok(Answer::Modes(chosen)) => chosen.first().and_then(|&i| miracles.get(i as usize)).copied(),
        _ => None,
    }
}

/// An instruction to draw `count` cards (CR 121.2): effects that change how many apply first (CR 121.2a), before any
/// of the individual draws, which are then made one at a time.
pub fn draw_cards(cx: &mut Cx, player: PlayerId, count: u32) {
    for n in 0..replace::draw_count(cx, player, count) {
        // Cards are drawn one at a time (CR 121.2): what a draw changed with no event (a replacement effect used up) is a
        // point of the game the state triggers see before the next.
        if n > 0 {
            // CR 121.4: with the library empty and the attempt already made, and no effect to replace or forbid a draw,
            // each draw left only repeats it - an instruction for 2^32 or more cards (Mathemagics) ends here.
            if replace::draws_only_repeat_an_attempt_from_an_empty_library(cx, player) {
                break;
            }
            cx.judge_if_due();
        }
        draw(cx, player);
    }
}

pub fn discard(cx: &mut Cx, card: ObjectId) {
    discard_together(cx, &[card]);
}

/// Discards cards together (CR 701.9).
pub fn discard_together(cx: &mut Cx, cards: &[ObjectId]) -> Vec<ObjectId> {
    discard_paying(cx, cards, &[])
}

/// `discard_together`, the cards among them in `paying` discarded to pay a cost that specifies a characteristic of them (CR
/// 118.12): no replacement that would put one into a hidden zone unrevealed is offered for it (CR 701.9c,
/// `ZoneChange::pays_characteristic`).
pub fn discard_paying(cx: &mut Cx, cards: &[ObjectId], paying: &[ObjectId]) -> Vec<ObjectId> {
    discard_moving(cx, cards, paying, false)
}

/// `discard_together` for cards the instruction discarding them revealed ("target opponent reveals their hand. You choose a
/// card from it. That player discards that card"): a card stays revealed through the parts of the effect it is relevant to
/// (CR 701.20a), its discard among them, so one a replacement puts into a hand or a library instead goes there seen by all
/// (CR 603.10a), its characteristics known (CR 701.9c applies only to a card put there without being revealed).
pub fn discard_revealed(cx: &mut Cx, cards: &[ObjectId]) -> Vec<ObjectId> {
    discard_moving(cx, cards, &[], true)
}

/// `player` cycles `card` (CR 702.29c): discards it to pay the cost of a cycling ability (`ActivatedDef::cycling`). The
/// discard's batch announces both, each naming the card where it went (CR 400.7) - in its graveyard, or wherever a
/// replacement put it - so "when you cycle this card" triggers from that zone (CR 702.29c), "whenever you cycle or discard
/// a card" sees one batch (CR 702.29d), and a card put into a hidden zone unrevealed has nothing of its own trigger (CR
/// 701.9c, `Event::Discarded::undefined`).
pub fn cycle(cx: &mut Cx, player: PlayerId, card: ObjectId) {
    let (new, mut events) = discard_events(cx, &[card], &[], false);
    events.extend(new.iter().map(|&card| Event::Cycled { card, player }));
    cx.emit_batch(&events, None);
}

/// The discard of `discard_paying` and `discard_revealed`: each move marked as paying a characteristic or as revealed.
fn discard_moving(cx: &mut Cx, cards: &[ObjectId], paying: &[ObjectId], revealed: bool) -> Vec<ObjectId> {
    let (new, events) = discard_events(cx, cards, paying, revealed);
    cx.emit_batch(&events, None);
    new
}

/// Moves the cards of `discard_moving`, and returns what they became with the discard's events, to be announced as one
/// batch.
fn discard_events(cx: &mut Cx, cards: &[ObjectId], paying: &[ObjectId], revealed: bool) -> (Vec<ObjectId>, Vec<Event>) {
    let moves = discard_moves(cx, cards, paying, revealed);
    let (new, seen) = move_together_seen(cx, &moves, ZoneReason::Discard);
    (new, discarded_events(cx, &moves, &seen))
}

/// The moves of a discard: each card still the card it was (CR 400.7) to its owner's graveyard - but one an opponent's spell
/// or ability can't make its owner discard, which stays where it is (CR 101.2).
fn discard_moves(cx: &Cx, cards: &[ObjectId], paying: &[ObjectId], revealed: bool) -> Vec<Moving> {
    let causer = cx.effect_controller();
    cards
        .iter()
        .filter(|&&c| live(cx.s(), c))
        .filter(|&&c| causer.is_none_or(|by| cantrip_api::may_cause(cx, by, cx.s().obj(c).owner, cantrip_api::CauseKind::Discard)))
        .map(|&c| Moving {
            reason: Some(ZoneReason::Discard),
            pays_characteristic: paying.contains(&c),
            revealed,
            ..Moving::to(c, Zone::graveyard(cx.s().obj(c).owner))
        })
        .collect()
}

/// The discard's own events once its `moves` are made: one for each card discarded, naming it where it went (CR 400.7),
/// announced as one batch, so that "whenever you discard one or more cards" sees the discard once (CR 603.2c). A card a
/// replacement put into a hidden zone instead of its owner's graveyard (Library of Leng), and not revealed as it went (not
/// `seen`: Wheel of Sun and Moon reveals it), has every characteristic undefined (CR 701.9c): the event says so, for the
/// history and every ability that reads it.
fn discarded_events(cx: &Cx, moves: &[Moving], seen: &[ObjectId]) -> Vec<Event> {
    let mut events: Vec<Event> = Vec::with_capacity(moves.len());
    for m in moves {
        // A card the moves left where it was (one a replacement had do nothing instead) became nothing; one named twice moved
        // once.
        let Some(card) = cx.s().obj(m.id).became else { continue };
        if events.iter().any(|e| matches!(e, Event::Discarded { card: c, .. } if *c == card)) {
            continue;
        }
        let undefined = !seen.contains(&m.id) && matches!(cx.s().obj(card).zone.kind(), ZoneKind::Hand | ZoneKind::Library);
        events.push(Event::Discarded { player: cx.s().obj(card).owner, card, undefined });
    }
    events
}

/// Mills (CR 701.17): the top cards together, top first, so the second card milled ends up above the first unless their
/// owner arranges them otherwise (CR 404.3, `arrange_graveyards`).
pub fn mill(cx: &mut Cx, player: PlayerId, count: u32) -> Vec<ObjectId> {
    mill_together(cx, &[(player, count)])
}

/// Mills as `mill` does, with the number the instruction became once the effects that change it applied (CR 614.1a).
pub fn mill_result(cx: &mut Cx, player: PlayerId, count: u32) -> MillResult {
    let count = replace::mill_count(cx, player, count);
    MillResult { cards: mill_counted(cx, &[(player, count)]), count }
}

/// Several players mill at once (CR 701.17, 101.4): "each player mills", "any number of target players each mill". Every
/// count is read before any card moves - the caller works them out first - and all the cards move in one simultaneous
/// zone change, announced as one event, so "whenever one or more cards are milled" sees one action (CR 603.2c). Each
/// player's cards are their library's top cards, top first. The effects that change how many a player mills apply to
/// each player's number first (CR 614.1a, `replace::mill_count`), every number before any card moves; a player mills as
/// many as their library holds (CR 701.17b).
pub fn mill_together(cx: &mut Cx, mills: &[(PlayerId, u32)]) -> Vec<ObjectId> {
    let counts: Vec<(PlayerId, u32)> = mills.iter().map(|&(player, count)| (player, replace::mill_count(cx, player, count))).collect();
    mill_counted(cx, &counts)
}

/// The mill itself, each number already replaced: the cards that went to a graveyard, or to exile instead (CR 701.17c).
fn mill_counted(cx: &mut Cx, counts: &[(PlayerId, u32)]) -> Vec<ObjectId> {
    let mut moves: Vec<Moving> = Vec::new();
    for &(player, count) in counts {
        let top = cx.s().zone(Zone::library(player)).iter().take(count as usize).copied();
        moves.extend(top.map(|c| Moving::to(c, Zone::graveyard(player))));
    }
    move_together(cx, &moves, ZoneReason::Mill)
        .into_iter()
        .filter(|&n| cx.s().obj(n).zone.is(ZoneKind::Graveyard) || cx.s().obj(n).zone.is(ZoneKind::Exile))
        .collect()
}

/// Deals damage (CR 120): marked on a creature, removed as loyalty from a planeswalker, lost as life by a player.
/// Protection prevents damage from sources of its colors (CR 702.16e).
pub fn damage(cx: &mut Cx, source: ObjectId, target: Target, amount: u32, combat: bool) -> u32 {
    cx.instruction(|cx| damage_result_event(cx, source, target, amount, combat), |_| true).amount
}

/// Damage's final recipient and actual excess (CR 120.10), not the amount proposed before prevention or redirection.
pub fn damage_result(cx: &mut Cx, source: ObjectId, target: Target, amount: u32, combat: bool) -> Option<DamageResult> {
    cx.instruction(|cx| damage_result_event(cx, source, target, amount, combat), |_| true).result
}

fn damage_result_event(cx: &mut Cx, source: ObjectId, target: Target, amount: u32, combat: bool) -> DamageOutcome {
    let (mut results, _, mut batch) = damage_batch(cx, &[(source, target, amount)], combat);
    batch.rest_of_preventions(cx);
    results.remove(0)
}

/// Deals damage to several recipients at once (CR 120.2, 603.2c): announced as one batch, as combat damage is. How much
/// each was dealt, in order.
pub fn damage_together(cx: &mut Cx, hits: &[(ObjectId, Target, u32)]) -> Vec<u32> {
    cx.instruction(|cx| damage_together_event(cx, hits), |_| true)
}

fn damage_together_event(cx: &mut Cx, hits: &[(ObjectId, Target, u32)]) -> Vec<u32> {
    let (results, _, mut batch) = damage_batch(cx, hits, false);
    batch.rest_of_preventions(cx);
    results.into_iter().map(|r| r.amount).collect()
}

/// Forms every original and replacement-introduced component before processing
/// any damage into results (CR 120.4b-d). Combat and spells share this procedure.
/// Each original assignment retains its own return value; added components are
/// announced in the same batch, with their own source and damage characteristics.
pub(crate) struct DamageOutcome { pub amount: u32, pub result: Option<DamageResult> }

pub(crate) fn damage_batch(cx: &mut Cx, hits: &[(ObjectId, Target, u32)], combat: bool) -> (Vec<DamageOutcome>, Vec<Event>, DamageBatch) {
    let mut batch = DamageBatch::new(cx, hits);
    let proposed: Vec<_> = hits.iter().map(|&(source, target, amount)| DamageEvent { source, target, amount, combat, heal_marked: false, shielded: None }).collect();
    let parts = replace::damage_event(cx, &proposed, &mut batch.prevented, &mut batch.shielded);
    // Damage characteristics are those of the event before any of its results,
    // including a life-dependent infect rule or a source's changed controller.
    for part in &parts {
        if part.event.amount > 0 && let Target::Player(p) = part.event.target { batch.as_though_infect(cx, p); }
    }
    let viewed: Vec<_> = parts.into_iter().map(|part| { let view = damage_view(cx, &part.event); (part, view) }).collect();
    let mut slots = vec![Vec::new(); hits.len()];
    let mut events = Vec::new();
    for (part, view) in viewed {
        if let Some(event) = apply_damage_with(cx, part.event, view, &mut batch) {
            let mut attributed: Vec<(usize, u32)> = Vec::new();
            for (origin, amount) in part.origins {
                let amount = if amount > 0 { amount as u32 } else { 0 };
                if let Some(i) = origin && amount > 0 {
                    if let Some((_, n)) = attributed.iter_mut().find(|(origin, _)| *origin == i) {
                        *n += amount;
                    } else { attributed.push((i, amount)); }
                }
            }
            for (i, amount) in attributed { slots[i].push((events.len(), amount)); }
            events.push(event);
        }
    }
    batch.finish(cx, &mut events);
    let results = slots.into_iter().map(|slots| {
        let mut outcome = DamageOutcome { amount: 0, result: None };
        for (at, attributed) in slots {
            let Event::Damage { target, recipient_excess, .. } = events[at] else { unreachable!("the original assignment's damage") };
            outcome.amount += attributed;
            outcome.result.get_or_insert(DamageResult { target, amount: attributed, excess: recipient_excess });
        }
        outcome
    }).collect();
    (results, events, batch)
}

/// Heals all damage marked on a permanent (CR 701.69a). A phased-out or new-zone object is not affected.
pub fn heal(cx: &mut Cx, permanent: ObjectId) {
    if !live(cx.s(), permanent)
        || !cx.s().obj(permanent).zone.is(ZoneKind::Battlefield)
        || cx.s().obj(permanent).zone.is(ZoneKind::PhasedOut)
    {
        return;
    }
    let object = cx.s().obj(permanent);
    if object.damage == 0 && !object.deathtouched {
        return;
    }
    cx.st().update_obj(permanent, |o| {
        o.damage = 0;
        o.deathtouched = false;
    });
    cx.state_changed();
}

/// The order in which the assignments of one simultaneous damage event (CR 120.2, 510.2) meet the prevention shields of
/// their recipients, as indices into `hits`: "If damage would be dealt to the shielded permanent or player by two or more
/// applicable sources at the same time, the player or the controller of the permanent chooses which damage the shield
/// prevents" (CR 615.7). For each recipient a shield would not cover in full, its player or controller chooses a source
/// whose damage the shield prevents first, then the next while some is left; the damage of the others is dealt normally
/// once it is used up. Everything else keeps its order. Players with such choices make them in APNAP order (CR 616.1).
#[cfg(test)]
pub fn order_for_shields(cx: &mut Cx, hits: &[(ObjectId, Target, u32)], combat: bool) -> Vec<usize> {
    let events: Vec<_> = hits.iter().map(|&(source, target, amount)| DamageEvent { source, target, amount, combat, heal_marked: false, shielded: None }).collect();
    order_for_shield_events(cx, &events)
}

#[cfg(test)]
pub(crate) fn order_for_shield_events(cx: &mut Cx, hits: &[DamageEvent]) -> Vec<usize> {
    damage_source_order(cx, hits).order
}

/// Indices whose relative order was chosen (or immaterial because all remaining damage was covered). An unasked suffix
/// may need a later choice if another prevention effect leaves more of the numbered shield available (CR 615.7).
pub(crate) struct DamageSourceOrder {
    pub order: Vec<usize>,
    pub established: Vec<usize>,
}
pub(crate) fn damage_source_order(cx: &mut Cx, hits: &[DamageEvent]) -> DamageSourceOrder {
    let mut order: Vec<usize> = (0..hits.len())
        .filter(|&i| !matches!(hits[i].target, Target::Player(p) if !cx.s().in_game(p)))
        .collect();
    let mut established = order.clone();
    if hits.len() < 2
        || !cx.s().effects.iter().any(|e| {
            matches!(
                e.kind,
                EffectKind::Prevention {
                    shield: Shield::Next(_),
                    ..
                }
            )
        })
    {
        return DamageSourceOrder { order, established };
    }
    let mut recipients: Vec<(PlayerId, Target)> = Vec::new();
    for d in hits {
        let to = d.target;
        let chooser = match to {
            Target::Player(p) if cx.s().in_game(p) => p,
            Target::Object(id)
                if live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Battlefield) =>
            {
                cx.s().obj(id).controller
            }
            _ => continue,
        };
        if !recipients.iter().any(|&(_, t)| t == to) {
            recipients.push((chooser, to));
        }
    }
    let apnap: Vec<PlayerId> = cx.s().apnap().collect();
    recipients.sort_by_key(|&(p, _)| apnap.iter().position(|&q| q == p).unwrap_or(usize::MAX));
    for (chooser, to) in recipients {
        // The assignments to it that a shield applies to, and what the shields have left between them.
        let mut shielded: Vec<usize> = Vec::new();
        let (mut by, mut left) = (ObjectId(0), 0u32);
        for (i, d) in hits.iter().enumerate() {
            let (target, amount) = (d.target, d.amount);
            if target != to || amount == 0 {
                continue;
            }
            if let Some((s, n)) = replace::shields(cx, d) {
                if shielded.is_empty() {
                    by = s;
                }
                shielded.push(i);
                left = n;
            }
        }
        // The amounts are as assigned; where a replacement effect may yet change them ("that much damage plus 1", or less),
        // the shield might not cover them after all, or have more left than they say, so the choice is asked whatever they
        // are, for as long as two or more sources are left.
        let may_change = replace::damage_may_change(cx);
        let exceeds = |rest: &[usize], mut remaining: u32| {
            for &i in rest {
                if hits[i].amount > remaining {
                    return true;
                }
                remaining -= hits[i].amount;
            }
            false
        };
        let mut rest = shielded.clone();
        let mut chosen = Vec::new();
        while may_change || (left > 0 && exceeds(&rest, left)) {
            let mut sources: Vec<ObjectId> = Vec::new();
            for &i in &rest {
                if !sources.contains(&hits[i].source) {
                    sources.push(hits[i].source);
                }
            }
            if sources.len() < 2 {
                break;
            }
            let ask = RequestKind::choose_cards(
                by,
                Purpose::PreventDamageFrom(left),
                sources.clone(),
                sources,
                1,
                false,
                "",
            );
            // Unanswered yet, the run goes on with the first and is replayed with the answer (`Cx::ask_core`).
            let pick = match cx.ask_core(chooser, ask, false) {
                Ok(Answer::Cards(c)) if c.len() == 1 => rest
                    .iter()
                    .position(|&i| hits[i].source == c[0])
                    .unwrap_or(0),
                _ => 0,
            };
            let i = rest.remove(pick);
            left -= hits[i].amount.min(left);
            chosen.push(i);
        }
        if !may_change && left == 0 {
            let mut sources = Vec::new();
            for &i in &rest {
                if !sources.contains(&hits[i].source) {
                    sources.push(hits[i].source);
                }
            }
            if sources.len() > 1 {
                established.retain(|i| !rest.contains(i));
            }
        }
        if chosen.is_empty() {
            continue;
        }
        chosen.extend(rest);
        // The shielded assignments take their places in the order chosen.
        let mut slots: Vec<usize> = shielded
            .iter()
            .map(|i| order.iter().position(|o| o == i).expect("in the order"))
            .collect();
        slots.sort_unstable();
        for (slot, i) in slots.into_iter().zip(chosen) {
            order[slot] = i;
        }
    }
    DamageSourceOrder { order, established }
}

/// What one simultaneous damage event (CR 120.2, 510.2) has done so far, across its assignments.
#[derive(Default)]
pub struct DamageBatch {
    /// What it has dealt to each permanent, for an effect that heals the damage marked before the event (it must not
    /// heal the event's own).
    dealt: Vec<DamageRecipient>,
    /// The life lost to noninfect player damage (CR 120.3a), prepared only once all damage replacements have applied.
    /// Losses to one player are combined; gains remain distinct from them (CR 120.4c-d).
    life: Vec<(PlayerId, i64)>,
    /// What each source with lifelink has dealt, and who controls it: one life gain event per source, however many
    /// recipients it dealt damage to (CR 120.3f; Angel of Vitality's and Leyline of Hope's rulings).
    lifelink: Vec<(ObjectId, PlayerId, i64)>,
    /// For each player dealt damage in it, whether the damage is dealt as though its source had infect (Phyrexian Unlife's "as
    /// long as you have 0 or less life"): judged once, as the event begins for that player, since all of it is dealt at once
    /// (CR 120.2, 510.2) - a life total the event itself takes to 0 makes none of it poison.
    as_infect: Vec<(PlayerId, bool)>,
    /// The permanent and player counters damage produces (CR 120.3b, 120.3d, 120.3g), one occurrence per source.
    /// The shared counter batch prepares them alongside this damage's life results and applies them only afterward.
    counters: counters::Batch,
    /// The permanents whose shield counter has applied to this event (CR 122.1c): one replacement for the simultaneous event
    /// (CR 120.4), so a second assignment of damage to the same permanent is prevented as well, and no second counter is removed.
    shielded: Vec<ObjectId>,
    /// The prevention effects spells and abilities made that prevented some of its damage and have more to do once it has all
    /// been dealt (CR 615.5, 615.8): `rest_of_preventions`.
    prevented: Vec<replace::PreventionUse>,
}

/// A recipient's state before this damage event, and the damage actually dealt in it. Shared with the marked-damage
/// accounting used by prevention, rather than an independent record of past damage (CR 120.10).
struct DamageRecipient {
    object: ObjectId,
    marked: u32,
    heal_marked: bool,
    touched: bool,
    loyalty: u32,
    defense: u32,
    amount: u32,
    creature_toughness: Option<u32>,
    prior_marked: u32,
    other_lethal: Option<u32>,
    deathtouch: bool,
}

impl DamageRecipient {
    fn excess(&self) -> u32 {
        let creature = self.creature_toughness.map(|toughness| {
            let lethal = toughness - self.prior_marked.min(toughness);
            if self.deathtouch { lethal.min(1) } else { lethal }
        });
        let lethal = creature.into_iter().chain(self.other_lethal).min();
        lethal.map_or(0, |lethal| self.amount - lethal.min(self.amount))
    }
}

impl DamageBatch {
    /// Read every possible recipient before any assignment changes the state (CR 120.10). For example, damage to a
    /// player can change Serra Ascendant's toughness before the next assignment reaches that creature. Redirection
    /// can choose a recipient absent from `hits`, so active replacement effects require the whole battlefield.
    pub fn new(cx: &Cx, hits: &[(ObjectId, Target, u32)]) -> Self {
        let mut batch = Self::default();
        if hits.len() < 2 {
            return batch;
        }
        let objects: Vec<ObjectId> = if replace::possible(cx, None) {
            cx.s().battlefield().to_vec()
        } else {
            let mut objects = Vec::new();
            for &(_, target, amount) in hits {
                if let Target::Object(id) = target
                    && amount > 0 && live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Battlefield)
                    && !objects.contains(&id)
                {
                    objects.push(id);
                }
            }
            objects
        };
        for id in objects {
            let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, id);
            batch.snapshot(cx.s(), id, &chars);
        }
        batch
    }

    fn snapshot(&mut self, state: &GameState, id: ObjectId, chars: &Chars) -> usize {
        let obj = state.obj(id);
        let creature_toughness = chars.is(CardType::Creature).then(|| chars.toughness.unwrap_or(0).max(0) as u32);
        let other_lethal = [
            chars.is(CardType::Planeswalker).then(|| obj.counters(CounterKind::Loyalty).max(0) as u32),
            chars.is(CardType::Battle).then(|| obj.counters(CounterKind::Defense).max(0) as u32),
        ].into_iter().flatten().min();
        self.dealt.push(DamageRecipient {
            object: id, marked: 0, heal_marked: false, touched: false, loyalty: 0, defense: 0, amount: 0,
            creature_toughness, prior_marked: obj.damage, other_lethal, deathtouch: false,
        });
        self.dealt.len() - 1
    }

    /// Whether this event's damage to `player` is dealt as though its source had infect (CR 702.90b): asked of the static
    /// abilities the first time the event deals `player` damage, before it changes their life.
    fn as_though_infect(&mut self, cx: &Cx, player: PlayerId) -> bool {
        if let Some(&(_, infect)) = self.as_infect.iter().find(|(p, _)| *p == player) {
            return infect;
        }
        let infect = crate::static_rules::damage_as_though_infect(cx, cx.cards(), player);
        self.as_infect.push((player, infect));
        infect
    }

    /// Finish and announce one damage event. First finalize the actual damage and collect its triggers (CR 120.4b),
    /// before any result-choice suspension can remove their sources or change their controllers (CR 603.3a).
    /// Then prepare every result in one APNAP pass (CR 120.4c, 616.1), with one loss per player but one lifelink gain
    /// per source (CR 119.9-10, 702.15e). No original result applies until all its replacements are prepared.
    /// Finally apply and record the results, collecting only their triggers then (CR 120.4d, 603.2c); the already
    /// recorded dealt-damage events are neither recorded nor collected twice.
    pub fn finish(&mut self, cx: &mut Cx, events: &mut Vec<Event>) {
        if events.is_empty() {
            return;
        }
        // Excess is a recipient's total across the whole simultaneous event, not one assignment's (CR 120.10).
        for event in events.iter_mut() {
            if let Event::Damage { target: Target::Object(id), recipient_excess, .. } = event {
                *recipient_excess = self.dealt.iter().find(|r| r.object == *id).map_or(0, DamageRecipient::excess);
            }
        }
        // Actual damage is already dealt at CR 120.4b: record the entire simultaneous damage before looking for
        // triggers, so history-dependent continuous effects see it (CR 603.10, 611.3a). Result records still wait.
        // Both trigger phases keep the boundary from before this instruction, even across replacement continuations.
        let start = cx.record_batch(events);
        let damage_count = events.len();
        settle::collect_batch(cx, events, None, start);

        let mut life = std::mem::take(&mut self.life);
        life.extend(std::mem::take(&mut self.lifelink).into_iter().map(|(_, controller, amount)| (controller, amount)));
        let mut changes = Vec::with_capacity(life.len());
        let mut counters = std::mem::take(&mut self.counters);
        while let Some(player) = counters.next_player(cx, |player| life.iter().any(|&(p, _)| p == player)) {
            if let Some(at) = life.iter().position(|&(p, _)| p == player) {
                let (_, amount) = life.remove(at);
                let delta = prepare_life_change(cx, player, amount);
                if delta != 0 {
                    changes.push((player, delta));
                }
            } else {
                counters.prepare_next(cx, player, counters::How::Effect);
            }
        }
        // A replacement's continuation or a concession may have changed a recipient or a prohibition. Judge all of
        // them against the same state, before any of the original life, counter or marked-damage results applies.
        retain_life_changes(cx, &mut changes);
        counters.retain_allowed(cx);
        apply_life_changes(cx, changes, events);
        events.extend(counters.place(cx));
        for recipient in &self.dealt {
            let id = recipient.object;
            if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
                continue;
            }
            if recipient.marked > 0 || recipient.heal_marked || recipient.touched {
                cx.st().update_obj(id, |o| {
                    if recipient.heal_marked {
                        o.damage = 0;
                    }
                    o.damage += recipient.marked;
                    o.deathtouched |= recipient.touched;
                });
            }
            for (kind, amount) in [(CounterKind::Loyalty, recipient.loyalty), (CounterKind::Defense, recipient.defense)] {
                if amount > 0 {
                    let removed = amount.min(cx.s().obj(id).counters(kind).max(0) as u32) as i32;
                    counters::remove(cx, id, kind, removed);
                }
            }
        }
        cx.record_batch(&events[damage_count..]);
        if events.len() > damage_count {
            settle::collect_batch(cx, &events[damage_count..], None, start);
        } else if damage_count > 0 {
            // Damage whose only results are marks or removed counters still leaves a new state to judge (CR 603.8).
            settle::state_triggers_now(cx);
        }
    }

    /// The rest of the prevention effects that prevented some of the event's damage (CR 615.5), once it has all been dealt and
    /// announced: a "next time" effect is used up (CR 615.8), and one with more to do does it ("Deflecting Palm deals that
    /// much damage to that source's controller"), before state-based actions are checked. Every caller of `finish` calls it
    /// once the event is announced.
    pub fn rest_of_preventions(&mut self, cx: &mut Cx) {
        if !self.prevented.is_empty() {
            replace::rest_of_preventions(cx, std::mem::take(&mut self.prevented));
        }
    }
}

/// The last-known row of `source` as it deals damage (`Event::Damage::known`): its characteristics then, `chars`, which the
/// damage was judged by, and its controller.
fn dealer_known(cx: &mut Cx, source: ObjectId, chars: &Chars) -> u32 {
    let row = last_known_from(cx, source, chars);
    cx.st().last_known_push(row)
}

/// The characteristics of a damage component at the damage-dealt stage, before
/// any simultaneous component is processed into life loss, counters or marks.
struct DamageView { source: Chars, target: Option<Chars>, controller: PlayerId, known: u32 }
fn damage_view(cx: &mut Cx, event: &DamageEvent) -> Option<DamageView> {
    if event.amount <= 0 { return None; }
    let target = match event.target {
        Target::Player(p) if cx.s().in_game(p) => None,
        Target::Object(id) if live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Battlefield) => Some(layers::chars(cx.cards(), cx.s(), &cx.g.log, id)),
        _ => return None,
    };
    let source = layers::source_chars(cx.cards(), cx.s(), &cx.g.log, event.source);
    let controller = cx.controller(event.source);
    let known = dealer_known(cx, event.source, &source);
    Some(DamageView { source, target, controller, known })
}

/// A focused test may exercise one prepared assignment directly; production
/// damage always forms the whole event through `damage_batch`.
#[cfg(test)]
pub fn deal_damage_with(cx: &mut Cx, source: ObjectId, target: Target, amount: u32, combat: bool, batch: &mut DamageBatch) -> Option<Event> {
    let shielded = batch.shielded.iter().copied().find(|&id| target == Target::Object(id));
    let mut event = replace::damage(cx, DamageEvent { source, target, amount, combat, heal_marked: false, shielded }, &mut batch.prevented);
    if let Some(id) = event.shielded && !batch.shielded.contains(&id) { batch.shielded.push(id); }
    if event.amount > 0 && let Some(by) = protection_prevents(cx, event.source, event.target, event.combat) {
        cx.emit(Event::DamagePrevented { source: event.source, target: event.target, amount: event.amount, combat: event.combat, by });
        event.amount = 0;
    }
    let view = damage_view(cx, &event);
    apply_damage_with(cx, event, view, batch)
}

fn apply_damage_with(cx: &mut Cx, event: DamageEvent, view: Option<DamageView>, batch: &mut DamageBatch) -> Option<Event> {
    let Some(view) = view else { return None };
    let (source, target, amount, combat, heal_marked) = (event.source, event.target, event.amount, event.combat, event.heal_marked);
    let DamageView { source: source_chars, target: target_chars, controller: source_controller, known } = view;
    match target {
        Target::None | Target::Ability(_) => None,
        Target::Player(p) => {
            // Damage from a source with infect is dealt to a player as that many poison counters (CR 702.90b, 120.3b), and
            // so is damage dealt "as though its source had infect" (Phyrexian Unlife).
            let infect = source_chars.has(Keyword::Infect) || batch.as_though_infect(cx, p);
            // Combat damage dealt to a player by a creature with toxic also gives them poison counters equal to its total
            // toxic value, whatever the amount dealt (CR 702.164c, 120.3g): prevented, no damage is dealt and none are given.
            let toxic = if combat && source_chars.is(CardType::Creature) { source_chars.toxic } else { 0 };
            if !infect {
                // All of this event's noninfect damage to one player causes one loss (CR 120.3a); keep it distinct
                // from every source-level lifelink gain, including a gain later replaced with a loss.
                match batch.life.iter_mut().find(|(player, _)| *player == p) {
                    Some((_, loss)) => *loss -= i64::from(amount),
                    None => batch.life.push((p, -i64::from(amount))),
                }
            }
            let poison = (if infect { amount } else { 0 } + u32::from(toxic)).min(u32::from(u16::MAX)) as u16;
            if poison > 0 {
                // "That source's controller" (CR 120.3b) and "that creature's controller" (CR 702.164c) gives them, at once
                // as results of the one damage, so "if you would put one or more counters" is theirs.
                let by = source_controller;
                batch.counters.push_player(counters::PlayerPlacement {
                    player: p, counts: counters::PlayerCounts { poison, ..Default::default() }, by,
                });
            }
            lifelink(source, source_controller, &source_chars, amount, batch);
            Some(Event::Damage { source, target, amount, kind: DamageKind::new(combat, source_chars.colors), controller: source_controller, recipient_excess: 0, known })
        }
        Target::Object(id) => {
            if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
                return None;
            }
            let target_chars = target_chars.expect("a permanent damage recipient");
            let deathtouch = source_chars.has(Keyword::Deathtouch);
            // The recipient's characteristics and counters are read before this event deals it damage. Later hits in
            // the same event must not see loyalty already removed or toughness reduced by that event's infect damage.
            let at = match batch.dealt.iter().position(|r| r.object == id) {
                Some(at) => at,
                None => batch.snapshot(cx.s(), id, &target_chars),
            };
            if heal_marked {
                batch.dealt[at].prior_marked = 0;
            }
            batch.dealt[at].amount += amount;
            // CR 120.4a: any damage beyond 1 dealt to a creature by a source with deathtouch is excess, combat or not.
            batch.dealt[at].deathtouch |= deathtouch;
            if target_chars.is(CardType::Creature) {
                if source_chars.has(Keyword::Infect) || source_chars.has(Keyword::Wither) {
                    // Damage from a source with infect or wither is dealt to a creature as that many -1/-1 counters (CR
                    // 702.90c, 702.80a, 120.3d), which "that source's controller" puts on it, whoever's turn or spell it is; it is still
                    // damage dealt, so deathtouch counts (CR 702.2b). Put as an effect's: infect and wither are static abilities (CR
                    // 702.90a, 702.80a), static abilities create effects (CR 609.1), and it is their effect, not the turn-based action dealing
                    // combat damage, that puts the counters (CR 120.3d) - so "if an effect would put counters" applies.
                    let mut counts = [0; CounterKind::COUNT];
                    counts[CounterKind::MinusOne as usize] = amount.min(i32::MAX as u32) as i32;
                    batch.counters.push(counters::Placement { object: id, counts, by: source_controller });
                } else {
                    // CR 614.1a: healing removes the marks from before this event, never this event's own marks.
                    batch.dealt[at].heal_marked |= heal_marked;
                    batch.dealt[at].marked += amount;
                }
                batch.dealt[at].touched |= deathtouch;
            }
            // Loyalty and defense removal are also results, not updates made while damage replacements or another
            // player's result choices are still pending (CR 120.3c, 120.3h, 120.4c-d).
            if target_chars.is(CardType::Battle) {
                batch.dealt[at].defense += amount;
            }
            if target_chars.is(CardType::Planeswalker) {
                batch.dealt[at].loyalty += amount;
            }
            lifelink(source, source_controller, &source_chars, amount, batch);
            Some(Event::Damage { source, target, amount, kind: DamageKind::new(combat, source_chars.colors), controller: source_controller, recipient_excess: 0, known })
        }
    }
}

/// Whether protection prevents the damage `source` is about to deal to `target` (CR 702.16e): a permanent's protection from
/// the source's qualities or its controller, or a player's protection from everything or from the source's controller
/// (CR 702.16j, 702.16k), unless the damage can't be prevented (CR 615.12). The object whose protection it is - the
/// permanent, or the source of the player's effect or static ability - for a player ordering it with replacement effects
/// (CR 616.1).
pub(crate) fn protection_prevents(cx: &Cx, source: ObjectId, target: Target, combat: bool) -> Option<ObjectId> {
    let by = match target {
        Target::Player(p) => layers::player_protection(cx.cards(), cx.s(), &cx.g.log, p, cx.controller(source))?,
        Target::Object(id) => {
            if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
                return None;
            }
            let target_chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, id);
            let source_chars = layers::source_chars(cx.cards(), cx.s(), &cx.g.log, source);
            if !target_chars.protected_from_by(&source_chars, || cantrip_api::mana_value_of(&|card| cx.card(card), cx.s(), source, &source_chars), cx.controller(source)) {
                return None;
            }
            id
        }
        _ => return None,
    };
    damage_can_be_prevented(cx, source, combat).then_some(by)
}

/// Whether a prevention effect may prevent the damage `source` is about to deal (CR 615.12): not in a turn an effect said
/// damage can't be prevented this turn (Bonecrusher Giant's Stomp), nor while a static ability says this damage can't be
/// (Sunspine Lynx's "Damage can't be prevented", Questing Beast's combat damage). Every prevention effect asks it:
/// protection's (CR 702.16e) and those of `replace` (CR 615).
pub(crate) fn damage_can_be_prevented(cx: &Cx, source: ObjectId, combat: bool) -> bool {
    let turn = cx.s().turn;
    turn.no_prevention_turn != turn.number && !crate::static_rules::damage_cant_be_prevented(cx, source, combat)
}

/// Damage dealt by a source with lifelink also causes its controller to gain that much life (CR 702.15b), as part of
/// the damage event, not a trigger: added to what the source has dealt in this event, gained once it is all dealt.
fn lifelink(source: ObjectId, controller: PlayerId, chars: &Chars, amount: u32, batch: &mut DamageBatch) {
    if chars.has(Keyword::Lifelink) {
        match batch.lifelink.iter_mut().find(|(s, _, _)| *s == source) {
            Some(entry) => entry.2 += i64::from(amount),
            None => batch.lifelink.push((source, controller, amount as i64)),
        }
    }
}

/// Each opponent of `player` still in the game loses `amount` life simultaneously (CR 101.4, 119.3): the total lost.
pub fn each_opponent_loses_life(cx: &mut Cx, player: PlayerId, amount: i32) -> i32 {
    let losses: Vec<_> = cx.s().opponents(player).map(|p| (p, amount)).collect();
    lose_life_together(cx, &losses)
}

/// The losses of one instruction (CR 101.4, 119.3, 608.2h): coalesced per player, replaced in APNAP order (CR 616.1)
/// before any of its life totals change, then announced as one event (CR 603.2c). Only the final losses count towards
/// "life lost this way", not gains or the separate actions a replacement's continuation performs.
pub fn lose_life_together(cx: &mut Cx, losses: &[(PlayerId, i32)]) -> i32 {
    let mut merged: Vec<(PlayerId, i64)> = Vec::with_capacity(losses.len());
    for &(player, n) in losses.iter().filter(|&&(_, n)| n > 0) {
        match merged.iter_mut().find(|(p, _)| *p == player) {
            Some(loss) => loss.1 -= i64::from(n),
            None => merged.push((player, -i64::from(n))),
        }
    }
    life_together(cx, &merged)
}

/// The changes of life of one simultaneous event, each its own occurrence (CR 101.4, 119.3): replaced and judged allowed
/// (`prepare_life_changes`, `retain_life_changes`) before any of them applies, then announced together (CR 603.2c). The life
/// lost.
fn life_together(cx: &mut Cx, life: &[(PlayerId, i64)]) -> i32 {
    let mut changes = prepare_life_changes(cx, life);
    retain_life_changes(cx, &mut changes);
    let mut events = Vec::with_capacity(changes.len());
    let total = apply_life_changes(cx, changes, &mut events);
    if !events.is_empty() {
        cx.emit_batch(&events, None);
    }
    total
}

/// One instruction whose parts several players carry out at once (CR 101.4, `Game::together`): the sacrifices (CR 701.21a),
/// the discards (CR 701.9) and the changes of life (CR 119.3) of `parts`, each change of life its own occurrence. Every
/// replacement of every part is worked out against the game as it is before any of them happens, then they all happen and are
/// announced together (CR 603.2c) - the discards' own events after them, as every discard's are (`discard_events`). The life
/// lost.
pub fn together(cx: &mut Cx, parts: cantrip_api::Together) -> i32 {
    let life: Vec<(PlayerId, i64)> = parts.life.iter().filter(|&&(_, n)| n != 0).map(|&(player, n)| (player, i64::from(n))).collect();
    let mut moves = discard_moves(cx, parts.discards, &[], false);
    let discarding = moves.len();
    moves.extend(sacrifice_moves(cx, parts.sacrifices));
    if moves.is_empty() {
        return life_together(cx, &life);
    }
    let (_, seen, lost) = cx.instruction(|cx| move_together_entering_event(cx, &moves, ZoneReason::Other, None, &life), |_| true);
    let events = discarded_events(cx, &moves[..discarding], &seen);
    if !events.is_empty() {
        cx.emit_batch(&events, None);
    }
    lost
}

/// Life results of one simultaneous event (CR 101.4, 120.4c, 616.1), with signed amounts, each its own occurrence: a player
/// who loses life "the appropriate number of times" (Rottenmouth Viper's rulings) suffers that many losses, each replaced on
/// its own and announced as its own change, so "whenever a player loses life" triggers for each (CR 603.2c). A caller whose
/// results are one loss for each player merges them first (`lose_life_together`); damage's are one loss for each player and
/// one lifelink gain for each source (CR 119.9-10, 702.15e). Replacements run in APNAP order, each player's results in the
/// order given, before any original life update, including when a replacement changes a result's sign.
fn prepare_life_changes(cx: &mut Cx, changes: &[(PlayerId, i64)]) -> Vec<(PlayerId, i32)> {
    let mut prepared = Vec::with_capacity(changes.len());
    if changes.is_empty() {
        return prepared;
    }
    for player in cx.s().apnap() {
        for &(p, amount) in changes {
            if p != player {
                continue;
            }
            let delta = prepare_life_change(cx, player, amount);
            if delta != 0 {
                prepared.push((player, delta));
            }
        }
    }
    prepared
}

/// A later replacement's continuation may remove a player or change a prohibition. Judge every final prohibition
/// before applying any original result, including the counter results of damage (CR 101.2, 120.4c-d).
fn retain_life_changes(cx: &Cx, changes: &mut Vec<(PlayerId, i32)>) {
    changes.retain(|&(player, delta)| cx.s().in_game(player) && !life_total_cant_change(cx.s(), player) && !blocked(cx, player, delta));
}

/// Apply prepared, allowed life results through the journal, returning the life actually lost and leaving their
/// announcements with the caller. Damage announces them with its other results, an explicit loss instruction on its own
/// (CR 603.2c). No replacement or prohibition is judged against a partially applied simultaneous event.
fn apply_life_changes(cx: &mut Cx, changes: Vec<(PlayerId, i32)>, events: &mut Vec<Event>) -> i32 {
    let mut total = 0i64;
    for (player, delta) in changes {
        cx.st().update_player(player, |p| p.life += delta);
        if delta < 0 {
            total -= i64::from(delta);
        }
        events.push(Event::LifeChange { player, delta });
    }
    total as i32
}

/// Whether `player`'s life total can't change (CR 119.7, 119.8): a resolved effect says so (`EffectKind::LifeTotalCantChange`).
pub fn life_total_cant_change(s: &GameState, player: PlayerId) -> bool {
    !s.effects.is_empty() && s.effects.iter().any(|e| e.kind == EffectKind::LifeTotalCantChange(player))
}

/// `player`'s life total goes up or down by `delta` (CR 119.3): an effect's, a cost's (CR 119.4), or a life total set to a
/// number (CR 119.5). Simultaneous losses and damage results use the same preparation through `prepare_life_changes`,
/// deferring their updates and announcement until every player's change is prepared.
pub fn change_life(cx: &mut Cx, player: PlayerId, delta: impl Into<i64>) {
    let delta = prepare_life_change(cx, player, delta);
    if delta == 0 {
        return;
    }
    cx.st().update_player(player, |p| p.life += delta);
    // CR 702.179d: an opponent losing life during the turn of a player with speed triggers its inherent ability
    // (`designations::inherent_triggers`).
    cx.emit(Event::LifeChange { player, delta });
}

/// The shared life-change pipeline before its update: single-player changes apply it at once; simultaneous losses and
/// damage results prepare every player's changes first (CR 101.4, 120.4c, 614.1a, 616.1).
fn prepare_life_change(cx: &mut Cx, player: PlayerId, delta: impl Into<i64>) -> i32 {
    let delta = delta.into() as i32;
    if delta == 0 || !cx.s().in_game(player) {
        return 0;
    }
    // CR 119.7: a player who can't gain life gains none, and nothing happens: no life gain event, so nothing that
    // triggers on one (CR 119.9) triggers, and no replacement effect of a life gain event does anything.
    if blocked(cx, player, delta) {
        return 0;
    }
    // CR 119.7, 119.8: a player whose life total can't change gains and loses none, and nothing replaces what can't happen
    // (Teferi's Protection's rulings: "Words of Worship's effect ... ends up replacing the event with nothing").
    if life_total_cant_change(cx.s(), player) {
        return 0;
    }
    // CR 614.1a, 616.1: "you gain that much life plus 1 instead", "they lose twice that much life instead". The event
    // history records the life that actually changed. A gain an effect turned into a loss, or a loss into a gain, the player
    // can't have is nothing (the Platinum Emperion rulings).
    let delta = crate::replace::life_change(cx, player, delta);
    if delta == 0 || !cx.s().in_game(player) || blocked(cx, player, delta) {
        return 0;
    }
    delta
}

/// Whether `player` can't have this change of life: a gain while they can't gain life (CR 119.7), a loss while they can't lose
/// life (CR 119.8).
fn blocked(cx: &Cx, player: PlayerId, delta: i32) -> bool {
    if delta > 0 {
        crate::static_rules::cant_gain_life(cx, cx.cards(), player)
    } else {
        crate::static_rules::cant_lose_life(cx, cx.cards(), player)
    }
}

/// Regeneration (CR 701.19a-b): tap, heal and remove from combat. Only those
/// whose replacement came from a shield use one up; a static ability consumes
/// no shield. The replacement was judged before the simultaneous moves.
pub fn regenerate_instead(cx: &mut Cx, ids: &[(ObjectId, bool)]) -> Vec<ObjectId> {
    let mut saved = Vec::new();
    for &(id, shield) in ids {
        if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
            continue;
        }
        if shield {
            let Some(at) = cx.s().effects.iter().position(|e| e.kind == EffectKind::RegenerationShield && e.affected == id) else {
                continue;
            };
            let keep: Vec<bool> = (0..cx.s().effects.len()).map(|i| i != at).collect();
            let effects_before = cx.s().effects.len();
            cx.st().effects_keep(&keep);
            // An effect that ends changes the game with no event (CR 603.8).
            if cx.s().effects.len() < effects_before {
                cx.state_changed();
            }
        }
        cx.st().update_obj(id, |o| {
            o.damage = 0;
            o.deathtouched = false;
        });
        tap(cx, id);
        // An attacking or blocking creature that regenerates is removed from combat (CR 506.4) - a planeswalker being attacked
        // that is one too with it; one that is neither is not.
        if cx.s().attackers.iter().any(|a| a.id == id) || cx.s().blockers.contains(&id) {
            remove_from_combat(cx, &[id]);
        }
        saved.push(id);
    }
    saved
}


/// Destroys permanents together (CR 701.8a): each that is still the object it was, on the battlefield and not indestructible
/// (CR 702.12b) goes to its owner's graveyard, all at once (CR 603.10a). With `Destroy` a regeneration shield replaces the
/// destruction (CR 701.19a); with `DestroyNoRegen` it does not (CR 701.19c). The new objects.
pub fn destroy_together(cx: &mut Cx, ids: &[ObjectId], reason: ZoneReason) -> Vec<ObjectId> {
    let moves: Vec<Moving> = ids
        .iter()
        .copied()
        .filter(|&id| cx.can_destroy(id))
        .map(|id| Moving::to(id, Zone::graveyard(cx.s().obj(id).owner)))
        .collect();
    move_together(cx, &moves, reason)
}

/// Whether `player` can sacrifice `id` now, as what is happening would have them (CR 701.21a, 101.2): still the object it
/// was, on the battlefield and controlled by `player` ("a player can't sacrifice ... a permanent they don't control"),
/// nothing says it can't be sacrificed (`static_rules::cant_be_sacrificed`), and, while a spell or ability resolves,
/// nothing forbids that spell or ability to cause `player` to sacrifice (Sigarda, Host of Herons). What `sacrifice`
/// sacrifices, and what a choice of what to sacrifice may take.
pub fn sacrificeable(cx: &Cx, player: PlayerId, id: ObjectId) -> bool {
    live(cx.s(), id)
        && cx.s().obj(id).zone.is(ZoneKind::Battlefield)
        && cx.s().obj(id).controller == player
        && !crate::static_rules::cant_be_sacrificed(cx, cx.cards(), id)
        && cx.effect_controller().is_none_or(|by| cantrip_api::may_cause(cx, by, player, cantrip_api::CauseKind::Sacrifice))
}

/// `player` sacrifices permanents (CR 701.21a), together, in the order given: each that is `sacrificeable` by them, so one
/// they don't control, one that can't be sacrificed, or any when the resolving spell or ability can't make them sacrifice,
/// stays (CR 101.2). The new objects.
pub fn sacrifice(cx: &mut Cx, player: PlayerId, ids: &[ObjectId]) -> Vec<ObjectId> {
    let each: Vec<(PlayerId, ObjectId)> = ids.iter().map(|&id| (player, id)).collect();
    sacrifice_each(cx, &each)
}

/// Several players sacrifice permanents at once (CR 701.21a, 101.4): each pair is who sacrifices and what, and each that
/// player can't sacrifice (`sacrificeable`) stays. The new objects, in order.
pub fn sacrifice_each(cx: &mut Cx, each: &[(PlayerId, ObjectId)]) -> Vec<ObjectId> {
    let moves = sacrifice_moves(cx, each);
    move_together(cx, &moves, ZoneReason::Sacrifice)
}

/// The moves of `sacrifice_each`: each permanent its player can sacrifice (`sacrificeable`) to its owner's graveyard.
fn sacrifice_moves(cx: &Cx, each: &[(PlayerId, ObjectId)]) -> Vec<Moving> {
    each.iter()
        .filter(|&&(player, id)| sacrificeable(cx, player, id))
        .map(|&(_, id)| Moving { reason: Some(ZoneReason::Sacrifice), ..Moving::to(id, Zone::graveyard(cx.s().obj(id).owner)) })
        .collect()
}

/// The objects among `ids` that are still the objects they were, for a verb that moves them (CR 400.7).
fn live_ones(cx: &Cx, ids: &[ObjectId]) -> Vec<ObjectId> {
    // A phased-out permanent can't be affected by anything (CR 702.26b): an ability that remembered it moves nothing.
    ids.iter().copied().filter(|&id| live(cx.s(), id) && !cx.s().obj(id).zone.is(ZoneKind::PhasedOut)).collect()
}

/// Puts objects together into their owners' graveyards (CR 400.3), in the order given unless their owner arranges them (CR
/// 404.3, `arrange_graveyards`), from anywhere but the battlefield
/// (`Game::put_into_graveyard`): a permanent among them is not moved, as a permanent goes to a graveyard by the verbs that
/// say why (destroy, sacrifice, CR 701.8, 701.21) or a move that names its reason (`Game::move_together`).
pub fn put_into_graveyard(cx: &mut Cx, ids: &[ObjectId]) -> Vec<ObjectId> {
    let ids: Vec<ObjectId> = ids.iter().copied().filter(|&id| !cx.s().obj(id).zone.is(ZoneKind::Battlefield)).collect();
    let moves: Vec<Moving> = live_ones(cx, &ids).into_iter().map(|id| Moving::to(id, Zone::graveyard(cx.s().obj(id).owner))).collect();
    move_together(cx, &moves, ZoneReason::Other)
}

/// Exiles objects together, each to its owner's exile zone (CR 406.2).
pub fn exile_all(cx: &mut Cx, ids: &[ObjectId]) -> Vec<ObjectId> {
    let moves: Vec<Moving> = live_ones(cx, ids).into_iter().map(|id| Moving::to(id, Zone::exile(cx.s().obj(id).owner))).collect();
    move_together(cx, &moves, ZoneReason::Exile)
}

/// A number below `bound` from the game's own generator (none is drawn when there is no choice).
pub fn random_below(cx: &mut Cx, bound: u32) -> u32 {
    if bound <= 1 { 0 } else { cx.random_below(bound) }
}

/// Puts cards of a player's library on its top in this order, the first on top, or on its bottom, the last lowest. The
/// cards are already in the library; nothing changes zones.
pub fn arrange_library(cx: &mut Cx, player: PlayerId, top: &[ObjectId], bottom: &[ObjectId]) {
    let library = Zone::library(player);
    let reordered_revealed = cx.s().player(player).revealed_library_top
        .is_some_and(|id| top.contains(&id) || bottom.contains(&id));
    for &id in top.iter().chain(bottom) {
        cx.st().zone_remove(library, id);
    }
    for (at, &id) in top.iter().enumerate() {
        cx.st().zone_insert(library, at, id);
    }
    for &id in bottom {
        cx.st().zone_push(library, id);
    }
    // CR 701.20d: a revealed card in the reordered selection becomes a new object,
    // even when it finishes on top. Renew after ordering so the input IDs stay live.
    if reordered_revealed {
        crate::library_visibility::conceal(cx, player);
    }
    crate::library_visibility::sync(cx);
}

/// Puts cards already in a player's library on its bottom in a random order (CR 401.4), the order drawn from the game's own
/// generator.
pub fn bottom_in_random_order(cx: &mut Cx, player: PlayerId, cards: &[ObjectId]) {
    let mut cards = cards.to_vec();
    for i in (1..cards.len()).rev() {
        let j = random_below(cx, i as u32 + 1) as usize;
        cards.swap(i, j);
    }
    arrange_library(cx, player, &[], &cards);
    for &card in &cards {
        cx.record(Event::Randomized { player, card });
    }
}

/// Puts objects into their owners' libraries together and arranges them there (CR 401.4).
pub fn put_into_library(cx: &mut Cx, ids: &[ObjectId], place: LibraryPlace) -> Vec<ObjectId> {
    as_one_instruction(cx, |cx| put_into_library_ordered(cx, ids, place))
}

fn put_into_library_ordered(cx: &mut Cx, ids: &[ObjectId], place: LibraryPlace) -> Vec<ObjectId> {
    // Shuffling specific objects is a separate instruction from moving them. CR 701.24c requires the referenced
    // owners' libraries to be shuffled even when an object has left, or a replacement prevents it from arriving.
    // Shuffling the owner's library doesn't affect a phased-out permanent, so its owner is still a shuffle reference.
    let mut shuffle_owners: Vec<PlayerId> = Vec::new();
    if place == LibraryPlace::ShuffledIn {
        for &id in ids {
            let obj = cx.s().obj(id);
            if !shuffle_owners.contains(&obj.owner) {
                shuffle_owners.push(obj.owner);
            }
        }
    }
    let moves: Vec<Moving> = live_ones(cx, ids).into_iter().map(|id| Moving::to(id, Zone::library(cx.s().obj(id).owner))).collect();
    let new = move_together(cx, &moves, ZoneReason::Other);
    let arrived: Vec<ObjectId> = new.iter().copied().filter(|&n| cx.s().obj(n).zone.is(ZoneKind::Library)).collect();
    let owners: Vec<PlayerId> = if place == LibraryPlace::ShuffledIn {
        shuffle_owners
    } else {
        let mut owners = Vec::new();
        for &n in &arrived {
            let owner = cx.s().obj(n).owner;
            if !owners.contains(&owner) {
                owners.push(owner);
            }
        }
        owners
    };
    // A merged permanent's cards are in the order its owner arranged them as they arrived (CR 730.3a): kept together, in
    // that order, wherever in the library they go.
    let groups: Vec<Vec<ObjectId>> = ids
        .iter()
        .filter(|&&id| cx.s().merge_of(id).is_some())
        .map(|&id| Query::became_all(&*cx, id).into_iter().filter(|n| arrived.contains(n)).collect::<Vec<_>>())
        .filter(|g: &Vec<ObjectId>| g.len() > 1)
        .collect();
    for owner in owners {
        let mut mine: Vec<ObjectId> = arrived.iter().copied().filter(|&n| cx.s().obj(n).owner == owner).collect();
        if !groups.is_empty() {
            let library = cx.s().zone(Zone::library(owner));
            let at = |n: &ObjectId| library.iter().position(|c| c == n).unwrap_or(usize::MAX);
            let mut ordered: Vec<ObjectId> = Vec::with_capacity(mine.len());
            for &n in &mine {
                if ordered.contains(&n) {
                    continue;
                }
                match groups.iter().find(|g| g.contains(&n)) {
                    Some(group) => {
                        let mut group = group.clone();
                        group.sort_by_key(at);
                        ordered.extend(group);
                    }
                    None => ordered.push(n),
                }
            }
            mine = ordered;
        }
        match place {
            LibraryPlace::Top => arrange_library(cx, owner, &mine, &[]),
            LibraryPlace::Bottom => arrange_library(cx, owner, &[], &mine),
            LibraryPlace::BottomInRandomOrder => bottom_in_random_order(cx, owner, &mine),
            LibraryPlace::ShuffledIn => crate::turn::shuffle_library(cx, owner),
            LibraryPlace::FromTop(depth) => {
                let library = Zone::library(owner);
                for &id in &mine {
                    cx.st().zone_remove(library, id);
                }
                for (k, &id) in mine.iter().enumerate() {
                    let at = (depth + k).min(cx.s().zone(library).len());
                    cx.st().zone_insert(library, at, id);
                }
            }
        }
    }
    new
}

/// Returns objects to their owners' hands together (CR 400.3, 101.4).
pub fn return_to_hand(cx: &mut Cx, ids: &[ObjectId]) -> Vec<ObjectId> {
    return_to_hand_as(cx, ids, false)
}

/// `return_to_hand`, the move `revealed` (`Moving::revealed`) when the instruction moving the cards reveals them.
pub fn return_to_hand_as(cx: &mut Cx, ids: &[ObjectId], revealed: bool) -> Vec<ObjectId> {
    let moves: Vec<Moving> =
        live_ones(cx, ids).into_iter().map(|id| Moving { revealed, ..Moving::to(id, Zone::hand(cx.s().obj(id).owner)) }).collect();
    move_together(cx, &moves, ZoneReason::Other)
}

/// Brings cards their owners have outside the game into the game, into their owners' hands (CR 400.11b, 108.3b): each
/// arrives as a new object and stays in the game until it ends. Only a card still outside the game moves (CR 400.11c). A
/// basic land of a Limited event's supply (`GameState::outside_supply`) is replaced outside the game by another, since a
/// Limited sideboard holds any number of them.
pub fn bring_into_hand(cx: &mut Cx, ids: &[ObjectId]) -> Vec<ObjectId> {
    bring_into_hand_as(cx, ids, false)
}

/// `bring_into_hand`, the move `revealed` (`Moving::revealed`) when the instruction bringing the cards in reveals them.
pub fn bring_into_hand_as(cx: &mut Cx, ids: &[ObjectId], revealed: bool) -> Vec<ObjectId> {
    bring_into_hand_inner(cx, ids, revealed, false)
}

/// The companion special action (CR 702.139c): unlike ordinary outside-game effects, it is allowed in Commander.
pub fn bring_companion_into_hand(cx: &mut Cx, id: ObjectId) -> Vec<ObjectId> {
    bring_into_hand_inner(cx, &[id], false, true)
}

fn bring_into_hand_inner(cx: &mut Cx, ids: &[ObjectId], revealed: bool, companion: bool) -> Vec<ObjectId> {
    if cx.s().commander && !companion {
        return Vec::new();
    }
    // A basic land of the supply leaves another in its place as it moves (`move_together`).
    let moves: Vec<Moving> = live_ones(cx, ids)
        .into_iter()
        .filter(|&id| cx.s().obj(id).zone.is(ZoneKind::Sideboard))
        .map(|id| Moving { revealed, ..Moving::to(id, Zone::hand(cx.s().obj(id).owner)) })
        .collect();
    move_together(cx, &moves, ZoneReason::Other)
}

/// Ends the effects on `id` whose "for as long as" duration is no longer true of it (CR 611.2b), the moment it stops
/// being true: called wherever a counter is taken off an object, or a permanent untaps, so a condition made true again
/// later never brings such an effect back.
pub fn end_lapsed_durations(cx: &mut Cx, id: ObjectId) {
    if cx.s().settle_held.get() > 0 {
        // A simultaneous instruction settles every conditional duration after all its changes.
        return;
    }
    let obj = *cx.s().obj(id);
    if cx.s().effects.iter().any(|e| e.affected == id && !e.duration.holds_for(&obj)) {
        let control = cx.s().effects.iter().any(|e| e.affected == id && !e.duration.holds_for(&obj) && matches!(e.kind, EffectKind::Control(_)));
        let effects_before = cx.s().effects.len();
        cx.st().effects_retain(|e| e.affected != id || e.duration.holds_for(&obj));
        // An effect that ends changes the game with no event (CR 603.8).
        if cx.s().effects.len() < effects_before {
            cx.state_changed();
        }
        // A control-changing effect that ended gives the permanent back as it ends (CR 613.1b), and whatever its controller
        // decides with it.
        if control {
            settle_control(cx);
        }
    }
}

/// CR 714.3c: as the active player's precombat main phase begins, that player puts a lore counter on each Saga they
/// control that has chapter abilities - "that player puts", so "if you would put one or more counters" (Innkeeper's
/// Talent, Vorinclex) applies (CR 122.6a). A turn-based action (CR 703.4f), so neither an effect's counter (Doubling
/// Season) nor one that uses the stack.
pub fn add_lore_counters(cx: &mut Cx) {
    let active = cx.s().turn.active;
    let mut lore = [0; CounterKind::COUNT];
    lore[CounterKind::Lore as usize] = 1;
    let mut placements = Vec::new();
    for id in cx.s().battlefield().to_vec() {
        if cx.s().obj(id).controller != active {
            continue;
        }
        let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, id);
        if chars.has_subtype(Subtype::Saga)
            && chars.abilities().any(|a| matches!(cx.cards().ability(a), AbilityDef::Triggered(t) if t.chapter > 0))
        {
            placements.push(counters::Placement { object: id, counts: lore, by: active });
        }
    }
    // "A lore counter on each Saga": one action, all of them at once (CR 714.3c, 101.4).
    counters::put_together(cx, &placements, counters::How::TurnBased);
}

/// A player gets energy counters, put by `by` (CR 107.14, 122.6), or pays them with a negative `delta` (CR 122.1).
pub fn change_energy(cx: &mut Cx, player: PlayerId, delta: i32, by: PlayerId) {
    if delta > 0 {
        counters::put_on_player(cx, player, counters::PlayerCounts { energy: delta.min(u16::MAX as i32) as u16, ..Default::default() }, by);
        return;
    }
    let pay = (-delta).clamp(0, 255) as u8;
    cx.st().update_player(player, |p| p.pay_energy(pay));
}

/// Proliferate (CR 701.34a): the permanents on the battlefield and the players in the game that have a counter are
/// offered, battlefield order first; any number are chosen, and each gets one more of every kind it had. What each had is
/// read before any counter goes on, so one put on by a replacement effect is not "already there" for a later one. The
/// proliferating player puts them all (CR 122.6a).
pub fn proliferate(cx: &mut Cx, player: PlayerId, source: ObjectId) -> R<()> {
    let mut candidates: Vec<Target> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| cx.s().obj(id).counter_array().iter().any(|&n| n > 0))
        .map(Target::Object)
        .collect();
    for p in cx.s().seats() {
        let pl = cx.s().player(p);
        if !pl.lost && (pl.poison() > 0 || pl.energy() > 0 || pl.experience() > 0) {
            candidates.push(Target::Player(p));
        }
    }
    if candidates.is_empty() {
        return Ok(());
    }
    let ask = RequestKind::ChooseTarget { replacing: None, label: "", slot_player: None, together: None,
        source,
        purpose: Purpose::Targets,
        count: cantrip_state::count_u32(candidates.len()),
        candidates,
        optional: true,
            least: 0,
        any_number: true,
    };
    let Answer::Targets(chosen) = cx.ask_core(player, ask, false)? else { return Ok(()) };
    // Read every recipient before any replacement runs. All kinds on one recipient are one placement, whether
    // that recipient is a permanent or a player (CR 701.34a); no sequential per-kind player applications.
    let one = |n: u8| u16::from(n > 0);
    let mut placements = Vec::new();
    let mut players = Vec::new();
    for target in chosen {
        match target {
            Target::Object(object) => placements.push(counters::Placement {
                object, counts: cx.s().obj(object).counter_array().map(|n| i32::from(n > 0)), by: player,
            }),
            Target::Player(recipient) => {
                let p = cx.s().player(recipient);
                players.push(counters::PlayerPlacement {
                    player: recipient, by: player,
                    counts: counters::PlayerCounts { poison: one(p.poison()), energy: one(p.energy()), experience: one(p.experience()) },
                });
            }
            _ => {}
        }
    }
    // The counter path shares damage's APNAP scheduling and defers every original result until all replacements
    // have been prepared, including those for players (CR 101.4, 616.1).
    counters::put_mixed_together(cx, &placements, &players, counters::How::Effect);
    Ok(())
}

/// Taps an untapped permanent (CR 701.26a): only a live, phased-in permanent on the battlefield can become tapped.
pub fn tap(cx: &mut Cx, id: ObjectId) {
    if tap_quietly(cx, id) {
        cx.emit(Event::Tapped { permanent: id });
    }
}

/// Taps a permanent (CR 701.26a) without announcing it: whether it became tapped. For permanents tapped at once, whose
/// `Event::Tapped`s are announced together (`tap_together`, CR 603.2c).
pub fn tap_quietly(cx: &mut Cx, id: ObjectId) -> bool {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) || cx.s().obj(id).tapped {
        return false;
    }
    cx.st().update_obj(id, |o| o.tapped = true);
    true
}

/// Taps permanents at once (CR 701.26a, 603.2c): the attackers a declaration taps (CR 508.1f), the creatures a crew, saddle or
/// teamwork cost taps, any other cost that taps several (CR 601.2h, 602.2b). Each is tapped, then what watches them sees one
/// batch of events, so a "whenever one or more creatures you control become tapped" ability triggers once. The permanents that
/// became tapped.
pub fn tap_together(cx: &mut Cx, ids: &[ObjectId]) -> Vec<ObjectId> {
    tap_together_with(cx, &mut Vec::new(), ids)
}

/// `tap_together` for a cost whose `{T}` was paid already (CR 602.2b, 601.2h): the permanents in `pending` were tapped
/// quietly (`tap_quietly`) and are announced in the same batch as the ones tapped now, since one cost taps them at once. They
/// are announced even when none of `ids` becomes tapped, and `pending` is left empty.
pub fn tap_together_with(cx: &mut Cx, pending: &mut Vec<ObjectId>, ids: &[ObjectId]) -> Vec<ObjectId> {
    let tapped: Vec<ObjectId> = ids.iter().copied().filter(|&id| tap_quietly(cx, id)).collect();
    let events: Vec<Event> = pending.drain(..).chain(tapped.iter().copied()).map(|permanent| Event::Tapped { permanent }).collect();
    if !events.is_empty() {
        cx.emit_batch(&events, None);
    }
    tapped
}

/// Marks the game as one where static abilities may apply, once an object whose card has any is made, or one that has
/// hone counters (CR 122.1j) (the layers' guard; `GameState::statics_seen`).
pub(crate) fn note_statics(cx: &mut Cx, id: ObjectId) {
    if cx.s().statics_seen {
        return;
    }
    let obj = cx.s().obj(id);
    if cx.cards().texts(cx.s(), obj).any(|c| cx.cards().statics[c.0 as usize]) || obj.counters(CounterKind::Hone) > 0 {
        cx.st().see_statics();
    }
}

/// Whether untapping is possible before replacement effects (CR 118.3, 101.2, 701.26b). Stun replaces a payable
/// action rather than prohibiting it (CR 118.11, 122.1d). Shared by effects and the public cost-payability query.
pub fn can_untap(q: &dyn Query, cards: &CardTable, id: ObjectId) -> bool {
    live(q.state(), id)
        && q.obj(id).zone.is(ZoneKind::Battlefield)
        && q.obj(id).tapped
        && !crate::static_rules::cant_become_untapped(q, cards, id)
}

/// Untaps a permanent (CR 701.26b), through the same authority as a simultaneous untap.
pub fn untap(cx: &mut Cx, id: ObjectId) {
    untap_together(cx, &[id]);
}

/// Untaps permanents simultaneously (CR 502.3, 701.26b, 603.2c): all prohibitions and stun replacements are decided
/// against the same pre-event state (CR 101.2, 614.6, 122.1d). Removing a stun counter instead does not untap, so cannot
/// end a tapped duration or produce an Untapped event. Only after every status/counter changes do durations/control
/// settle and triggers see one batch. Magnetic Mountain and Intruder Alarm need this same simultaneous instruction.
pub fn untap_together(cx: &mut Cx, ids: &[ObjectId]) {
    let mut plan = Vec::new();
    for &id in ids {
        if !plan.iter().any(|&(other, _)| other == id) && can_untap(cx, cx.cards(), id) {
            plan.push((id, cx.s().obj(id).counters(CounterKind::Stun) > 0));
        }
    }
    if plan.is_empty() {
        return;
    }
    // No intermediate status or duration change can affect another member of this event. In particular, removing a
    // stun counter may end a control duration, which must not change the remaining prohibitions or replacements.
    as_one_instruction(cx, |cx| {
        for &(id, stun) in &plan {
            if stun {
                counters::remove(cx, id, CounterKind::Stun, 1);
            } else {
                cx.st().update_obj(id, |o| o.tapped = false);
            }
        }
    });
    let events: Vec<Event> = plan.into_iter().filter(|&(_, stun)| !stun).map(|(permanent, _)| Event::Untapped { permanent }).collect();
    if !events.is_empty() {
        cx.emit_batch(&events, None);
    }
}

/// Exerts a permanent (CR 701.43a): it won't untap during `player`'s next untap step.
pub fn exert(cx: &mut Cx, id: ObjectId, player: PlayerId) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
        return;
    }
    cx.st().update_obj(id, |o| o.marks.exerted_by |= 1 << player.0);
    cx.emit(Event::Exerted { permanent: id, player });
}

/// Monstrosity N (CR 701.37a): a permanent that is not monstrous gets N +1/+1 counters and becomes monstrous, and what
/// watches for it triggers (CR 701.37c). `by` puts the counters: the controller of the ability that does it.
pub fn monstrosity(cx: &mut Cx, id: ObjectId, n: i32, by: PlayerId) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) || cx.s().obj(id).marks.monstrous {
        return;
    }
    cx.st().update_obj(id, |o| o.marks.monstrous = true);
    counters::put(cx, id, CounterKind::PlusOne, n, by, counters::How::Effect);
    cx.emit(Event::Monstrous { permanent: id });
}

/// A permanent becomes saddled until end of turn (CR 702.171b).
pub fn saddle(cx: &mut Cx, id: ObjectId) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
        return;
    }
    let turn = cx.s().turn.number;
    cx.st().update_obj(id, |o| o.marks.saddled_turn = turn);
}

/// A permanent becomes solved (CR 719.3a): it is given the solved designation (CR 719.3b), which it keeps until it leaves
/// the battlefield, and what watches for a Case becoming solved triggers. An object that has left the battlefield is a new
/// object (CR 400.7) and becomes nothing; one already solved stays as it is, and nothing triggers.
pub fn solve(cx: &mut Cx, id: ObjectId) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) || cx.s().obj(id).marks.solved {
        return;
    }
    cx.st().update_obj(id, |o| o.marks.solved = true);
    cx.emit(Event::Solved { permanent: id });
}

/// Suspect a creature (CR 701.60a): a permanent on the battlefield becomes suspected until it leaves the battlefield, an
/// effect whose layer-6 grant is menace and "This creature can't block" (CR 701.60c). A suspected permanent can't become
/// suspected again (CR 701.60d), and nothing triggers then.
pub fn suspect(cx: &mut Cx, id: ObjectId) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) || Query::is_suspected(cx, id) {
        return;
    }
    Game::add_effect(cx, id, id, EffectKind::Suspected, Duration::Indefinite);
    cx.emit(Event::Suspected { permanent: id });
}

/// Two creatures fight (CR 701.14a): each deals damage equal to its power to the other, at once, as one event (CR 120.2;
/// not combat damage, CR 701.14d). A creature that fights itself deals damage to itself equal to twice its power (CR
/// 701.14c). Nothing happens unless both are creatures on the battlefield (CR 701.14b).
pub fn fight(cx: &mut Cx, first: ObjectId, second: ObjectId) {
    let fights = |cx: &Cx, id: ObjectId| {
        live(cx.s(), id)
            && cx.s().obj(id).zone.is(ZoneKind::Battlefield)
            && layers::chars(cx.cards(), cx.s(), &cx.g.log, id).is(CardType::Creature)
    };
    if !fights(cx, first) || !fights(cx, second) {
        return;
    }
    let power = |cx: &Cx, id: ObjectId| layers::chars(cx.cards(), cx.s(), &cx.g.log, id).power.unwrap_or(0).max(0) as u32;
    let (a, b) = (power(cx, first), power(cx, second));
    if first == second {
        damage(cx, first, Target::Object(first), a.saturating_mul(2), false);
        return;
    }
    damage_together(cx, &[(first, Target::Object(second), a), (second, Target::Object(first), b)]);
}

/// Creates tokens attacking, tapped when `tapped` (CR 508.4): they enter together attacking `defender` without
/// having been declared as attackers, so no "whenever attacks" ability sees them. Outside combat they are only tokens.
pub fn create_tokens_attacking(cx: &mut Cx, tokens: &[(CardId, PlayerId)], tapped: bool, defender: Target) -> R<Vec<ObjectId>> {
    create_tokens_batch_attacking(cx, tokens, tapped, None, &[], Some(defender))
}

/// Makes created tokens attack `defender` (CR 508.4), when there is a combat to attack in.
pub fn attack_with_tokens(cx: &mut Cx, created: &[ObjectId], defender: Target) {
    enter_attacking(cx, created, defender);
}

/// Puts a card onto the battlefield tapped and attacking `defender` (CR 508.4): ninjutsu's "put this card onto the
/// battlefield from your hand tapped and attacking". Outside combat it is only tapped. It enters with `counters` as well
/// (CR 122.6, 614.1c): Grim Reaper's finality counter.
pub fn put_onto_battlefield_attacking(cx: &mut Cx, object: ObjectId, controller: PlayerId, defender: Target, counters: &[(CounterKind, i32)]) -> R<ObjectId> {
    // Thousand-Faced Shadow and Fallen Shinobi use this ninjutsu path.
    // It is attacking before its arrival is observed (CR 603.6a, 508.4).
    let new = put_onto_battlefield_into(cx, Vec::new(), &[(object, controller, true)], ZoneReason::Other, counters, None, None, Some(defender), &[])?
        .first().copied().unwrap_or(object);
    Ok(new)
}

/// An attacking creature attacks `defender` from now on (`Game::set_attack_target`; Tuvok, Chief Tactical Officer's
/// "reselect which player or permanent that creature is attacking"), and its band with it (CR 702.22d).
pub fn set_attack_target(cx: &mut Cx, attacker: ObjectId, defender: Target) {
    // Its defending player once it attacks that (CR 508.5), and it must be something its controller could attack at all
    // (CR 508.4a): an opponent, or a planeswalker or battle of one.
    let Some(defending) = defending_player_of(cx.s(), defender) else { return };
    let Some(&current) = cx.s().attackers.iter().find(|a| a.id == attacker) else { return };
    let controlling = cx.s().obj(attacker).controller;
    if current.target == defender || !cx.could_enter_attacking(controlling, defender) {
        return;
    }
    let attacked_as = defender.object().map_or(TypeSet::NONE, |d| attackable_types(&cx.chars(d)));
    // All creatures in an attacking band attack the same player, planeswalker or battle (CR 702.22d), so reselecting a
    // member reselects the band.
    let band = current.band;
    let mut attackers = cx.s().attackers.clone();
    let mut assignments = Vec::new();
    for a in attackers.iter_mut() {
        if a.id == attacker || (band != 0 && a.band == band) {
            a.target = defender;
            a.defending = defending;
            a.attacked_as = attacked_as;
            assignments.push(Event::AttackAssigned { attacker: a.id, defender, defending });
        }
    }
    cx.st().set_attackers(attackers);
    for assignment in assignments { cx.record(assignment); }
}

/// Permanents just put onto the battlefield attacking `defender` become attacking creatures (CR 506.3, 508.4), not declared
/// as attackers, so no "whenever ... attacks" sees them; one that entered during the declare blockers step or later is
/// unblocked (CR 508.4d). Each one that the rules keep out of combat only enters: outside a combat with attackers declared
/// (`TurnState::attacked`), one that is not a creature or is also a battle (CR 506.3a, 506.3f), or is controlled by a
/// player other than the attacking player (CR 506.3b), and all of them when `defender` can no longer be attacked
/// (CR 506.3c, 508.4a: a player not in the game, a planeswalker or battle no longer on the battlefield, no longer one, or
/// no longer controlled or protected by a defending player).
pub fn enter_attacking(cx: &mut Cx, entered: &[ObjectId], defender: Target) {
    if !cx.s().turn.attacked {
        return;
    }
    let attacking = cx.s().turn.active;
    // Its defending player (CR 508.5): the player attacked, or the controller or protector of the permanent.
    let Some(defending) = defending_player_of(cx.s(), defender).filter(|_| cx.could_enter_attacking(attacking, defender)) else {
        return;
    };
    // What it is attacked as (CR 506.4e): what the permanent is now; nothing, for a player.
    let attacked_as = defender.object().map_or(TypeSet::NONE, |d| attackable_types(&cx.chars(d)));
    let mut attackers = cx.s().attackers.clone();
    let mut assignments = Vec::new();
    for &id in entered {
        if !eligible_to_enter_attacking(cx, id) {
            continue;
        }
        // One already in this combat as it arrived - a replacement made it attack (`ZoneChange::attacking`, CR 508.4), which
        // could only choose a defender - attacks what this instruction says instead: it names one, and the permanent is
        // not put onto the battlefield twice.
        match attackers.iter().position(|a| a.id == id) {
            Some(at) => {
                if attackers[at].target != defender {
                    attackers[at] = Attacker::new(id, defender, defending, attacked_as);
                    assignments.push(Event::AttackAssigned { attacker: id, defender, defending });
                }
            }
            None => {
                attackers.push(Attacker::new(id, defender, defending, attacked_as));
                assignments.push(Event::AttackAssigned { attacker: id, defender, defending });
            }
        }
    }
    if !assignments.is_empty() {
        cx.st().set_attackers(attackers);
        for assignment in assignments { cx.record(assignment); }
    }
}

/// `enter_attacking` for tokens created together with an entry each (`create_tokens_attacking_each`, CR 508.4). `places[i]`
/// is the entry `created[i]` is one of "those tokens" for (`replace::token_batch`), none for a token a replacement added of
/// its own. The first token made for an entry attacks its defender, as the effect says. A further one a replacement made
/// for it (Doubling Season) attacks the defender too when the effect names exactly one ("attacking that opponent"), and
/// otherwise what its controller chooses among what the entry allows ("that player or a planeswalker they control"). One
/// a replacement added (Quina's Frog) is made for no entry, so the effect specifies nothing it attacks: its controller
/// chooses among everything it could (`enter_attacking_by_choice`). Each choice is made as the token enters, before its
/// entry is matched (CR 603.6a).
fn enter_attacking_each(cx: &mut Cx, created: &[ObjectId], places: &[Option<usize>], entries: &[EntersAttacking]) {
    let mut first = vec![true; entries.len()];
    let mut by_defender: Vec<(Target, Vec<ObjectId>)> = Vec::new();
    let mut added: Vec<ObjectId> = Vec::new();
    for (&id, &place) in created.iter().zip(places) {
        let Some(entry) = place.and_then(|place| entries.get(place).map(|entry| (place, entry))) else {
            added.push(id);
            continue;
        };
        let (place, entry) = entry;
        let defender = if std::mem::replace(&mut first[place], false) || entry.others.is_empty() {
            entry.defender
        } else {
            if !eligible_to_enter_attacking(cx, id) {
                continue;
            }
            let controller = cx.s().obj(id).controller;
            let candidates: Vec<Target> = entry.others.iter().copied().filter(|&t| cx.could_enter_attacking(controller, t)).collect();
            match choose_entering_defender(cx, id, controller, candidates) {
                Some(defender) => defender,
                None => continue,
            }
        };
        match by_defender.iter_mut().find(|(t, _)| *t == defender) {
            Some((_, ids)) => ids.push(id),
            None => by_defender.push((defender, vec![id])),
        }
    }
    for (defender, ids) in by_defender {
        enter_attacking(cx, &ids, defender);
    }
    enter_attacking_by_choice(cx, &added);
}

/// What a permanent entering attacking attacks, chosen by its controller among `candidates` (CR 508.4): asked only when
/// there is a choice, and the first candidate when the question cannot be answered yet, the run going on and being
/// replayed with the answer (`Cx::ask_rule`, as the graveyard order is). None when there is nothing to attack.
fn choose_entering_defender(cx: &mut Cx, id: ObjectId, controller: PlayerId, candidates: Vec<Target>) -> Option<Target> {
    match candidates[..] {
        [] => None,
        [only] => Some(only),
        _ => {
            let first = candidates[0];
            let ask = RequestKind::ChooseTarget { replacing: None,
                label: "Choose what it attacks as it enters attacking: a player, a planeswalker or a battle.",
                slot_player: None,
                together: None,
                source: id,
                purpose: Purpose::EnterAttacking,
                candidates,
                optional: false,
                least: 1,
                count: 1,
                any_number: false,
            };
            match cx.ask_rule(controller, ask) {
                Ok(Some(Answer::Targets(t))) => Some(t.first().copied().unwrap_or(first)),
                _ => Some(first),
            }
        }
    }
}

/// CR 506.3a-b,f: only a creature controlled by the attacking player can join this combat.
fn eligible_to_enter_attacking(cx: &Cx, id: ObjectId) -> bool {
    if !cx.s().turn.attacked || !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield)
        || cx.s().obj(id).controller != cx.s().turn.active {
        return false;
    }
    let chars = cx.chars(id);
    chars.is(CardType::Creature) && !chars.is(CardType::Battle)
}

/// Permanents a replacement has arrive attacking (CR 614.1c, 508.4): its "instead it enters tapped and attacking" names no
/// defender, so each attacks what its controller chooses as it arrives (`Query::enter_attacking_targets`) - asked only when there is a choice, as the effect
/// that names one asks (`cantrip_mechanics::combat::choose_attack_target`), and the first candidate when the question cannot
/// be answered yet, the run going on and being replayed with the answer (`Cx::ask_rule`, as the graveyard order is). A
/// permanent the rules keep out of combat only enters (`enter_attacking`, CR 506.3).
fn enter_attacking_by_choice(cx: &mut Cx, entered: &[ObjectId]) {
    for &id in entered {
        if !eligible_to_enter_attacking(cx, id) {
            continue;
        }
        let controller = cx.s().obj(id).controller;
        let candidates = cx.enter_attacking_targets(controller);
        let Some(defender) = choose_entering_defender(cx, id, controller, candidates) else { continue };
        enter_attacking(cx, &[id], defender);
    }
}

/// Settles one phased-out permanent from layer 2's order (`layers::control_order`): settled only where the rules ask who
/// controls it though it is treated as not existing (CR 800.4a, 702.26n: a player leaving).
fn settle_controller(cx: &mut Cx, id: ObjectId) -> bool {
    let order = layers::control_order(cx.cards(), cx.s(), &cx.g.log);
    match order.into_iter().find(|&(of, _, _)| of == id) {
        Some((_, controller, by_effect)) => write_controller(cx, id, controller, by_effect),
        None => false,
    }
}

/// Writes what layer 2 made of a permanent's controller: `controller`, given by a control-changing effect or not. The
/// controller it had before any did is kept beside it (`Marks::control_base`). Whether its controller changed.
fn write_controller(cx: &mut Cx, id: ObjectId, controller: PlayerId, by_effect: bool) -> bool {
    let obj = *cx.s().obj(id);
    let base = obj.marks.control_base.unwrap_or(obj.controller);
    let control_base = by_effect.then_some(base);
    if controller == obj.controller && control_base == obj.marks.control_base {
        return false;
    }
    let turn = cx.s().turn.number;
    cx.st().update_obj(id, |o| {
        o.marks.control_base = control_base;
        if o.controller != controller {
            o.controller = controller;
            o.controlled_since = turn;
        }
    });
    if controller != obj.controller {
        // It came under this player's control (CR 613.1b): what an echo ability asks of the history (CR 702.30a).
        cx.record(Event::ControlChanged { permanent: id, from: obj.controller, controller });
        // A permanent whose controller changes is removed from combat (CR 506.4): an attacking or blocking creature, and a
        // planeswalker or battle being attacked, whose attackers then attack nothing (CR 506.4c).
        remove_from_combat(cx, &[id]);
        // "For as long as you control [it]" is over for good, though control come back (CR 611.2b).
        if cx.s().permissions.iter().any(|p| p.grant.expiry.controlled() == Some(id)) {
            cx.st().end_lapsed_permissions();
        }
    }
    controller != obj.controller
}

/// Whether the condition of an effect's "for as long as" duration (`Duration::AsLongAs`, CR 611.2b) is still true: the
/// card's own condition, asked of the effect. Any other duration holds as far as this goes.
pub fn lasting_holds(cx: &Cx, e: &cantrip_state::ContinuousEffect) -> bool {
    if !e.duration.holds_for(cx.s().obj(e.affected)) {
        return false;
    }
    match e.duration {
        Duration::AsLongAs(card, index) => cx.card(card).lasting.get(index as usize).is_none_or(|l| (l.holds)(cx, e)),
        Duration::WhileSourceTapped | Duration::WhileSourceTappedAndControlled | Duration::WhileSourceControlled => {
            live(cx.s(), e.source)
                && cx.s().obj(e.source).zone.is(ZoneKind::Battlefield)
                && (e.duration == Duration::WhileSourceControlled || cx.s().obj(e.source).tapped)
                && (e.duration == Duration::WhileSourceTapped || cx.s().obj(e.source).controller == e.controller)
        }
        _ => true,
    }
}

/// Whether this duration has a condition that can end without a zone change (CR 611.2b).
fn conditional(duration: Duration) -> bool {
    matches!(duration, Duration::AsLongAs(..) | Duration::WhileSourceTapped | Duration::WhileSourceTappedAndControlled | Duration::WhileSourceControlled | Duration::WhileTapped | Duration::WhileCounter(_))
}

/// The common source-dependent durations' uninterrupted interval (CR 611.2b). The event history is the authority for a
/// break before resolution; current tapped/control state alone cannot distinguish an untap/retap or loss/regain of control. A
/// complete condition may start again DURING this resolution: since it currently holds, its last affirmative
/// transition after its last lapse is exactly the beginning of the current full interval, including a conjunction.
/// Custom AsLongAs predicates are not reconstructed here: they still own their current-state condition only.
fn source_interval_begins(cx: &Cx, e: &cantrip_state::ContinuousEffect) -> bool {
    if !matches!(e.duration, Duration::WhileSourceTapped | Duration::WhileSourceTappedAndControlled | Duration::WhileSourceControlled) {
        return true;
    }
    // An immediate mana ability has its own declaration/cost/resolution boundaries. A nested activation must
    // never borrow the enclosing stack item's interval, or hide a lapse while paying its own costs (CR 605.3).
    let (since, tapped_since, resolution) = if cx.mana_source.is_some() {
        let Some(interval) = cx.g.mana_interval.as_ref().filter(|i| i.source == e.source) else { return true };
        (interval.since, interval.tapped_since, interval.resolution)
    } else {
        let Some(item) = cx.resolving().filter(|i| i.object == e.source) else { return true };
        (item.since, item.tapped_since, cx.s().agenda[cx.frame].log_mark)
    };
    // Core readers of the raw log must notify loop detection/layer caches just as Query::history does.
    cantrip_api::history::note_read();
    let tapped = e.duration != Duration::WhileSourceControlled;
    let controlled = e.duration != Duration::WhileSourceTapped;
    let mut lapsed = None;
    let mut began = None;
    for record in cx.g.log.iter().skip(since as usize) {
        let transition = match record.event {
            Event::Untapped { permanent } if permanent == e.source && tapped && record.serial >= tapped_since => Some(false),
            Event::Tapped { permanent } if permanent == e.source && tapped && record.serial >= tapped_since => Some(true),
            Event::ControlChanged { permanent, controller, .. } if permanent == e.source && controlled => Some(controller == e.controller),
            Event::PhasedOut { permanent, .. } if permanent == e.source && (controlled || record.serial >= tapped_since) => Some(false),
            Event::PhasedIn { permanent } if permanent == e.source && (controlled || record.serial >= tapped_since) => Some(true),
            Event::ZoneChange { old, from, .. } if old == e.source && from.is(ZoneKind::Battlefield) => Some(false),
            _ => None,
        };
        match transition {
            Some(false) => lapsed = Some(record.serial),
            Some(true) => began = Some(record.serial),
            None => {}
        }
    }
    lapsed.is_none_or(|end| began.is_some_and(|start| start > end && start >= resolution))
}

/// Whether an effect's duration begins as it would first apply (CR 611.2b). Source-dependent durations additionally ask
/// the history: an interval ended before resolution does not revive merely because its condition currently holds.
pub fn duration_begins(cx: &Cx, e: &cantrip_state::ContinuousEffect) -> bool {
    !matches!(e.duration, Duration::DuringControllersNextUntapSteps(0))
        && !source_departed(cx.s(), e) && lasting_holds(cx, e) && source_interval_begins(cx, e)
}

/// Whether any control-changing effect could apply, or have applied: the guard of `settle_control`, free in nearly every
/// position.
pub(crate) fn control_may_change(cx: &Cx) -> bool {
    let s = cx.s();
    let cards = cx.cards();
    s.effects.iter().any(|e| matches!(e.kind, EffectKind::Control(_)))
        || s.battlefield().iter().any(|&id| {
            let o = s.obj(id);
            o.marks.control_base.is_some() || (s.statics_seen && cards.texts(s, o).any(|c| cards.control_statics[c.0 as usize]))
        })
}

/// Removes `permanents` from combat (CR 506.4): an attacking creature among them stops attacking, a blocking one stops
/// blocking, and a planeswalker or battle among them being attacked stops being attacked - the creatures attacking it go on
/// attacking, attacking nothing (`Attacker::target` `Target::None`), and may still be blocked; unblocked, they assign no
/// combat damage (CR 506.4c, 510.1b). Their defending player stays who it was (CR 508.5, `Attacker::defending`). The one
/// place combat forgets a permanent: leaving the battlefield, phasing out, a change of controller or protector,
/// regeneration.
pub(crate) fn remove_from_combat(cx: &mut Cx, permanents: &[ObjectId]) {
    if permanents.is_empty() {
        return;
    }
    let removed = |id: ObjectId| permanents.contains(&id);
    let s = cx.s();
    let attacking = s.attackers.iter().any(|a| removed(a.id) || a.target.object().is_some_and(removed));
    if attacking {
        let attackers = s
            .attackers
            .iter()
            .filter(|a| !removed(a.id))
            .map(|&a| if a.target.object().is_some_and(removed) { Attacker { target: Target::None, attacked_as: TypeSet::NONE, ..a } } else { a })
            .collect();
        cx.st().set_attackers(attackers);
    }
    if cx.s().blocks.iter().any(|b| removed(b.blocker) || removed(b.attacker)) {
        let blocks = cx.s().blocks.iter().copied().filter(|b| !removed(b.blocker) && !removed(b.attacker)).collect();
        cx.st().set_blocks(blocks);
    }
    if cx.s().blockers.iter().any(|&id| removed(id)) {
        let blockers = cx.s().blockers.iter().copied().filter(|&id| !removed(id)).collect();
        cx.st().set_blockers(blockers);
    }
}

/// An effect makes an attacking creature blocked (CR 509.1h, 509.3c). No creature becomes a blocker, so this
/// emits the transition alone, not a per-blocker event. Dazzling Beauty and Curtain of Light print it.
pub(crate) fn set_blocked(cx: &mut Cx, attacker: ObjectId, blocked: bool) {
    if !cx.s().attackers.iter().any(|a| a.id == attacker && a.blocked != blocked) {
        return;
    }
    let mut attackers = cx.s().attackers.clone();
    for a in &mut attackers {
        if a.id == attacker { a.blocked = blocked; }
    }
    cx.st().set_attackers(attackers);
    if blocked {
        cx.emit(Event::BecameBlocked { attacker });
    } else {
        // Not the declare-blockers event: "attacks and isn't blocked" does not trigger (CR 509.3g).
        cx.state_changed();
    }
}

/// The card types a permanent can be attacked as that it has (CR 506.3): planeswalker, battle, both or neither.
pub(crate) fn attackable_types(chars: &Chars) -> TypeSet {
    TypeSet(chars.types.0 & TypeSet::of(&[CardType::Planeswalker, CardType::Battle]).0)
}

/// What a permanent being attacked as `was` (`Attacker::attacked_as`) is being attacked as now, its card types as the layers
/// have them now (`attackable_types`); none once the rules remove it from combat by its types (CR 506.4, 506.4e). Its
/// controller and protector are the ones it had (a change removes it, `remove_from_combat`).
/// - It stays as long as it keeps what it was attacked as, and a planeswalker or battle that becomes both is being attacked
///   as both (nothing removes it).
/// - One that was both: it is removed once it is neither; a planeswalker and no longer a battle stays only while it is
///   controlled by its protector - a protector it no longer has controls nothing (CR 310.9g keeps it one) - and a battle and
///   no longer a planeswalker stays (CR 506.4e).
/// - One that was only a planeswalker and is now only a battle, or only a battle and is now only a planeswalker, has
///   stopped being what it was attacked as, and is removed (CR 506.4); it was never both, so CR 506.4e is not about it.
fn attacked_as_now(cx: &Cx, id: ObjectId, was: TypeSet) -> Option<TypeSet> {
    let now = attackable_types(&cx.chars(id));
    let both = TypeSet::of(&[CardType::Planeswalker, CardType::Battle]);
    let stays = if now == TypeSet::NONE {
        false
    } else if now == was || now == both {
        true
    } else if was == both {
        // CR 506.4e.
        let obj = cx.s().obj(id);
        !now.has(CardType::Planeswalker) || obj.marks.protector == Some(obj.controller)
    } else {
        false
    };
    stays.then_some(now)
}

/// Combat as the card types the layers give its permanents now say (CR 506.4), settled the moment they change, for good:
/// - an attacking or blocking creature that has stopped being a creature is removed from combat - it stops attacking or
///   blocking (and the blocks it was in end), though it become a creature again before combat damage;
/// - a planeswalker or battle being attacked that has stopped being what it is attacked as is removed from combat (CR
///   506.4e): the creatures attacking it attack nothing from then on (CR 506.4c), though it become one again. One that
///   stays has what it is now attacked as recorded (`attacked_as_now`).
///
/// The two are apart (CR 506.4d): a blocking creature that is also a planeswalker being attacked and stops being a
/// creature is still attacked, and one that stops being a planeswalker is still blocking. Asked wherever characteristics
/// may have changed - at every event and every effect begun (`settle_lasting_and_control`), and at every change with no
/// event (`Cx::state_changed`). Free while nothing attacks.
pub(crate) fn settle_combat(cx: &mut Cx) {
    let s = cx.s();
    if s.attackers.is_empty() && s.blockers.is_empty() {
        return;
    }
    let creature = |id: ObjectId| cx.chars(id).is(CardType::Creature);
    let mut changed = false;
    let mut attackers: Vec<Attacker> = Vec::with_capacity(s.attackers.len());
    for &a in &s.attackers {
        if !creature(a.id) {
            changed = true;
            continue;
        }
        let next = match a.target {
            Target::Object(id) => match attacked_as_now(cx, id, a.attacked_as) {
                Some(now) => Attacker { attacked_as: now, ..a },
                None => Attacker { target: Target::None, attacked_as: TypeSet::NONE, ..a },
            },
            _ => a,
        };
        changed |= next != a;
        attackers.push(next);
    }
    let kept = |b: &&Block| attackers.iter().any(|a| a.id == b.attacker) && creature(b.blocker);
    let blocks: Option<Vec<Block>> = (s.blocks.iter().filter(kept).count() != s.blocks.len()).then(|| s.blocks.iter().filter(kept).copied().collect());
    let blockers: Option<Vec<ObjectId>> = s.blockers.iter().any(|&id| !creature(id)).then(|| s.blockers.iter().copied().filter(|&id| creature(id)).collect());
    if changed {
        cx.st().set_attackers(attackers);
    }
    if let Some(blocks) = blocks {
        cx.st().set_blocks(blocks);
    }
    if let Some(blockers) = blockers {
        cx.st().set_blockers(blockers);
    }
}

/// Settles at once what continuous effects say of control (CR 613.1b) and of "for as long as" durations (CR 611.2b), as
/// each event happens rather than only as the state-based actions are next checked: an Aura that gave control leaving or
/// moving, a condition turning false partway through a resolution. Before an event's triggers are looked for (`Cx::emit`),
/// so an ability triggering on it, and whatever the resolution reads next, sees the controller the effects now give.
pub fn settle_at_once(cx: &mut Cx) {
    if cx.s().settle_held.get() > 0 {
        return;
    }
    settle_lasting_and_control(cx);
}

/// Ends the "for as long as" effects whose condition is false (CR 611.2b) and settles control (CR 613.1b), again and again
/// while either changes anything: a change of control can end an effect "for as long as you control [it]", whose end
/// changes control again. It ends, as each round but the last ends an effect or changes nothing. Whether anything changed.
/// The effects "for as long as [their source] remains on the battlefield" whose source has gone end first: no change of
/// control moves a permanent, so they end once.
pub fn settle_lasting_and_control(cx: &mut Cx) -> bool {
    let mut changed = end_departed_sources(cx);
    // Each round that goes on ended an effect; there are no more rounds than effects, and one to see nothing more changes.
    for _ in 0..=cx.s().effects.len() {
        let lasting = cx.s().effects.iter().any(|e| conditional(e.duration));
        let ended = lasting && end_lapsed_conditions(cx);
        let moved = control_may_change(cx) && settle_control(cx);
        changed |= ended || moved;
        if !ended && !(moved && lasting) {
            break;
        }
    }
    settle_combat(cx);
    crate::world::sync(cx);
    crate::hand_visibility::sync(cx);
    changed
}

/// Runs `f` as one instruction (CR 608.2c): what continuous effects say of control and "for as long as" durations is
/// settled once it has finished, not between its parts - the two halves of an exchange of control (CR 701.12a) leave no
/// moment in which a player controls neither permanent.
pub fn as_one_instruction<T>(cx: &mut Cx, f: impl FnOnce(&mut Cx) -> T) -> T {
    let held = cx.s().settle_held.get();
    cx.s().settle_held.set(held.saturating_add(1));
    // Lowered again however the instruction ends: a host that catches a panic goes on with the game.
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut *cx)));
    cx.s().settle_held.set(held);
    let done = match done {
        Ok(done) => done,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    if held == 0 {
        settle_at_once(cx);
        crate::library_visibility::sync(cx);
    }
    done
}

/// Whether an effect's "for as long as [its source] remains on the battlefield" duration (`Duration::WhileSourceOnBattlefield`,
/// CR 611.2b) is over: its source is no longer the permanent it was (CR 400.7), or is phased out, which the duration can no
/// longer see (CR 702.26f). Any other duration is not, as far as this goes.
pub fn source_departed(s: &cantrip_state::GameState, e: &cantrip_state::ContinuousEffect) -> bool {
    e.duration == Duration::WhileSourceOnBattlefield && !(live(s, e.source) && s.obj(e.source).zone.is(ZoneKind::Battlefield))
}

/// Ends each effect "for as long as [its source] remains on the battlefield" whose source has left it or phased out (CR
/// 611.2b, 702.26f), the moment it has, as each event happens (`settle_at_once`): not only as the state-based actions are
/// next checked, which no cost being paid (CR 601.2h) or resolution (CR 608.2) waits for. A control-changing effect that
/// ends gives its permanent back with the next `settle_control`. Whether any ended.
pub fn end_departed_sources(cx: &mut Cx) -> bool {
    let s = cx.s();
    if !s.effects.iter().any(|e| source_departed(s, e)) {
        return false;
    }
    let keep: Vec<bool> = s.effects.iter().map(|e| !source_departed(s, e)).collect();
    cx.st().effects_keep(&keep);
    // An effect that ends changes the game with no event (CR 603.8).
    cx.state_changed();
    true
}

/// Ends each effect whose "for as long as [condition]" duration (`Duration::AsLongAs`) is no longer true (CR 611.2b), as
/// each event happens and as the state-based actions are checked (`settle_at_once`): it is over for good, though the
/// condition become true again. A control-changing effect that ends gives its permanent back with the next
/// `settle_control`. Whether any ended.
pub fn end_lapsed_conditions(cx: &mut Cx) -> bool {
    if !cx.s().effects.iter().any(|e| conditional(e.duration)) {
        return false;
    }
    let keep: Vec<bool> = cx.s().effects.iter().map(|e| lasting_holds(cx, e)).collect();
    if keep.iter().all(|&k| k) {
        return false;
    }
    let effects_before = cx.s().effects.len();
    cx.st().effects_keep(&keep);
    // An effect that ends changes the game with no event (CR 603.8).
    if cx.s().effects.len() < effects_before {
        cx.state_changed();
    }
    true
}

/// Settles the controller of every permanent (CR 613.1b, layer 2) as layer 2's order says (`layers::control_order`): the
/// control-changing effects in timestamp order unless one depends on another (CR 613.8) - one permanent's controller can
/// decide whom another's effect gives control to (an Aura's controller), and that is a dependency, not a repetition. Whether
/// any controller changed. Free while no control-changing effect applies and no permanent's control was ever changed.
pub fn settle_control(cx: &mut Cx) -> bool {
    if !control_may_change(cx) {
        return false;
    }
    let order = layers::control_order(cx.cards(), cx.s(), &cx.g.log);
    let start = cx.g.log.len();
    let mut changed = false;
    for (id, controller, by_effect) in order {
        if cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
            changed |= write_controller(cx, id, controller, by_effect);
        }
    }
    if changed {
        // The changes of control layer 2 made at once happened together (CR 603.2c): what they trigger ("when you lose
        // control of [it]", CR 603.2) is looked for once every permanent's controller is settled.
        let events: Vec<Event> =
            cx.g.log[start..].iter().map(|r| r.event).filter(|e| matches!(e, Event::ControlChanged { .. })).collect();
        crate::settle::collect_batch(cx, &events, None, start);
    }
    changed
}

/// A permanent becomes a copy of something for `duration` (CR 707.2, 611.2b): its copiable values are `values` (exceptions
/// the effect states included) from now on, and when the effect ends - however it ends - they are what the copy effects
/// still on it make them, in timestamp order (CR 613.2a, 613.7; `GameState::effect_remove` undoes it). It stays the same
/// object, with its counters, damage, status and attachments (CR 707.2). Nothing if it is not on the battlefield, and
/// nothing if its duration is already over as it would begin (CR 611.2b), as for any other continuous effect.
pub fn become_copy(cx: &mut Cx, source: ObjectId, id: ObjectId, values: Copiable, duration: Duration) {
    if !live(cx.s(), id) || !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
        return;
    }
    become_copy_of(cx, source, id, values, duration);
    // Its characteristics changed with no event (CR 707.2, 603.8).
    cx.state_changed();
}

fn become_copy_of(cx: &mut Cx, source: ObjectId, id: ObjectId, values: Copiable, duration: Duration) {
    let prior = cx.s().obj(id).copy;
    let controller = cx.s().obj(source).controller;
    let timestamp = cx.st().timestamp();
    let effect = cantrip_state::ContinuousEffect {
        source,
        controller,
        timestamp,
        affected: id,
        kind: EffectKind::RestoreCopy(prior),
        duration,
        words: Words::NONE,
    };
    if !duration_begins(cx, &effect) {
        return;
    }
    cx.st().update_obj(id, |o| o.copy = Some(values));
    cx.st().effect_push(effect);
    note_statics(cx, id);
}

/// An effect states that a player loses the game (CR 104.3e): they leave it at once, and if one player is left, that player
/// has won (CR 104.2a).
/// A player who "can't lose the game" (Platinum Angel, CR 104.3) does not.
pub fn lose_game(cx: &mut Cx, player: PlayerId) {
    if cx.s().player(player).lost || cx.s().is_over() || crate::static_rules::doesnt_lose(&*cx, cx.cards(), player, LossReason::Effect) {
        return;
    }
    leaves_having_lost(cx, player, LossReason::Effect);
}

/// A player concedes (CR 104.3a): they lose and leave the game at once, which nothing prevents ("can't lose the game" does not
/// stop it); the monarch and the initiative pass on (CR 725.4, 726.4) and their objects leave (CR 800.4a).
/// Conceding is a player's action (CR 104.3a), so it breaks the run of passes: every player still in passes again, with what
/// the concession caused, before the top of the stack resolves or the step ends (CR 117.4; the owner's ruling, 2026-11-05).
pub fn concede(cx: &mut Cx, player: PlayerId) {
    if cx.s().player(player).lost || cx.s().is_over() {
        return;
    }
    cx.st().update_turn(|t| t.passes = 0);
    leaves_having_lost(cx, player, LossReason::Conceded);
}

/// `player` has lost for `reason` and leaves the game; the game ends if one or no player is left (CR 104.2a, 104.4a).
fn leaves_having_lost(cx: &mut Cx, player: PlayerId, reason: LossReason) {
    // The board just before the loss, which an ability that triggers on it looks back on (CR 603.10f).
    let (battlefield, then) = (cx.s().battlefield().to_vec(), crate::settle::Then::now(cx));
    cx.st().update_player(player, |pl| pl.lost = true);
    crate::settle::lost_the_game(cx, &[(player, reason)], &battlefield, &then);
    let left: Vec<PlayerId> = cx.s().apnap().collect();
    match left.len() {
        0 => {
            end_game(cx, Outcome::Draw);
            cx.record(Event::GameDrawn { reason: DrawReason::AllLost });
        }
        1 => end_game(cx, Outcome::Won(left[0])),
        _ => {}
    }
}

/// The game ends with `outcome` (CR 104): every way it ends comes here. At its end every face-down permanent and face-down
/// spell is revealed to all players by its owner (CR 708.9), recorded before the end is, so the history and every seat
/// have it; a seat may look at each from then on (`visibility::knows_face`, the game being over).
pub fn end_game(cx: &mut Cx, outcome: Outcome) {
    if cx.s().is_over() {
        return;
    }
    for (player, card) in face_down_to_reveal(cx, |_| true) {
        cx.record(Event::Revealed { player, card });
    }
    cx.st().set_outcome(outcome);
}

/// The face-down permanents (phased out too) and face-down spells (CR 708.9) that `which` takes, each with its owner, who
/// reveals it.
fn face_down_to_reveal(cx: &Cx, which: impl Fn(&Obj) -> bool) -> Vec<(PlayerId, ObjectId)> {
    let s = cx.s();
    [Zone::BATTLEFIELD, Zone::PHASED_OUT, Zone::STACK]
        .into_iter()
        .flat_map(|zone| s.zone(zone).iter().copied())
        .filter(|&id| s.obj(id).face_down && which(s.obj(id)))
        .map(|id| (s.obj(id).owner, id))
        .collect()
}

/// A player gets poison counters (CR 122.1f), put on them by `by` (CR 122.6a); ten lose the game as a state-based action
/// (CR 704.5c).
pub fn give_poison(cx: &mut Cx, player: PlayerId, amount: u32, by: PlayerId) {
    counters::put_on_player(cx, player, counters::PlayerCounts { poison: amount.min(u16::MAX as u32) as u16, ..Default::default() }, by);
}

/// The way `player` loses the game to the next state-based action check (CR 704.5a-c, 704.6c), if they do: 0 or less life,
/// a draw attempted from an empty library since the last check, ten or more poison counters, 21 or more combat damage from
/// one commander, in that order, leaving out each that a static ability says they don't lose for (CR 104.3: Platinum
/// Angel, Phyrexian Unlife). The check itself reads the commander damage once for every player
/// (`loses_to_state_based_actions_given`).
#[cfg(test)]
pub fn loses_to_state_based_actions(cx: &Cx, player: PlayerId) -> Option<LossReason> {
    loses_to_state_based_actions_given(cx, player, &dealt_21_by_a_commander(cx))
}

/// The players dealt 21 or more combat damage by one commander (CR 903.10a), for `loses_to_state_based_actions_given`: none
/// outside a Commander game, where the history is not read for it.
pub fn dealt_21_by_a_commander(cx: &Cx) -> Vec<PlayerId> {
    if cx.s().commander { cx.history().players_dealt_21_by_a_commander(cx.s()) } else { Vec::new() }
}

/// `loses_to_state_based_actions`, with the players `dealt_21_by_a_commander` read once for a whole check.
pub fn loses_to_state_based_actions_given(cx: &Cx, player: PlayerId, commander_beaten: &[PlayerId]) -> Option<LossReason> {
    let p = cx.s().player(player);
    let commander_damage = commander_beaten.contains(&player);
    [
        (p.life <= 0, LossReason::Life),
        (p.drew_from_empty, LossReason::EmptyLibrary),
        (p.poison() >= 10, LossReason::Poison),
        (commander_damage, LossReason::CommanderDamage),
    ]
        .into_iter()
        .find(|&(applies, reason)| applies && !crate::static_rules::doesnt_lose(cx, cx.cards(), player, reason))
        .map(|(_, reason)| reason)
}

/// An effect states that a player wins the game (CR 104.2b): every opponent has lost, which ends the game with them
/// the winner (CR 104.2a). A player who has lost wins nothing. Nor is one who is about to lose to a state-based action
/// (CR 704.5a-c) turned away: the win happens as the effect resolves, and those actions are performed only at the next
/// check, which the end of the game never reaches - the rule that a player who would win and lose simultaneously loses
/// (CR 104.3f) is about a win and a loss that happen at once, not one that is still waiting (Fblthp, Impossibly Lost's
/// ruling). A player who can't win the game (Platinum Angel's "your opponents can't win the game") does not, and nor does
/// one with an opponent who can't lose it: winning is every opponent losing (CR 104.2b, 104.3e).
pub fn win_game(cx: &mut Cx, player: PlayerId) {
    if cx.s().player(player).lost || cx.s().is_over() {
        return;
    }
    let opponents: Vec<PlayerId> = cx.s().opponents(player).collect();
    if crate::static_rules::cant_win(&*cx, cx.cards(), player)
        || opponents.iter().any(|&o| crate::static_rules::doesnt_lose(&*cx, cx.cards(), o, LossReason::Effect))
    {
        return;
    }
    for &o in &opponents {
        cx.st().update_player(o, |pl| pl.lost = true);
    }
    // Every opponent has lost before the designations pass, so none of them is passed one on the way (CR 725.4, 726.4).
    crate::designations::monarch_left(cx);
    crate::designations::initiative_left(cx);
    for o in opponents {
        cx.record(Event::GameLost { player: o, reason: LossReason::Effect });
    }
    end_game(cx, Outcome::Won(player));
}

/// Records each player whose controller is no longer the one in `before`, which holds each player's (CR 723.1): what a
/// seat sees follows who it controls (CR 723.4), and the history is where who saw which hidden card is asked.
pub(crate) fn record_player_control(cx: &mut Cx, before: &[PlayerId]) {
    for (i, &was) in before.iter().enumerate() {
        let player = PlayerId(i as u8);
        let controller = cx.s().player_controller(player);
        if controller != was {
            cx.record(Event::PlayerControlled { player, controller });
        }
    }
}

/// A player has left a game that goes on without them (CR 800.4a): every object they own leaves the game, together, and the
/// effects that gave them control of anything end; then the objects on the stack they control that no card represents
/// cease to exist; then what they still control is exiled. The last player standing wins at once instead (CR 104.2a), and
/// there is nothing left to clear.
pub fn leave_game(cx: &mut Cx, player: PlayerId) {
    if cx.s().apnap().count() < 2 {
        return;
    }
    // Their face-down permanents and spells are revealed to all players as they leave (CR 708.9), each of whom may look at
    // it from then on, though it has ceased to exist (`visibility::knows_face`).
    let revealed = face_down_to_reveal(cx, |o| o.owner == player);
    if !revealed.is_empty() {
        let everyone = cx.s().seats().fold(0u8, |mask, p| mask | 1 << p.0);
        for &(_, card) in &revealed {
            cx.st().update_obj(card, |o| o.seen_by = everyone);
        }
        let events: Vec<Event> = revealed.into_iter().map(|(player, card)| Event::Revealed { player, card }).collect();
        cx.emit_batch(&events, None);
    }
    // Everything they own, wherever it is, leaves at once: each as it was before any of them left (`cease_together`).
    let owned: Vec<(Zone, ObjectId)> = cx
        .s()
        .zones
        .iter()
        .flatten()
        .copied()
        .filter(|&id| cx.s().obj(id).owner == player)
        .map(|id| (cx.s().obj(id).zone, id))
        .collect();
    for at in (0..cx.s().stack.len()).rev() {
        let item = cx.s().stack[at];
        if item.kind == StackKind::Spell && cx.s().obj(item.object).owner == player {
            cx.st().stack_remove(at);
        }
    }
    cease_together(cx, &owned);
    // The effects that gave them control of objects or of players end (CR 800.4a, 723), and control is settled again (CR
    // 613.1b). One still waiting for a turn ends as it would apply (`turn::take_controlled_turn`), as what else it does
    // (an extra turn for the player it was about) is not the leaver's.
    let gave = |e: &ContinuousEffect| match e.kind {
        EffectKind::Control(p) => p == player,
        EffectKind::ControlsPlayer(_) => e.controller == player,
        _ => false,
    };
    if cx.s().effects.iter().any(gave) {
        let controllers: Vec<PlayerId> = (0..cx.s().players.len()).map(|i| cx.s().player_controller_before_leaving(PlayerId(i as u8), player)).collect();
        let effects_before = cx.s().effects.len();
        cx.st().effects_retain(|e| !gave(e));
        // An effect that ends changes the game with no event (CR 603.8).
        if cx.s().effects.len() < effects_before {
            cx.state_changed();
        }
        record_player_control(cx, &controllers);
    }
    settle_control(cx);
    // Then the objects on the stack they control that no card represents (an ability, a copy of a spell) cease to exist,
    // and a spell they control that another owns is exiled with their other objects below.
    let mut exiled: Vec<ObjectId> = Vec::new();
    for at in (0..cx.s().stack.len()).rev() {
        let item = cx.s().stack[at];
        if item.controller != player {
            continue;
        }
        cx.st().stack_remove(at);
        if item.kind != StackKind::Spell {
            continue;
        }
        if cx.s().obj(item.object).card_copy {
            cease_to_exist(cx, Zone::STACK, item.object);
        } else {
            exiled.push(item.object);
        }
    }
    // Triggers they control that have not yet been put on the stack have nothing to go on it.
    for at in (0..cx.s().triggers.len()).rev() {
        if cx.s().triggers[at].controller == player {
            cx.st().trigger_remove(at);
        }
    }
    for at in (0..cx.s().delayed.len()).rev() {
        if cx.s().delayed[at].controller == player {
            cx.st().delayed_remove(at);
        }
    }
    let gone = |id: ObjectId| cx.s().obj(id).zone.is(ZoneKind::Gone);
    let in_combat = cx.s().attackers.iter().flat_map(|a| [a.id].into_iter().chain(a.target.object()));
    let departed: Vec<ObjectId> = in_combat.chain(cx.s().blockers.iter().copied()).filter(|&id| gone(id)).collect();
    remove_from_combat(cx, &departed);
    // What they still control is exiled (CR 800.4a), a phased-out permanent included (CR 702.26n) once the effects that
    // gave them control of it have ended; one that goes back to another player phases in later (`phasing::untap_step`).
    let phased: Vec<ObjectId> = cx.s().zone(Zone::PHASED_OUT).iter().copied().filter(|&id| cx.s().obj(id).controller == player).collect();
    for &id in &phased {
        settle_controller(cx, id);
    }
    exiled.extend(cx.s().battlefield().iter().copied().filter(|&id| cx.s().obj(id).controller == player));
    exiled.extend(phased.into_iter().filter(|&id| cx.s().obj(id).controller == player));
    let moves: Vec<Moving> = exiled.into_iter().map(|id| Moving::to(id, Zone::exile(cx.s().obj(id).owner))).collect();
    if !moves.is_empty() {
        move_together(cx, &moves, ZoneReason::Exile);
    }
    // Their permanents that left with them leave what they exiled or phased out "until" they left to come back
    // (CR 610.3, 610.4).
    return_until_left(cx);
}

#[cfg(test)]
#[path = "departed_damage_tests.rs"]
mod departed_damage_tests;

#[cfg(test)]
#[path = "shield_total_tests.rs"]
mod shield_total_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Game, Setup};

    fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.is(CardType::Creature)
    }

    fn two_counters(_: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
        entering.counters[CounterKind::PlusOne as usize] += 2;
        Ok(())
    }

    /// Humility's first half: "All creatures lose all abilities."
    static HUMBLER: CardDef = CardDef {
        name: "Humbler",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[loses_all_abilities(creature, "")],
        ..CardDef::BLANK
    };
    /// "This creature enters with two +1/+1 counters on it."
    static GROWN: CardDef = CardDef {
        name: "Grown",
        types: TypeSet::of(&[CardType::Creature]),
        power: Some(0),
        toughness: Some(0),
        enters: Some(two_counters),
        ..CardDef::BLANK
    };
    /// The same of an artifact, which "all creatures" does not reach.
    static GROWN_RELIC: CardDef = CardDef { name: "Grown Relic", types: TypeSet::of(&[CardType::Artifact]), enters: Some(two_counters), ..CardDef::BLANK };

    fn artifact(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
        c.is(CardType::Artifact) && !c.is(CardType::Creature)
    }

    fn add_creature(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
        c.types = c.types.with(CardType::Creature);
    }

    /// March of the Machines' first half: "Each noncreature artifact is an artifact creature."
    static ANIMATOR: CardDef = CardDef {
        name: "Animator",
        types: TypeSet::of(&[CardType::Enchantment]),
        statics: &[static_ability(Layer::Type, artifact, add_creature, "")],
        ..CardDef::BLANK
    };

    fn entering(id: ObjectId) -> Entering {
        Entering {
            object: id,
            controller: PlayerId(0),
            tapped: false,
            counters: [0; CounterKind::COUNT],
            copy: None,
            prepared: false,
            chosen_color: None,
            chosen_subtype: None,
            chosen_player: None,
            chosen_card: None,
            chosen_option: None,
            chosen_number: None,
            chosen_object: None, emblem: None,
            attach: None,
            attach_player: None,
            x: 0,
            transformed: false,
            protector: None, read_ahead: None, riot_haste: false,
            instead: None,
            becomes: None,
            own: None,
            melded: None,
            counters_if: None,
        }
    }

    /// A game with Bot1's hand holding `on_battlefield`'s cards, put onto the battlefield in that order, and then Grown and
    /// Grown Relic, left in the hand (returned).
    fn game_with(on_battlefield: &[&'static CardDef]) -> (Game, Vec<ObjectId>) {
        let cards: &'static CardTable = Box::leak(Box::new(CardTable::new([&HUMBLER, &ANIMATOR, &GROWN, &GROWN_RELIC, &BAN])));
        let mut deck: Vec<CardId> = on_battlefield.iter().map(|&d| cards.id_of(d)).collect();
        deck.extend([cards.id_of(&GROWN), cards.id_of(&GROWN_RELIC)]);
        let mut game = Game::new(cards, &Setup { decks: vec![deck, Vec::new()], ..Setup::default() });
        let hand = Zone::hand(PlayerId(0));
        let held: Vec<ObjectId> = game.state.zone(hand).to_vec();
        for &id in &held[..on_battlefield.len()] {
            let timestamp = game.state.timestamp();
            game.state.zone_remove(hand, id);
            game.state.update_obj(id, |o| {
                o.zone = Zone::BATTLEFIELD;
                o.timestamp = timestamp;
            });
            game.state.zone_push(Zone::BATTLEFIELD, id);
            game.state.see_statics();
        }
        (game, held[on_battlefield.len()..].to_vec())
    }

    /// The counters each of Grown and Grown Relic would enter with, `on_battlefield` there first; each card is still in the
    /// hand afterwards.
    fn entering_counters(on_battlefield: &[&'static CardDef]) -> Vec<(&'static str, i32)> {
        let (mut game, held) = game_with(on_battlefield);
        let hand = Zone::hand(PlayerId(0));
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        let mut out = Vec::new();
        for &id in &held {
            let mut entering = entering(id);
            assert!(prepare_entry(&mut cx, &mut entering).is_ok());
            // Its own replacements left to apply (`Entering::own`), applied as they would be with nothing else to order.
            apply_own_entering(&mut cx, &mut entering, PlayerId(0), false);
            assert!(cx.s().obj(id).zone == hand && cx.s().zone(hand).contains(&id) && !cx.s().battlefield().contains(&id));
            out.push((cx.cards().get(cx.s().obj(id).card).name, entering.counters[CounterKind::PlusOne as usize]));
        }
        out
    }

    /// CR 614.12: a creature entering while "all creatures lose all abilities" applies has, as it would exist on the
    /// battlefield, no "enters with" ability, so it gets no counters; an artifact the ability does not reach gets its two, and
    /// so does the creature with nothing on the battlefield. The question leaves both cards where they were.
    #[test]
    fn an_entering_permanent_that_would_lose_its_abilities_has_no_enters_replacement() {
        assert_eq!(entering_counters(&[]), [("Grown", 2), ("Grown Relic", 2)]);
        assert_eq!(entering_counters(&[&HUMBLER]), [("Grown", 0), ("Grown Relic", 2)]);
    }

    /// CR 614.12: "continuous effects that already exist and would apply to the permanent" are those of every layer, other
    /// objects' type-changing ones included: an artifact that "each noncreature artifact is an artifact creature" would make a
    /// creature there loses its "enters with" to "all creatures lose all abilities" (Everflowing Chalice under March of the
    /// Machines and Humility), whichever of the two came first; the creature card does as before.
    #[test]
    fn an_entering_artifact_another_effect_would_make_a_creature_is_judged_as_one() {
        assert_eq!(entering_counters(&[&ANIMATOR]), [("Grown", 2), ("Grown Relic", 2)]);
        assert_eq!(entering_counters(&[&ANIMATOR, &HUMBLER]), [("Grown", 0), ("Grown Relic", 0)]);
        assert_eq!(entering_counters(&[&HUMBLER, &ANIMATOR]), [("Grown", 0), ("Grown Relic", 0)]);
    }

    /// `as_it_would_exist` reads the entering object through every layer as it would be on the battlefield, and leaves it,
    /// its zone and the battlefield as they were; with the answer on `Cx::entering`, `Query::chars` and
    /// `Query::lost_all_abilities` answer so of that object and of no other.
    #[test]
    fn the_object_about_to_enter_is_answered_as_it_would_exist_and_left_where_it_is() {
        let (mut game, held) = game_with(&[&ANIMATOR, &HUMBLER]);
        let relic = held[1];
        let row = *game.state.obj(relic);
        let battlefield = game.state.battlefield().to_vec();
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        let would = as_it_would_exist(&mut cx, &entering(relic));
        assert!(would.chars.is(CardType::Creature) && would.chars.is(CardType::Artifact) && would.lost);
        assert!(*cx.s().obj(relic) == row && cx.s().battlefield() == battlefield.as_slice());
        assert!(!cx.chars(relic).is(CardType::Creature) && !cx.lost_all_abilities(relic));
        cx.entering = Some(would);
        assert!(cx.chars(relic).is(CardType::Creature) && cx.lost_all_abilities(relic));
        assert!(!cx.chars(held[0]).is(CardType::Artifact));
    }

    /// CR 608.2h, 400.7: a permanent's name is part of its last-known information. Animator, named Everflame by an effect
    /// (CR 612.8, The Irencrag's), is Everflame as it last was once it has gone to the graveyard, where the card it now is
    /// has its own name.
    #[test]
    fn a_name_an_effect_set_is_the_last_known_name_of_a_permanent_that_left() {
        let (mut game, _) = game_with(&[&ANIMATOR]);
        let animator = game.state.battlefield()[0];
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        cantrip_api::Game::add_effect(&mut cx, animator, animator, EffectKind::SetName("Everflame"), Duration::Indefinite);
        assert_eq!(cx.chars(animator).name, "Everflame");
        let card = move_to(&mut cx, animator, Zone::graveyard(PlayerId(0)));
        assert_eq!(cx.chars(animator).name, "Everflame");
        assert!(cx.chars(animator).name_set);
        assert_eq!(cx.chars(card).name, "Animator");
    }

    /// Melira's "creatures you control can't have -1/-1 counters put on them" (CR 122.6, 101.2).
    fn no_minus_counters_on_your_creatures(q: &dyn Query, asker: &Asker, object: ObjectId, controller: PlayerId, kind: CounterKind) -> bool {
        kind == CounterKind::MinusOne && controller == asker.controller && q.chars(object).is(CardType::Creature)
    }

    static BAN: CardDef = CardDef {
        name: "Minus Ban",
        types: TypeSet::of(&[CardType::Creature]),
        power: Some(2),
        toughness: Some(2),
        counter_bans: &[no_minus_counters_on_your_creatures],
        ..CardDef::BLANK
    };

    /// The -1/-1 counters each of Grown and Grown Relic has, put onto the battlefield from the hand "with a -1/-1 counter on
    /// it" (persist's words, CR 702.79a), `on_battlefield` there first.
    fn minus_counters(on_battlefield: &[&'static CardDef]) -> Vec<(&'static str, i32)> {
        let (mut game, held) = game_with(on_battlefield);
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        let mut out = Vec::new();
        for &id in &held {
            let new = put_onto_battlefield_with(&mut cx, &[(id, PlayerId(0), false)], ZoneReason::Other, &[(CounterKind::MinusOne, 1)]).expect("nothing is asked")[0];
            out.push((cx.cards().get(cx.s().obj(new).card).name, cx.s().obj(new).counters(CounterKind::MinusOne)));
        }
        out
    }

    /// CR 614.12, 122.6: a counter ban is asked of the permanent as it would exist on the battlefield. Under the Ban, the
    /// creature card gets no -1/-1 counter and the artifact card does; with the Animator making every noncreature artifact a
    /// creature there, neither does - the artifact is judged as the creature it enters as, not the card it was.
    #[test]
    fn a_counter_ban_judges_an_entering_permanent_as_it_would_exist() {
        assert_eq!(minus_counters(&[]), [("Grown", 1), ("Grown Relic", 1)]);
        assert_eq!(minus_counters(&[&BAN]), [("Grown", 0), ("Grown Relic", 1)]);
        assert_eq!(minus_counters(&[&BAN, &ANIMATOR]), [("Grown", 0), ("Grown Relic", 0)]);
    }

    /// CR 701.24c: both a stale specific reference and a move replacement still shuffle the referenced owner's library.
    #[test]
    fn shuffled_in_shuffles_the_referenced_library_when_the_object_does_not_arrive() {
        fn game_with_library() -> (Game, ObjectId) {
            let cards: &'static CardTable = Box::leak(Box::new(CardTable::new([&GROWN])));
            let deck = vec![cards.id_of(&GROWN); 12];
            let game = Game::new(cards, &Setup { decks: vec![deck, Vec::new()], ..Setup::default() });
            let source = game.state.zone(Zone::hand(PlayerId(0)))[0];
            (game, source)
        }

        // The object left its zone before the effect resolves: its old reference no longer moves anything, but the
        // owner's library is still shuffled.
        let (mut game, source) = game_with_library();
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        put_into_graveyard(&mut cx, &[source]);
        let rng = cx.s().rng;
        put_into_library(&mut cx, &[source], LibraryPlace::ShuffledIn);
        assert!(cx.s().obj(source).became.is_some());
        assert_eq!(cx.s().zone(Zone::graveyard(PlayerId(0))).len(), 1);
        assert_ne!(cx.s().rng, rng);

        // A battlefield object would move, but a replacement sends it elsewhere. Its owner's library is still shuffled.
        let (mut game, source) = game_with_library();
        let hand = Zone::hand(PlayerId(0));
        let timestamp = game.state.timestamp();
        game.state.zone_remove(hand, source);
        game.state.update_obj(source, |o| {
            o.zone = Zone::BATTLEFIELD;
            o.timestamp = timestamp;
            o.exile_on_leave = true;
        });
        game.state.zone_push(Zone::BATTLEFIELD, source);
        game.state.see_statics();
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        let rng = cx.s().rng;
        let moved = put_into_library(&mut cx, &[source], LibraryPlace::ShuffledIn);
        assert_eq!(cx.s().obj(moved[0]).zone, Zone::exile(PlayerId(0)));
        assert_ne!(cx.s().rng, rng);

        // Phasing leaves the same object, but does not make the library shuffle impossible.
        let (mut game, source) = game_with_library();
        let hand = Zone::hand(PlayerId(0));
        game.state.zone_remove(hand, source);
        game.state.update_obj(source, |o| o.zone = Zone::PHASED_OUT);
        game.state.zone_push(Zone::PHASED_OUT, source);
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        let rng = cx.s().rng;
        put_into_library(&mut cx, &[source], LibraryPlace::ShuffledIn);
        assert_eq!(cx.s().obj(source).zone, Zone::PHASED_OUT);
        assert_ne!(cx.s().rng, rng);
    }

    /// CR 701.26a, 400.7, 702.26b: only an untapped permanent present on the battlefield can become tapped. Repeating
    /// the instruction, remembering an old object, naming a card in hand, or a phased-out permanent does nothing and
    /// announces no tapped event.
    #[test]
    fn tap_only_changes_a_live_untapped_permanent_on_the_battlefield() {
        let (mut game, held) = game_with(&[&ANIMATOR]);
        let permanent = game.state.battlefield()[0];
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        cantrip_api::Game::tap(&mut cx, permanent);
        assert!(cx.s().obj(permanent).tapped);
        assert_eq!(cx.g.log.iter().filter(|r| matches!(r.event, Event::Tapped { permanent: id } if id == permanent)).count(), 1);
        let mark = cx.s().mark();
        let events = cx.g.log.len();
        cantrip_api::Game::tap(&mut cx, permanent);
        cantrip_api::Game::tap(&mut cx, held[0]);
        assert_eq!(cx.s().mark(), mark);
        assert_eq!(cx.g.log.len(), events);
        assert!(!cx.s().obj(held[0]).tapped);

        cantrip_api::Game::untap(&mut cx, permanent);
        let returned = return_to_hand(&mut cx, &[permanent])[0];
        let mark = cx.s().mark();
        let events = cx.g.log.len();
        cantrip_api::Game::tap(&mut cx, permanent);
        cantrip_api::Game::tap(&mut cx, returned);
        assert_eq!(cx.s().mark(), mark);
        assert_eq!(cx.g.log.len(), events);
        assert!(!cx.s().obj(permanent).tapped && !cx.s().obj(returned).tapped);

        let phased = put_onto_battlefield(&mut cx, &[(returned, PlayerId(0), false)], ZoneReason::Other).expect("no choices")[0];
        crate::phasing::phase_out(&mut cx, &[phased], None, false);
        let mark = cx.s().mark();
        let events = cx.g.log.len();
        cantrip_api::Game::tap(&mut cx, phased);
        assert_eq!(cx.s().mark(), mark);
        assert_eq!(cx.g.log.len(), events);
        assert!(!cx.s().obj(phased).tapped);
    }
}
