//! Counters (CR 122): the one path by which counters come to be on an object or a player.
//!
//! Every counter that is put on anything is put by a player (CR 122.6a, 608.2, 120.3b-d, 714.3c), and that player is
//! named here, never left out: "if you would put one or more counters" (Innkeeper's Talent, Vorinclex) asks who it is.
//! The counters are put through the replacement effects of counters being put (CR 614.1a) - on a permanent, a permanent
//! entering with them (CR 122.6, 614.1c) or a player - and the `CountersPut` event names the player too. The storage
//! (`Obj::counters`, `Player::poison` and the rest) is private to `cantrip-state`, and its one adder, `put_path_add`, is
//! called from this module alone (`only_the_put_path_puts_counters`). Taking counters off (`remove`) is not putting them.

use cantrip_api::*;
use cantrip_state::{Obj, PendingTrigger};

use crate::layers::{self, live};
use crate::{Cx, ops, replace};

/// How counters are put, besides by whom: by an effect (CR 609.1), or as a cost (CR 118.3) or a turn-based action
/// (CR 714.3c), which "if an effect would put one or more counters" (Doubling Season, `CounterPlacement::effect`) does
/// not see. Every other replacement of counters being put applies to all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    Effect,
    Cost,
    TurnBased,
}

/// `amount` counters of one kind put on `id` by `by` (CR 122.6). Nothing for an amount of none or fewer.
pub fn put(cx: &mut Cx, id: ObjectId, kind: CounterKind, amount: i32, by: PlayerId, how: How) {
    if amount <= 0 {
        return;
    }
    let mut counts = [0; CounterKind::COUNT];
    counts[kind as usize] = amount;
    put_all(cx, id, counts, by, how);
}

/// A restriction on this effect's final placement, not a replacement effect or a permanent-wide counter ban
/// (Clockwork Beast and Clockwork Avian; CR 101.2, 608.2c, 609.3, 614.17). Replacements see the requested quantity first.
pub fn put_with_limit(cx: &mut Cx, id: ObjectId, kind: CounterKind, amount: i32, by: PlayerId, maximum_total: i32) {
    if amount <= 0 { return; }
    let mut counts = [0; CounterKind::COUNT]; counts[kind as usize] = amount;
    let placement = Placement { object: id, counts, by };
    let limit = CounterLimit { object: id, kind, maximum_total };
    let events = cx.instruction(|cx| {
        let mut batch = Batch { pending: vec![placement], limit: Some(limit), ..Default::default() };
        while let Some(player) = batch.next_player(cx, |_| false) { batch.prepare_next(cx, player, How::Effect); }
        batch.retain_allowed(cx);
        batch.place(cx)
    }, |_| true);
    if !events.is_empty() { cx.emit_batch(&events, None); }
}

#[derive(Clone, Copy)]
struct CounterLimit {
    object: ObjectId,
    kind: CounterKind,
    maximum_total: i32,
}

/// Counters of several kinds put on `id` together by `by` (CR 122.6): one placement of `put_together`, announced as one
/// event however many kinds it has.
pub fn put_all(cx: &mut Cx, id: ObjectId, counts: [i32; CounterKind::COUNT], by: PlayerId, how: How) {
    put_together(cx, &[Placement { object: id, counts, by }], how);
}

/// Counters put on one object as one result (CR 122.6), of each kind, and the player who puts them (CR 122.6a): all one
/// instruction puts on it, or one source's damage dealt as counters (CR 120.3d), each its own though one player puts both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub object: ObjectId,
    pub counts: [i32; CounterKind::COUNT],
    pub by: PlayerId,
}

/// A cost paid on already chosen recipients (CR 118.3, 118.11, 118.12a): validate the whole before paying any part.
/// Keep separate components separate even on the same object (Sheltering Ancient's cost paid for each age counter);
/// a replacement that changes one component to zero still pays that component, rather than invalidating the cost.
pub fn pay_cost(cx: &mut Cx, payer: PlayerId, puts: &[(ObjectId, CounterKind, i32)]) -> bool {
    if cx.s().player(payer).lost || puts.iter().any(|&(object, kind, amount)| {
        amount < 0 || amount > 0 && (!live(cx.s(), object)
            || cx.s().obj(object).zone.is(ZoneKind::PhasedOut)
            || cx.s().obj(object).zone.is(ZoneKind::Battlefield)
                && crate::static_rules::counters_banned(cx, cx.cards(), object, cx.s().obj(object).controller, kind))
    }) {
        return false;
    }
    // The components are separate actions, paid after every choice is made (CR 702.24a). Do not merge their replacement
    // or trigger observations: a later component sees the earlier one's counters. A replacement may modify a later
    // recipient or prevent its placement; that still pays the declared cost (CR 118.11).
    for &(object, kind, amount) in puts {
        put(cx, object, kind, amount, payer, How::Cost);
    }
    true
}

