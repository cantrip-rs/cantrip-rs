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
pub fn put(cx: &mut Cx, id: ObjectId, kind: CounterKind, amount: i16, by: PlayerId, how: How) {
    if amount <= 0 {
        return;
    }
    let mut counts = [0; CounterKind::COUNT];
    counts[kind as usize] = amount;
    put_all(cx, id, counts, by, how);
}

/// Counters of several kinds put on `id` together by `by` (CR 122.6): one placement of `put_together`, announced as one
/// event however many kinds it has.
pub fn put_all(cx: &mut Cx, id: ObjectId, counts: [i16; CounterKind::COUNT], by: PlayerId, how: How) {
    put_together(cx, &[Placement { object: id, counts, by }], how);
}

/// Counters put on one object as one result (CR 122.6), of each kind, and the player who puts them (CR 122.6a): all one
/// instruction puts on it, or one source's damage dealt as counters (CR 120.3d), each its own though one player puts both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub object: ObjectId,
    pub counts: [i16; CounterKind::COUNT],
    pub by: PlayerId,
}

/// Counters put on several objects by one instruction, or of several kinds on one (CR 608.2, 101.4): one simultaneous
/// event, which a "one or more" ability sees once (CR 603.2c, 122.6). Each placement is its own: the replacement effects of
/// counters being put apply to each (CR 614.1a), so two on one object - two infect sources' damage, CR 120.3d - are each
/// replaced ("that many plus one" adds one to each), and the caller makes what one instruction puts on an object one
/// placement (`Game::add_counters_together`). Each object's counters are first what the replacement effects of counters being put on it leave (CR 614.1a,
/// 616.1), every object's worked out before any counter goes on (CR 614.6: a replacement modifies the event before it
/// happens); then they are all put on, and all announced together.
pub fn put_together(cx: &mut Cx, placements: &[Placement], how: How) {
    let events = place_together(cx, placements, how);
    if !events.is_empty() {
        cx.emit_batch(&events, None);
    }
}

