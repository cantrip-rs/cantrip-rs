//! Simultaneous untap legality, stun replacements and shared printed recipes.
// The first two probes were independently authored during the blocked review.
use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;
const YOU: PlayerId = PlayerId(0);
static CREATURE: CardDef = CardDef { name: "Untap History Probe", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
fn game() -> (Game, ObjectId) {
    let table = Box::leak(Box::new(CardTable::new([&CREATURE])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let id = g.state.push_obj(Obj::new(table.id_of(&CREATURE), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    (g, id)
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}
fn review_gate(q: &dyn Query, me: &Asker, id: ObjectId) -> bool {
    id != me.source && q.state().obj(me.source).tapped && q.chars(id).is(CardType::Creature)
}
static REVIEW_GATE: CardDef = CardDef {
    name: "Reviewer static untap restriction",
    cant_become_untapped: &[review_gate],
    ..CREATURE
};
#[test]
fn review_independent_simultaneous_untap_uses_pre_event_prohibitions() {
    for gate_first in [true, false] {
        let table = Box::leak(Box::new(CardTable::new([&CREATURE, &REVIEW_GATE])));
        let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let mut put = |def: &'static CardDef| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id);
            g.state.update_obj(id, |o| o.tapped = true);
            id
        };
        let source = put(&CREATURE);
        let (gate, other) = if gate_first { (put(&REVIEW_GATE), put(&CREATURE)) } else {
            let other = put(&CREATURE); (put(&REVIEW_GATE), other)
        };
        ops::note_statics(&mut cx(&mut g), gate);
        let item = crate::actions::stack_item(StackKind::Triggered, source, AbilityRef::spell(table.id_of(&CREATURE)), YOU, &[]);
        cantrip_mechanics::effects::untap_other_creatures_you_control(&mut cx(&mut g), &item).unwrap();
        assert!(!g.state.obj(gate).tapped, "positive control: restriction source itself can untap");
        assert!(g.state.obj(other).tapped, "simultaneous untap must honor pre-event prohibition independent of battlefield order");
        assert!(g.state.obj(source).tapped, "excluded source stays tapped");
    }
}

#[test]
fn review_independent_shared_untap_honors_stun_bans_and_source_exclusion() {
    let (mut g, source) = game();
    g.state.update_obj(source, |o| o.tapped = true);
    let mut ids = Vec::new();
    for _ in 0..3 {
        let id = g.state.push_obj(Obj::new(g.cards.id_of(&CREATURE), YOU, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id);
        g.state.update_obj(id, |o| o.tapped = true);
        ids.push(id);
    }
    crate::counters::put(&mut cx(&mut g), ids[0], CounterKind::Stun, 2, YOU, crate::counters::How::Effect);
    crate::counters::put(&mut cx(&mut g), ids[1], CounterKind::Stun, 1, YOU, crate::counters::How::Effect);
    cantrip_api::Game::add_effect(&mut cx(&mut g), source, ids[1], EffectKind::CantBecomeUntapped, Duration::Indefinite);
    let item = crate::actions::stack_item(StackKind::Triggered, source, AbilityRef::spell(g.cards.id_of(&CREATURE)), YOU, &[]);
    cantrip_mechanics::effects::untap_other_creatures_you_control(&mut cx(&mut g), &item).unwrap();
    assert_eq!(g.state.obj(ids[0]).counters(CounterKind::Stun), 1, "one simultaneous untap removes exactly one stun");
    assert!(g.state.obj(ids[0]).tapped, "stun replaces shared untap");
    assert_eq!(g.state.obj(ids[1]).counters(CounterKind::Stun), 1, "prohibited untap leaves stun untouched");
    assert!(g.state.obj(ids[1]).tapped, "cant become untapped applies");
    assert!(!g.state.obj(ids[2]).tapped, "positive control: ordinary peer untaps");
    assert!(g.state.obj(source).tapped, "source remains excluded");
}

fn stun_gate(q: &dyn Query, me: &Asker, id: ObjectId) -> bool {
    id != me.source && q.state().obj(me.source).counters(CounterKind::Stun) > 0
}
static STUN_GATE: CardDef = CardDef { name: "Stun untap restriction probe", cant_become_untapped: &[stun_gate], ..CREATURE };

#[test]
fn simultaneous_stun_replacement_is_chosen_before_another_untap_and_duplicates_count_once() {
    for gate_first in [true, false] {
        let table = Box::leak(Box::new(CardTable::new([&CREATURE, &STUN_GATE])));
        let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let mut put = |def: &'static CardDef| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id);
            g.state.update_obj(id, |o| o.tapped = true);
            id
        };
        let gate = put(&STUN_GATE);
        let other = put(&CREATURE);
        ops::note_statics(&mut cx(&mut g), gate);
        crate::counters::put(&mut cx(&mut g), gate, CounterKind::Stun, 1, YOU, crate::counters::How::Effect);
        let ids = if gate_first { [gate, other, gate] } else { [other, gate, gate] };
        cantrip_api::Game::untap_together(&mut cx(&mut g), &ids);
        assert!(g.state.obj(other).tapped, "a stun replacement cannot change another object's pre-event untap prohibition");
        assert!(g.state.obj(gate).tapped, "one intrinsic stun replacement leaves source tapped");
        assert_eq!(g.state.obj(gate).counters(CounterKind::Stun), 0);
        cantrip_api::Game::untap_together(&mut cx(&mut g), &[other, gate]);
        assert!(!g.state.obj(other).tapped, "separate later event uses the changed restriction state");
        assert!(!g.state.obj(gate).tapped);
    }
    let (mut g, id) = game();
    g.state.update_obj(id, |o| o.tapped = true);
    crate::counters::put(&mut cx(&mut g), id, CounterKind::Stun, 2, YOU, crate::counters::How::Effect);
    cantrip_api::Game::untap_together(&mut cx(&mut g), &[id, id]);
    assert_eq!(g.state.obj(id).counters(CounterKind::Stun), 1, "duplicate selection removes one counter at most");
    assert!(g.state.obj(id).tapped);
}

#[test]
fn untap_step_uses_pre_event_restrictions_in_both_battlefield_orders() {
    for gate_first in [true, false] {
        let table = Box::leak(Box::new(CardTable::new([&CREATURE, &REVIEW_GATE])));
        let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let mut put = |def: &'static CardDef| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id);
            g.state.update_obj(id, |o| o.tapped = true);
            id
        };
        let (gate, other) = if gate_first { (put(&REVIEW_GATE), put(&CREATURE)) } else {
            let other = put(&CREATURE); (put(&REVIEW_GATE), other)
        };
        ops::note_statics(&mut cx(&mut g), gate);
        crate::turn::untap_step(&mut cx(&mut g), YOU).unwrap();
        assert!(!g.state.obj(gate).tapped, "untap-step restriction source is allowed to untap");
        assert!(g.state.obj(other).tapped, "same-step source untap never enables a prohibited permanent");
        crate::turn::untap_step(&mut cx(&mut g), YOU).unwrap();
        assert!(!g.state.obj(other).tapped, "next event is a positive control for the relaxed restriction");
    }
}

#[test]
fn simultaneous_untap_ignores_stale_phased_and_untapped_objects_and_survives_tree_undo_redo() {
    let (mut g, ordinary) = game();
    let mut put = || {
        let id = g.state.push_obj(Obj::new(g.cards.id_of(&CREATURE), YOU, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let stunned = put(); let phased = put(); let departed = put(); let untapped = put();
    for id in [ordinary, stunned, phased, departed] { g.state.update_obj(id, |o| o.tapped = true); }
    for id in [stunned, phased, untapped] { crate::counters::put(&mut cx(&mut g), id, CounterKind::Stun, 2, YOU, crate::counters::How::Effect); }
    crate::phasing::phase_out(&mut cx(&mut g), &[phased], None, false);
    let exiled = ops::exile_all(&mut cx(&mut g), &[departed])[0];
    let before = g.at; let mark = (g.state.mark(), g.log.len());
    g.tree.close(before, mark);
    let child = g.tree.add(before, (0, YOU, Answer::Yes), mark, &[]);
    ops::untap_together(&mut cx(&mut g), &[ordinary, stunned, stunned, phased, departed, exiled, untapped]);
    g.at = child;
    assert!(!g.state.obj(ordinary).tapped, "ordinary permanent untaps");
    assert_eq!(g.state.obj(stunned).counters(CounterKind::Stun), 1);
    assert!(g.state.obj(stunned).tapped);
    assert!(g.state.obj(phased).tapped); assert_eq!(g.state.obj(phased).counters(CounterKind::Stun), 2);
    assert!(g.state.obj(departed).tapped, "last-known departed row is unchanged");
    assert_eq!(g.state.obj(untapped).counters(CounterKind::Stun), 2, "already untapped permanent spends no counter");
    g.goto(before);
    assert!(g.state.obj(ordinary).tapped);
    assert_eq!(g.state.obj(stunned).counters(CounterKind::Stun), 2, "undo restores replacement");
    g.goto(child);
    assert!(!g.state.obj(ordinary).tapped);
    assert_eq!(g.state.obj(stunned).counters(CounterKind::Stun), 1, "redo restores one simultaneous replacement");
    assert!(g.state.obj(phased).zone.is(ZoneKind::PhasedOut));
}

#[test]
fn attacked_this_turn_recipe_uses_pre_event_legality_across_controllers_and_orders() {
    for gate_first in [true, false] {
        let table = Box::leak(Box::new(CardTable::new([&CREATURE, &REVIEW_GATE])));
        let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let mut put = |def: &'static CardDef, controller| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), controller, Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id);
            g.state.update_obj(id, |o| o.tapped = true);
            id
        };
        let them = PlayerId(1);
        let (gate, other) = if gate_first { (put(&REVIEW_GATE, YOU), put(&CREATURE, them)) } else {
            let other = put(&CREATURE, them); (put(&REVIEW_GATE, YOU), other)
        };
        let nonattacker = put(&CREATURE, YOU);
        ops::note_statics(&mut cx(&mut g), gate);
        for id in [gate, other] {
            let chars = g.chars(id);
            let row = ops::last_known_from(&mut cx(&mut g), id, &chars);
            let known = g.state.last_known_push(row);
            cx(&mut g).record(Event::Attacks { attacker: id, defender: Target::Player(them), known, defending: them });
        }
        cantrip_mechanics::effects::untap_creatures_that_attacked_this_turn(&mut cx(&mut g));
        assert!(!g.state.obj(gate).tapped, "Own the Day/World at War recipe selects the prior attacker");
        assert!(g.state.obj(other).tapped, "simultaneous attacked-creature recipe preserves pre-event prohibition");
        assert!(g.state.obj(nonattacker).tapped, "current nonattacker stays excluded");
        cantrip_mechanics::effects::untap_creatures_that_attacked_this_turn(&mut cx(&mut g));
        assert!(!g.state.obj(other).tapped, "separate later event untaps the same opponent-controlled prior attacker");
        assert!(g.state.obj(nonattacker).tapped);
    }
}