/// Counters put on several objects by one instruction, or of several kinds on one (CR 608.2, 101.4): one simultaneous
/// event, which a "one or more" ability sees once (CR 603.2c, 122.6). Each placement is its own: the replacement effects of
/// counters being put apply to each (CR 614.1a), so two on one object - two infect sources' damage, CR 120.3d - are each
/// replaced ("that many plus one" adds one to each), and the caller makes what one instruction puts on an object one
/// placement (`Game::add_counters_together`). Each object's counters are first what the replacement effects of counters being put on it leave (CR 614.1a,
/// 616.1), every object's worked out before any counter goes on (CR 614.6: a replacement modifies the event before it
/// happens); then they are all put on, and all announced together.
pub fn put_together(cx: &mut Cx, placements: &[Placement], how: How) {
    put_mixed_together(cx, placements, &[], how);
}

/// One simultaneous instruction's permanent and player counters (CR 701.34a, 101.4, 616.1). All kinds on one
/// recipient form one placement; damage instead supplies separate occurrences per source. Nothing is added until
/// every replacement is prepared. Proliferate, ordinary placements and damage use this same batch/put path.
pub(crate) fn put_mixed_together(cx: &mut Cx, placements: &[Placement], players: &[PlayerPlacement], how: How) {
    let events = cx.instruction(|cx| place_mixed(cx, placements, players, how), |_| true);
    if !events.is_empty() { cx.emit_batch(&events, None); }
}

fn place_mixed(cx: &mut Cx, placements: &[Placement], players: &[PlayerPlacement], how: How) -> Vec<Event> {
    let mut batch = Batch { pending: placements.to_vec(), player_pending: players.to_vec(), ..Default::default() };
    while let Some(player) = batch.next_player(cx, |_| false) {
        batch.prepare_next(cx, player, how);
    }
    batch.retain_allowed(cx);
    batch.place(cx)
}

/// The existing counter preparation/application split as one mixed batch. Damage holds this batch alongside its
/// life results, inserting those occurrences into the same APNAP sequence; proliferate has only counter occurrences.
/// This is work pending for one instruction, never a second record of counters or event history.
#[derive(Default)]
pub(crate) struct Batch {
    pending: Vec<Placement>,
    player_pending: Vec<PlayerPlacement>,
    prepared: Vec<Placement>,
    player_prepared: Vec<PlayerPlacement>,
    limit: Option<CounterLimit>,
}

impl Batch {
    pub(crate) fn push(&mut self, placement: Placement) {
        self.pending.push(placement);
    }

    pub(crate) fn push_player(&mut self, placement: PlayerPlacement) {
        self.player_pending.push(placement);
    }

    /// Re-read controllers between occurrences: concessions/continuations can return an unprepared recipient to an
    /// earlier player, restarting APNAP (CR 101.4d). `also` includes the damage batch's still-pending life occurrences.
    pub(crate) fn next_player(&self, cx: &Cx, also: impl Fn(PlayerId) -> bool) -> Option<PlayerId> {
        cx.s().apnap().find(|&player| {
            also(player) || self.pending.iter().any(|p| cx.s().obj(p.object).controller == player)
                || self.player_pending.iter().any(|p| p.player == player)
        })
    }

    pub(crate) fn prepare_next(&mut self, cx: &mut Cx, player: PlayerId, how: How) {
        if let Some(at) = self.pending.iter().position(|p| cx.s().obj(p.object).controller == player) {
            let mut placement = self.pending.remove(at);
            prepare(cx, &mut placement, how);
            self.prepared.push(placement);
        } else if let Some(at) = self.player_pending.iter().position(|p| p.player == player) {
            let mut placement = self.player_pending.remove(at);
            prepare_player(cx, &mut placement);
            self.player_prepared.push(placement);
        }
    }

    /// Validate all original counter results against one state after replacement continuations, before application.
    pub(crate) fn retain_allowed(&mut self, cx: &Cx) {
        retain_allowed(cx, &mut self.prepared);
        retain_players_allowed(cx, &mut self.player_prepared);
    }