/// `put_together`, the events handed back for the caller to announce with the rest of a simultaneous event: the counters
/// damage is dealt as (CR 120.3d) are announced with the damage (CR 510.2, 603.2c).
pub fn place_together(cx: &mut Cx, placements: &[Placement], how: How) -> Vec<Event> {
    let mut merged: Vec<Placement> = placements
        .iter()
        .filter(|p| p.counts.iter().any(|&n| n > 0))
        .map(|p| Placement { counts: p.counts.map(|n| n.max(0)), ..*p })
        .collect();
    // Phased-out permanents cannot be affected (CR 702.26b); an object gone from its zone is another (CR 400.7).
    let can = |cx: &Cx, id: ObjectId| live(cx.s(), id) && !cx.s().obj(id).zone.is(ZoneKind::PhasedOut);
    // On a permanent, as many as the replacement effects of counters being put leave (CR 614.1a); on a card in another
    // zone, as many as it can have (CR 101.2), which no replacement of counters put on a permanent sees.
    for m in &mut merged {
        if !can(cx, m.object) {
            m.counts = [0; CounterKind::COUNT];
            continue;
        }
        if cx.s().obj(m.object).zone.is(ZoneKind::Battlefield) {
            let controller = cx.s().obj(m.object).controller;
            m.counts = replace::counters(cx, m.object, controller, m.by, false, how == How::Effect, m.counts);
        }
    }
    let mut events = Vec::new();
    for m in &merged {
        // A replacement's continuation may move or phase out a recipient before the placement finishes.
        if !can(cx, m.object) || m.counts.iter().all(|&n| n <= 0) {
            continue;
        }
        let (id, counts) = (m.object, m.counts);
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
/// the state-based action of CR 704.5q. When the last defense counter comes off a Siege, its defeat ability triggers
/// (CR 310.12b).
pub fn remove(cx: &mut Cx, id: ObjectId, kind: CounterKind, n: i16) {
    if n <= 0 || !live(cx.s(), id) || cx.s().obj(id).zone.is(ZoneKind::PhasedOut) {
        return;
    }
    let before = cx.s().obj(id).counters(kind);
    if before <= 0 {
        return;
    }
    cx.st().update_obj(id, |o| o.remove_counters(kind, n));
    ops::end_lapsed_durations(cx, id);
    if kind == CounterKind::Defense && cx.s().obj(id).counters(CounterKind::Defense) == 0 {
        siege_defeat_triggers(cx, id);
    }
}

/// "Move" up to `n` counters of one kind from `from` onto `to` (CR 122.5): taken off `from` and put onto `to` by the player doing it
/// (CR 122.6a). If either action isn't possible - the same object, none of the kind on `from`, `to` can't have counters of the
/// kind put on it (CR 101.2: `counters_banned`), or either has left its zone (CR 400.7) or phased out (CR 702.26b) - none is
/// removed or put. How many came off `from`.
pub fn mv(cx: &mut Cx, from: ObjectId, to: ObjectId, kind: CounterKind, n: i16) -> i16 {
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
            (count > 0).then_some(CounterPool { object: id, kind, count: count as u16 })
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
        remove(cx, object, kind, n.min(i16::MAX as u16) as i16);
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
            (count > 0).then_some(CounterPool { object: id, kind, count: count as u16 })
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
        remove(cx, object, kind, n.min(i16::MAX as u16) as i16);
        removed |= cx.s().obj(object).counters(kind) < before;
    }
    Ok(removed)
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
    cx.st().trigger_push(PendingTrigger {
        source: id,
        ability,
        controller,
        subject: Target::None,
        data: [0; 2],
        members: cantrip_state::Span::default(),
        count: None,
        since: None,
        acquired: Acquired::OWN,
    });
}

/// What a permanent enters with because of what it is (CR 614.1c), counters it enters with like any other (CR 122.6),
/// put on by its controller (CR 122.6a): a planeswalker its printed loyalty (CR 306.5b), a battle its printed defense
/// (CR 310.4b), a Saga a lore counter (CR 714.3a). Added to the counters the replacements give it, before the replacements
/// of counters being put apply to them all (`entering`).
pub fn intrinsic(cards: &CardTable, obj: &Obj, counts: &mut [i16; CounterKind::COUNT]) {
    let card = obj.copied_card().unwrap_or_else(|| cards.face(obj.card, obj.transformed));
    let def = cards.get(card);
    // A copy effect's exception may state its loyalty (CR 707.9b: "has starting loyalty X"); a face-down permanent has
    // the values it is face down as, and none of a copy's.
    if let Some(loyalty) = obj.copiable().and_then(|c| c.stated_loyalty()).or(def.loyalty) {
        counts[CounterKind::Loyalty as usize] += loyalty;
    }
    if let Some(defense) = def.defense {
        counts[CounterKind::Defense as usize] += defense;
    }
    let saga = def.subtypes.has(Subtype::Saga) && def.abilities.iter().any(|a| matches!(a, AbilityDef::Triggered(t) if t.chapter > 0));
    if saga {
        counts[CounterKind::Lore as usize] += 1;
    }
}

/// The counters a permanent will enter with (CR 122.6, 614.1c), once the replacement effects of counters being put have
/// applied to them (CR 614.1a), the permanent judged as it would exist on the battlefield (the caller's `judged_entering`):
/// `by` puts them, its controller unless the effect giving them names another player (CR 122.6a). Placed by
/// `place_arrived` once it is there.
pub fn entering(cx: &mut Cx, id: ObjectId, controller: PlayerId, by: PlayerId, counts: [i16; CounterKind::COUNT]) -> [i16; CounterKind::COUNT] {
    replace::counters(cx, id, controller, by, true, true, counts)
}

/// Puts the counters an object arrives with on it as it arrives, before anything sees it (CR 614.1c: a permanent enters
/// with them), and announces them as counters `by` put on it (CR 122.6) into `events`, which the caller records after
/// the move's own event: a Saga's lore counter too, so its first chapter triggers (CR 714.3a, 714.2b), and all of them, so
/// "when the Nth ... counter is put" sees each it entered with. For a permanent, those `entering` worked out; for a card
/// arriving elsewhere, those a replacement effect gave it (Dauthi Voidwalker's void counter, CR 614.1a), which no
/// replacement of counters put on a permanent sees.
pub fn place_arrived(cx: &mut Cx, id: ObjectId, counts: [i16; CounterKind::COUNT], by: PlayerId, events: &mut Vec<Event>) {
    if counts.iter().all(|&n| n <= 0) {
        return;
    }
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
pub(crate) fn hypothetical(o: &mut Obj, counts: &[i16; CounterKind::COUNT]) {
    o.clear_counters();
    o.put_path_add(counts);
}

/// Counters of a player's kinds (CR 122.1): poison, energy, experience.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerCounts {
    pub poison: u16,
    pub energy: u16,
    pub experience: u16,
}

/// Counters put on `player` by `by` (CR 122.6): an event replacement effects can change (CR 614.1a). A player who can't
/// get counters of a kind gets none (CR 101.2: Melira, Sylvok Outcast; Solemnity), and no replacement effect of the event
/// that doesn't happen applies.
pub fn put_on_player(cx: &mut Cx, player: PlayerId, counts: PlayerCounts, by: PlayerId) {
    let banned = |cx: &Cx, n: u16, kind| n > 0 && crate::static_rules::player_counters_banned(cx, cx.cards(), player, kind);
    let mut counts = counts;
    if banned(cx, counts.poison, PlayerCounter::Poison) {
        counts.poison = 0;
    }
    if banned(cx, counts.energy, PlayerCounter::Energy) {
        counts.energy = 0;
    }
    if banned(cx, counts.experience, PlayerCounter::Experience) {
        counts.experience = 0;
    }
    if counts == PlayerCounts::default() {
        return;
    }
    let counts = replace::player_counters(cx, player, by, counts);
    cx.st().update_player(player, |p| p.put_path_add(counts.poison, counts.energy, counts.experience));
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
        Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
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