    /// All results are prepared and allowed: apply through the single put path and return the permanent events for
    /// the caller to announce with the rest of its instruction. No replacement runs against partially updated state.
    pub(crate) fn place(self, cx: &mut Cx) -> Vec<Event> {
        place_players_prepared(cx, &self.player_prepared);
        match self.limit {
            Some(limit) => place_prepared_with_limit(cx, &self.prepared, Some(limit)),
            None => place_prepared(cx, &self.prepared),
        }
    }
}

/// Phased-out permanents cannot be affected (CR 702.26b); an object gone from its zone is another (CR 400.7).
fn can_put(cx: &Cx, id: ObjectId) -> bool {
    live(cx.s(), id) && !cx.s().obj(id).zone.is(ZoneKind::PhasedOut)
}

/// Apply replacements to one occurrence, putting nothing yet (CR 614.6). Separate infect sources remain separate
/// placements even on the same creature: "that many plus one" adds one to each (CR 120.3d).
pub(crate) fn prepare(cx: &mut Cx, placement: &mut Placement, how: How) {
    placement.counts = placement.counts.map(|n| n.max(0));
    if !can_put(cx, placement.object) {
        placement.counts = [0; CounterKind::COUNT];
    } else if cx.s().obj(placement.object).zone.is(ZoneKind::Battlefield) {
        let controller = cx.s().obj(placement.object).controller;
        placement.counts = replace::counters(cx, placement.object, controller, placement.by, false, how == How::Effect, placement.counts);
    }
}

/// Recheck recipients and prohibitions after every replacement's continuation, before any original result applies
/// (CR 101.2, 614.6). Damage does this for all result kinds against the same pre-result state.
pub(crate) fn retain_allowed(cx: &Cx, placements: &mut [Placement]) {
    for placement in placements {
        placement.counts = if can_put(cx, placement.object) {
            replace::without_banned(cx, placement.object, cx.s().obj(placement.object).controller, placement.counts)
        } else {
            [0; CounterKind::COUNT]
        };
    }
}

/// Put prepared, allowed counters through the single journalled put path. No replacement choice remains here; the
/// caller announces these events together with every other result of its instruction (CR 603.2c).
pub(crate) fn place_prepared(cx: &mut Cx, placements: &[Placement]) -> Vec<Event> {
    place_prepared_with_limit(cx, placements, None)
}

fn place_prepared_with_limit(cx: &mut Cx, placements: &[Placement], limit: Option<CounterLimit>) -> Vec<Event> {
    let mut events = Vec::new();
    for m in placements {
        if m.counts.iter().all(|&n| n <= 0) {
            continue;
        }
        let (id, mut semantic) = (m.object, m.counts.map(|n| n.max(0)));
        // Apply the restriction after counter replacements and before physical storage checks or events.
        if let Some(limit) = limit.filter(|limit| limit.object == id) {
            let room = limit.maximum_total.saturating_sub(cx.s().obj(id).counters(limit.kind)).max(0);
            semantic[limit.kind as usize] = semantic[limit.kind as usize].min(room);
        }
        if semantic.iter().all(|&n| n <= 0) { continue; }
        // RX-5 retains the counter domain and the state's saturating adder. Wider intermediate proposals do not
        // turn an accepted boundary into a panic or widen the stored counter field.
        let counts = semantic.map(|n| n.max(0));
        cx.st().update_obj(id, |o| o.put_path_add(&counts));
        // The layers look for hone counters from now on (CR 122.1j).
        if counts[CounterKind::Hone as usize] > 0 {
            cx.st().see_statics();
        }
        // CR 613.7c: a counter gets a timestamp as it is put on, and every counter of its kind already there gets the same
        // one; the ability a keyword counter gives applies in layer 6 at it (CR 613.1f, 122.1b).
        if cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
            for k in CounterKind::KEYWORDS {
                if counts[k as usize] > 0 {
                    cx.st().stamp_counters(id, k);
                }
            }
        }
        // The counters that were put are announced (CR 121.1, 122.6a), for a Saga's chapters (CR 714.2b) and for "whenever
        // counters are put on this", as many as the replacements left, with the player who put them.
        for kind in CounterKind::ALL {
            let amount = counts[kind as usize];
            if amount > 0 {
                let total = cx.s().obj(id).counters(kind);
                let types = if cx.s().obj(id).zone.is(ZoneKind::Battlefield) { cx.chars(id).types } else { TypeSet::default() };
                events.push(Event::CountersPut { object: id, kind, amount, total, by: m.by, types });
            }
        }
    }
    events
}

/// Takes up to `n` counters of one kind off `id` (CR 122.1): what a cost removes (CR 118.3), what an effect removes, and
/// the state-based action of CR 704.5q, recorded as `Event::CountersRemoved`. When the last defense counter comes off a
/// Siege, its defeat ability triggers (CR 310.12b).
pub fn remove(cx: &mut Cx, id: ObjectId, kind: CounterKind, n: i32) {
    if n <= 0 || !live(cx.s(), id) || cx.s().obj(id).zone.is(ZoneKind::PhasedOut) {
        return;
    }
    let before = cx.s().obj(id).counters(kind);
    if before <= 0 {
        return;
    }
    cx.st().update_obj(id, |o| o.remove_counters(kind, n));
    // What came off is recorded, with what removed it as the record's cause: "a mire counter has not been removed from with
    // this artifact" (Cyclopean Tomb) asks the history.
    let amount = before - cx.s().obj(id).counters(kind);
    cx.record(Event::CountersRemoved { object: id, kind, amount });
    ops::end_lapsed_durations(cx, id);
    if kind == CounterKind::Defense && cx.s().obj(id).counters(CounterKind::Defense) == 0 {
        siege_defeat_triggers(cx, id);
    }
}

/// "Move" up to `n` counters of one kind from `from` onto `to` (CR 122.5): taken off `from` and put onto `to` by the player doing it
/// (CR 122.6a). If either action isn't possible - the same object, none of the kind on `from`, `to` can't have counters of the
/// kind put on it (CR 101.2: `counters_banned`), or either has left its zone (CR 400.7) or phased out (CR 702.26b) - none is
/// removed or put. How many came off `from`.
pub fn mv(cx: &mut Cx, from: ObjectId, to: ObjectId, kind: CounterKind, n: i32) -> i32 {
    let here = |cx: &Cx, id: ObjectId| live(cx.s(), id) && !cx.s().obj(id).zone.is(ZoneKind::PhasedOut);
    if n <= 0 || from == to || !here(cx, from) || !here(cx, to) || cx.s().obj(from).counters(kind) <= 0 {
        return 0;
    }
    let controller = cx.s().obj(to).controller;
    if crate::static_rules::counters_banned(cx, cx.cards(), to, controller, kind) {
        return 0;
    }
    let before = cx.s().obj(from).counters(kind);
    remove(cx, from, kind, n);
    let moved = before - cx.s().obj(from).counters(kind);
    let by = cx.doer();
    put(cx, to, kind, moved, by, How::Effect);
    moved
}

/// "Remove up to `maximum` counters from target creature" as an effect resolves (CR 608.2d, 122.1): `player` chooses
/// which of the counters on `id`, of any kinds and how many of each, from none to `maximum` in all, and they come off
/// together. Nothing is asked of an object that has changed zones (CR 400.7) or that has no counters. How many came off:
/// "for each counter removed this way". Heartless Act, Hex Parasite, Price of Betrayal (its permanents).
pub fn remove_up_to(cx: &mut Cx, player: PlayerId, source: ObjectId, id: ObjectId, maximum: u8) -> crate::R<u16> {
    if maximum == 0 || !live(cx.s(), id) || cx.s().obj(id).zone.is(ZoneKind::PhasedOut) {
        return Ok(0);
    }
    let obj = cx.s().obj(id);
    let pools: Vec<CounterPool> = CounterKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = obj.counters(kind);
            (count > 0).then_some(CounterPool { object: id, kind, count: count as u32 })
        })
        .collect();
    if pools.is_empty() {
        return Ok(0);
    }
    let Answer::Counters(taken) = cx.ask_core(player, RequestKind::RemoveCounters { source, pools, total: maximum, up_to: true }, false)? else {
        return Ok(0);
    };
    let mut removed = 0u16;
    for (object, kind, n) in taken {
        let before = cx.s().obj(object).counters(kind);
        remove(cx, object, kind, n as i32);
        removed += (before - cx.s().obj(object).counters(kind)) as u16;
    }
    Ok(removed)
}

/// "Remove a counter from a [permanent] you control" as an effect resolves (CR 608.2d, 122.1): `player` chooses one counter, of
/// any kind, on any of `permanents`, and it comes off. Nothing is asked when none of them has one (or they have changed zones,
/// CR 400.7); says whether one came off, for "if you do". Mister Hyde, Monster Within, Sanctuary Warden and Chisei, Heart of Oceans.
pub fn remove_one_from_among(cx: &mut Cx, player: PlayerId, source: ObjectId, permanents: &[ObjectId]) -> crate::R<bool> {
    let mut pools: Vec<CounterPool> = Vec::new();
    for &id in permanents {
        if !live(cx.s(), id) || cx.s().obj(id).zone.is(ZoneKind::PhasedOut) {
            continue;
        }
        let obj = cx.s().obj(id);
        pools.extend(CounterKind::ALL.into_iter().filter_map(|kind| {
            let count = obj.counters(kind);
            (count > 0).then_some(CounterPool { object: id, kind, count: count as u32 })
        }));
    }
    if pools.is_empty() {
        return Ok(false);
    }
    let Answer::Counters(taken) = cx.ask_core(player, RequestKind::RemoveCounters { source, pools, total: 1, up_to: false }, false)? else {
        return Ok(false);
    };
    let mut removed = false;
    for (object, kind, n) in taken {
        let before = cx.s().obj(object).counters(kind);
        remove(cx, object, kind, n as i32);
        removed |= cx.s().obj(object).counters(kind) < before;
    }
    Ok(removed)
}

/// Chooses a kind of counter already on `object`, removing none (CR 122.1): the only kind it has with no question, or the
/// player's choice among several. For an effect (CR 608.2d; Dathon, His Heart Open), `cost` false: no kind is `None`, and
/// the question can't be cancelled. For a cost that removes "a counter of any kind" from its source (CR 118.3, 601.2b),
/// `cost` true: the question is asked as part of the cost, and the player may back out of it (`Stop::Cancel`).
pub fn choose_kind(cx: &mut Cx, player: PlayerId, source: ObjectId, object: ObjectId, cost: bool) -> crate::R<Option<CounterKind>> {
    let kinds: Vec<CounterKind> = CounterKind::ALL.into_iter().filter(|&kind| cx.s().obj(object).counters(kind) > 0).collect();
    if kinds.len() <= 1 {
        return Ok(kinds.first().copied());
    }
    let ask = RequestKind::ChooseModes {
        source,
        modes: kinds.iter().map(|&kind| Label::Counter(kind)).collect(),
        eligible: (0..kinds.len() as u8).collect(),
        count: 1,
        at_most: 0,
        affordable: Vec::new(),
        additional_cost: cost,
        repeat: false,
    };
    match cx.ask_core(player, ask, cost)? {
        Answer::Modes(chosen) => Ok(chosen.first().map(|&index| kinds[index as usize])),
        _ if cost => Err(crate::Stop::Cancel),
        _ => Ok(None),
    }
}

/// A Siege that has lost its last defense counter triggers its defeat ability (CR 310.12b).
fn siege_defeat_triggers(cx: &mut Cx, id: ObjectId) {
    if !cx.s().obj(id).zone.is(ZoneKind::Battlefield) {
        return;
    }
    let chars = layers::chars(cx.cards(), cx.s(), &cx.g.log, id);
    if !chars.has_subtype(Subtype::Siege) {
        return;
    }
    let ability = AbilityRef { card: cx.cards().intrinsic, index: cantrip_api::intrinsic::SIEGE_DEFEAT };
    let controller = cx.s().obj(id).controller;
    cx.st().trigger_push(PendingTrigger { event_amount: None,
        source: id,
        ability,
        controller,
        subject: Target::None,
        data: [0; 2],
        members: cantrip_state::Span::default(),
        count: None,
        since: None,
        acquired: Acquired::OWN,
        words: None,
    });
}

/// What a permanent enters with because of what it is (CR 614.1c), counters it enters with like any other (CR 122.6),
/// put on by its controller (CR 122.6a): a planeswalker its printed loyalty (CR 306.5b), a battle its printed defense
/// (CR 310.4b), a Saga a lore counter (CR 714.3a) - or, with read ahead, the number its controller chose as it entered
/// (`read_ahead`, CR 702.155b, 714.3b). Added to the counters the replacements give it, before the replacements of counters
/// being put apply to them all (`entering`).
pub fn intrinsic(cards: &CardTable, obj: &Obj, read_ahead: Option<u8>, counts: &mut [i32; CounterKind::COUNT]) {
    let card = obj.copied_card().unwrap_or_else(|| cards.face(obj.card, obj.transformed));
    let def = cards.get(card);
    // A copy effect's exception may state its loyalty (CR 707.9b: "has starting loyalty X"); a face-down permanent has
    // the values it is face down as, and none of a copy's.
    if let Some(loyalty) = obj.copiable().and_then(|c| c.stated_loyalty()).or(def.loyalty) {
        counts[CounterKind::Loyalty as usize] = counts[CounterKind::Loyalty as usize].saturating_add(i32::from(loyalty));
    }
    if let Some(defense) = def.defense {
        counts[CounterKind::Defense as usize] = counts[CounterKind::Defense as usize].saturating_add(i32::from(defense));
    }
    let saga = def.subtypes.has(Subtype::Saga) && def.abilities.iter().any(|a| matches!(a, AbilityDef::Triggered(t) if t.chapter > 0));
    if let Some(chosen) = read_ahead {
        counts[CounterKind::Lore as usize] += i32::from(chosen);
    } else if saga {
        counts[CounterKind::Lore as usize] = counts[CounterKind::Lore as usize].saturating_add(1);
    }
}

/// The counters a permanent will enter with (CR 122.6, 614.1c), once the replacement effects of counters being put have
/// applied to them (CR 614.1a), the permanent judged as it would exist on the battlefield (the caller's `judged_entering`):
/// `by` puts them, its controller unless the effect giving them names another player (CR 122.6a). Placed by
/// `place_arrived` once it is there.
pub fn entering(cx: &mut Cx, id: ObjectId, controller: PlayerId, by: PlayerId, counts: [i32; CounterKind::COUNT]) -> [i32; CounterKind::COUNT] {
    replace::counters(cx, id, controller, by, true, true, counts)
}

/// Puts the counters an object arrives with on it as it arrives, before anything sees it (CR 614.1c: a permanent enters
/// with them), and announces them as counters `by` put on it (CR 122.6) into `events`, which the caller records after
/// the move's own event: a Saga's lore counter too, so its first chapter triggers (CR 714.3a, 714.2b), and all of them, so
/// "when the Nth ... counter is put" sees each it entered with. For a permanent, those `entering` worked out; for a card
/// arriving elsewhere, those a replacement effect gave it (Dauthi Voidwalker's void counter, CR 614.1a), which no
/// replacement of counters put on a permanent sees.
pub fn place_arrived(cx: &mut Cx, id: ObjectId, counts: [i32; CounterKind::COUNT], by: PlayerId, events: &mut Vec<Event>) {
    if counts.iter().all(|&n| n <= 0) {
        return;
    }
    let counts = counts.map(|n| n.max(0));
    cx.st().update_obj(id, |o| o.put_path_add(&counts));
    for kind in CounterKind::ALL {
        let amount = counts[kind as usize];
        if amount > 0 {
            let total = cx.s().obj(id).counters(kind);
            let types = if cx.s().obj(id).zone.is(ZoneKind::Battlefield) { cx.chars(id).types } else { TypeSet::default() };
            events.push(Event::CountersPut { object: id, kind, amount, total, by, types });
        }
    }
}

/// The permanent as it would exist on the battlefield with the counters it would enter with (CR 614.12): not counters
/// put, but a moment's hypothesis that `ops::as_it_would_exist` takes back (its journal mark) before anything happens.
pub(crate) fn hypothetical(o: &mut Obj, counts: &[i32; CounterKind::COUNT]) {
    o.clear_counters();
    o.put_path_add(&counts);
}

/// Counters of a player's kinds (CR 122.1): poison, energy, experience.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerCounts {
    pub poison: u16,
    pub energy: u16,
    pub experience: u16,
}

/// One occurrence of counters given to a player, kept separate from other sources' occurrences (CR 120.3b, 120.3g).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlayerPlacement {
    pub player: PlayerId,
    pub counts: PlayerCounts,
    pub by: PlayerId,
}

/// Counters put on `player` by `by` (CR 122.6): an event replacement effects can change (CR 614.1a). A player who can't
/// get counters of a kind gets none (CR 101.2: Melira, Sylvok Outcast; Solemnity), and no replacement effect of the event
/// that doesn't happen applies.
pub fn put_on_player(cx: &mut Cx, player: PlayerId, counts: PlayerCounts, by: PlayerId) {
    put_mixed_together(cx, &[], &[PlayerPlacement { player, counts, by }], How::Effect);
}

/// Prepare, but do not give, one source's player counters (CR 120.4c, 614.6). Damage schedules this alongside every
/// other result for the affected player, never while its damage replacements are still being applied.
pub(crate) fn prepare_player(cx: &mut Cx, placement: &mut PlayerPlacement) {
    retain_players_allowed(cx, std::slice::from_mut(placement));
    if placement.counts != PlayerCounts::default() {
        placement.counts = replace::player_counters(cx, placement.player, placement.by, placement.counts);
    }
}

/// A later replacement may remove a recipient or change a prohibition (CR 800.4a, 101.2). Check all players before
/// applying any original result, as for permanent counters and life changes.
pub(crate) fn retain_players_allowed(cx: &Cx, placements: &mut [PlayerPlacement]) {
    for placement in placements {
        let player = placement.player;
        if !cx.s().in_game(player) {
            placement.counts = PlayerCounts::default();
            continue;
        }
        let banned = |n: u16, kind| n > 0 && crate::static_rules::player_counters_banned(cx, cx.cards(), player, kind);
        let counts = &mut placement.counts;
        if banned(counts.poison, PlayerCounter::Poison) {
            counts.poison = 0;
        }
        if banned(counts.energy, PlayerCounter::Energy) {
            counts.energy = 0;
        }
        if banned(counts.experience, PlayerCounter::Experience) {
            counts.experience = 0;
        }
    }
}

/// Apply prepared player placements through the same put path used by `put_on_player`.
pub(crate) fn place_players_prepared(cx: &mut Cx, placements: &[PlayerPlacement]) {
    for placement in placements {
        let PlayerCounts { poison, energy, experience } = placement.counts;
        if placement.counts != PlayerCounts::default() {
            cx.st().update_player(placement.player, |p| p.put_path_add(poison, energy, experience));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Game, Setup};

    const YOU: PlayerId = PlayerId(0);
    const THEM: PlayerId = PlayerId(1);

    static BEAR: CardDef = CardDef {
        name: "Counter Phasing Test Bear", types: TypeSet::of(&[CardType::Creature]),
        power: Some(2), toughness: Some(2), ..CardDef::BLANK
    };

    fn applies(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
        matches!(proposal, Proposal::Counters(c) if c.effect && c.putter == THEM)
    }

    fn twice(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
        if let Proposal::Counters(c) = proposal {
            for n in &mut c.counts { *n *= 2; }
        }
    }

    static DOUBLER: CardDef = CardDef {
        name: "Counter Phasing Test Doubler", types: TypeSet::of(&[CardType::Enchantment]),
        replacements: &[replacement(ReplacementKind::Counters, applies, twice, "double counters put by the other player")],
        ..CardDef::BLANK
    };

    fn cx(g: &mut Game) -> Cx<'_> {
        Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
    }

    #[test]
    fn rx5_counter_storage_saturates_and_preserves_the_existing_event_contract() {
        let table = Box::leak(Box::new(CardTable::new([&BEAR])));
        let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let bear = game.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, bear);
        let mut context = cx(&mut game);
        put(&mut context, bear, CounterKind::Shield, i32::MAX - 7, YOU, How::Effect);
        put(&mut context, bear, CounterKind::Shield, 10, YOU, How::Effect);
        assert_eq!(context.s().obj(bear).counters(CounterKind::Shield), i32::MAX);
        assert!(matches!(context.g.log.last().unwrap().event,
            Event::CountersPut { object, kind: CounterKind::Shield, amount: 10, total: i32::MAX, .. } if object == bear));
    }

    #[test]
    fn phased_out_placement_does_nothing_but_existing_counters_and_other_zones_survive() {
        let table = Box::leak(Box::new(CardTable::new([&BEAR, &DOUBLER])));
        let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let object = |game: &mut Game, card: &'static CardDef, zone: Zone| {
            let id = game.state.push_obj(Obj::new(table.id_of(card), YOU, zone));
            game.state.zone_push(zone, id);
            id
        };
        let bear = object(&mut game, &BEAR, Zone::BATTLEFIELD);
        object(&mut game, &DOUBLER, Zone::BATTLEFIELD);
        let cards = [Zone::hand(YOU), Zone::graveyard(YOU), Zone::exile(YOU)].map(|z| object(&mut game, &BEAR, z));
        let mut context = cx(&mut game);
        put_all(&mut context, bear, {
            let mut c = [0; CounterKind::COUNT]; c[CounterKind::PlusOne as usize] = 1; c[CounterKind::Shield as usize] = 1; c
        }, THEM, How::Effect);
        assert_eq!(context.s().obj(bear).counters(CounterKind::PlusOne), 2);
        assert_eq!(context.s().obj(bear).counters(CounterKind::Shield), 2);
        crate::phasing::phase_out(&mut context, &[bear], None, false);
        let events = context.g.log.len();
        for how in [How::Effect, How::Cost, How::TurnBased] {
            put(&mut context, bear, CounterKind::PlusOne, 3, THEM, how);
        }
        remove(&mut context, bear, CounterKind::PlusOne, 1);
        assert_eq!(remove_up_to(&mut context, YOU, bear, bear, 3), Ok(0));
        assert_eq!(context.g.log.len(), events, "a nonexistent placement emitted an event");
        assert_eq!(context.s().obj(bear).counters(CounterKind::PlusOne), 2);
        assert_eq!(context.s().obj(bear).counters(CounterKind::Shield), 2);
        for card in cards {
            put(&mut context, card, CounterKind::PlusOne, 3, THEM, How::Effect);
            assert_eq!(context.s().obj(card).counters(CounterKind::PlusOne), 3, "card in another zone was rejected or doubled");
        }
        crate::phasing::untap_step(&mut context, YOU);
        put(&mut context, bear, CounterKind::PlusOne, 1, THEM, How::Effect);
        assert_eq!(context.s().obj(bear).counters(CounterKind::PlusOne), 4);
        assert!(matches!(context.g.log.last().unwrap().event, Event::CountersPut { object, amount: 2, total: 4, by: THEM, .. } if object == bear));
    }

    /// CR 122.5: a counter moves only if it can come off the first object and go onto a second, other one that is still in
    /// play: the same object, a first with none, a phased-out second, and one that can't be put on by Solemnity-like rules
    /// (the scenario) move nothing; an ordinary move takes them off and puts them on, up to what is there.
    #[test]
    fn counters_move_only_when_both_halves_are_possible() {
        let table = Box::leak(Box::new(CardTable::new([&BEAR])));
        let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let bear = |game: &mut Game| {
            let id = game.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, id);
            id
        };
        let (a, b, c) = (bear(&mut game), bear(&mut game), bear(&mut game));
        let mut context = cx(&mut game);
        context.doer = Some(YOU);
        put(&mut context, a, CounterKind::PlusOne, 3, YOU, How::Effect);
        assert_eq!(mv(&mut context, a, a, CounterKind::PlusOne, 2), 0, "onto itself");
        assert_eq!(mv(&mut context, b, a, CounterKind::PlusOne, 2), 0, "none to move");
        assert_eq!(mv(&mut context, a, b, CounterKind::PlusOne, 0), 0, "none asked");
        crate::phasing::phase_out(&mut context, &[c], None, false);
        assert_eq!(mv(&mut context, a, c, CounterKind::PlusOne, 2), 0, "onto a phased-out permanent");
        assert_eq!(context.s().obj(a).counters(CounterKind::PlusOne), 3);
        assert_eq!(mv(&mut context, a, b, CounterKind::PlusOne, 5), 3, "as many as it has");
        assert_eq!(context.s().obj(a).counters(CounterKind::PlusOne), 0);
        assert_eq!(context.s().obj(b).counters(CounterKind::PlusOne), 3);
    }

    fn phase_recipient(g: &mut dyn cantrip_api::Game, _: &Asker, _: &Proposal, after: &Proposal) {
        if let Proposal::Counters(c) = after { g.phase_out(&[c.object]); }
    }

    static PHASER: CardDef = CardDef {
        name: "Counter Phasing Test Continuation", types: TypeSet::of(&[CardType::Enchantment]),
        replacements: &[ReplacementDef {
            then: Some(phase_recipient),
            ..replacement(ReplacementKind::Counters, applies, twice, "double counters and phase out recipient")
        }],
        ..CardDef::BLANK
    };

    #[test]
    fn replacement_continuation_phasing_out_recipient_prevents_the_remaining_placement() {
        let table = Box::leak(Box::new(CardTable::new([&BEAR, &PHASER])));
        let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let bear = game.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, bear);
        let phaser = game.state.push_obj(Obj::new(table.id_of(&PHASER), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, phaser);
        put(&mut cx(&mut game), bear, CounterKind::PlusOne, 2, THEM, How::Effect);
        assert!(game.state.obj(bear).zone.is(ZoneKind::PhasedOut));
        assert_eq!(game.state.obj(bear).counters(CounterKind::PlusOne), 0);
        assert!(!game.log.iter().any(|r| matches!(r.event, Event::CountersPut { .. })));
    }

    /// The counter storage's adder is called from the put path alone: any other caller would put counters with no
    /// player putting them and no replacement applied (CR 122.6a, 614.1a). The privacy of the storage stops every other
    /// write; this stops a second caller of the one adder, which the compiler cannot, as it is in another crate.
    #[test]
    fn only_the_put_path_puts_counters() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut offenders = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    let normal = path.to_string_lossy().replace('\\', "/");
                    let allowed = normal.ends_with("cantrip-rules/src/counters.rs") || normal.ends_with("cantrip-state/src/object.rs");
                    if !allowed && text.contains(concat!("put_path", "_add")) {
                        offenders.push(normal);
                    }
                }
            }
        }
        assert!(offenders.is_empty(), "counters put outside the put path (crates/cantrip-rules/src/counters.rs): {offenders:?}");
    }
}
