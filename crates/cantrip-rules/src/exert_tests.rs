//! Exert's rule procedure and active durations (CR 701.43a-c), isolated deterministic positions.
use crate::{Cx, Game, Setup, ops, turn};
use cantrip_api::*;
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static CREATURE: CardDef = CardDef { name: "Exert Probe", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
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
fn untap(g: &mut Game, active: PlayerId) {
    g.state.update_turn(|t| { t.active = active; t.set_step(Step::Untap); });
    turn::untap_step(&mut cx(g), active).unwrap();
}
#[test]
fn repeat_exertions_record_each_event_without_tapping_and_expire_together() {
    let (mut g, id) = game();
    ops::exert(&mut cx(&mut g), id, YOU);
    ops::exert(&mut cx(&mut g), id, YOU);
    assert!(!g.state.obj(id).tapped, "exert does not tap");
    let exertions = g.log.iter().filter(|r| matches!(r.event, Event::Exerted { permanent, player: YOU } if permanent == id)).count();
    assert_eq!(exertions, 2, "each exertion has an event");
    g.state.update_obj(id, |o| o.tapped = true);
    untap(&mut g, YOU);
    assert!(g.state.obj(id).tapped, "exert skips next untap");
    assert_eq!(g.state.obj(id).marks.exerted_by, 0, "all same-player exertions expire together");
    untap(&mut g, YOU);
    assert!(!g.state.obj(id).tapped, "following untap proceeds");
}
#[test]
fn stun_is_not_consumed_by_a_prohibited_untap_but_replaces_a_later_untap() {
    let (mut g, id) = game();
    g.state.update_obj(id, |o| o.tapped = true);
    crate::counters::put(&mut cx(&mut g), id, CounterKind::Stun, 1, YOU, crate::counters::How::Effect);
    ops::exert(&mut cx(&mut g), id, YOU);
    untap(&mut g, YOU);
    assert_eq!(g.state.obj(id).counters(CounterKind::Stun), 1, "prohibited untap preserves stun");
    untap(&mut g, YOU);
    assert_eq!(g.state.obj(id).counters(CounterKind::Stun), 0, "next untap consumes stun");
    assert!(g.state.obj(id).tapped, "stun replaces untap");
    untap(&mut g, YOU);
    assert!(!g.state.obj(id).tapped, "untap after stun succeeds");
}
#[test]
fn an_unrelated_untap_preserves_exert_and_an_already_untapped_next_step_expires_it() {
    let (mut g, id) = game();
    g.state.update_obj(id, |o| o.tapped = true);
    ops::exert(&mut cx(&mut g), id, YOU);
    ops::untap(&mut cx(&mut g), id);
    assert!(!g.state.obj(id).tapped, "exert permits unrelated untaps");
    assert_eq!(g.state.obj(id).marks.exerted_by, 1, "unrelated untap retains exert");
    untap(&mut g, YOU);
    assert_eq!(g.state.obj(id).marks.exerted_by, 0, "untapped next step expires exert");
    g.state.update_obj(id, |o| o.tapped = true);
    untap(&mut g, YOU);
    assert!(!g.state.obj(id).tapped);
}
#[test]
fn different_players_exertions_keep_independent_expiry_through_control_changes() {
    let (mut g, id) = game();
    ops::exert(&mut cx(&mut g), id, YOU);
    g.state.update_obj(id, |o| { o.controller = THEM; o.tapped = true; });
    ops::exert(&mut cx(&mut g), id, THEM);
    assert_eq!(g.state.obj(id).marks.exerted_by, 3, "both players exert prohibitions retained");
    untap(&mut g, YOU);
    assert_eq!(g.state.obj(id).marks.exerted_by, 2, "only active player's exert expires");
    assert!(g.state.obj(id).tapped);
    untap(&mut g, THEM);
    assert!(g.state.obj(id).tapped, "new controller's exert still prohibits untap");
    assert_eq!(g.state.obj(id).marks.exerted_by, 0);
    untap(&mut g, THEM);
    assert!(!g.state.obj(id).tapped);
}
#[test]
fn a_stolen_exerted_creature_untaps_for_its_owner_before_the_exerting_players_step() {
    let (mut g, id) = game();
    ops::exert(&mut cx(&mut g), id, YOU);
    g.state.update_obj(id, |o| { o.controller = THEM; o.tapped = true; });
    untap(&mut g, THEM);
    assert!(!g.state.obj(id).tapped, "owner's untap is not the exerting player's untap");
    assert_eq!(g.state.obj(id).marks.exerted_by, 1);
    untap(&mut g, YOU);
    assert_eq!(g.state.obj(id).marks.exerted_by, 0);
}
#[test]
fn foreign_permanent_exert_uses_the_doer_and_zone_changes_clear_the_prohibition() {
    let (mut g, id) = game();
    g.state.update_obj(id, |o| o.controller = THEM);
    let mut context = cx(&mut g);
    context.doer = Some(YOU);
    cantrip_api::Game::exert(&mut context, id);
    assert_eq!(g.state.obj(id).marks.exerted_by, 1, "exert duration belongs to player performing it");
    let exiled = ops::exile_all(&mut cx(&mut g), &[id])[0];
    let events = g.log.iter().filter(|r| matches!(r.event, Event::Exerted { .. })).count();
    ops::exert(&mut cx(&mut g), exiled, YOU);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Exerted { .. })).count(), events, "nonbattlefield exert is prohibited");
    let new = ops::put_onto_battlefield_with(&mut cx(&mut g), &[(exiled, THEM, false)], ZoneReason::Other, &[]).unwrap()[0];
    assert_ne!(new, id, "new object identity");
    assert_eq!(g.state.obj(new).marks.exerted_by, 0, "new object has no exert duration");
}


// Independent review counterexamples retained as regression tests.
#[test]
fn review_independent_phased_out_exert_expires_during_the_exerting_players_step() {
    let (mut g, id) = game();
    ops::exert(&mut cx(&mut g), id, YOU);
    g.state.update_obj(id, |o| { o.controller = THEM; o.tapped = true; });
    crate::phasing::phase_out(&mut cx(&mut g), &[id], None, false);
    crate::phasing::untap_step(&mut cx(&mut g), YOU);
    assert!(g.state.obj(id).zone.is(ZoneKind::PhasedOut), "positive control: creature stays phased out");
    untap(&mut g, YOU);
    assert_eq!(g.state.obj(id).marks.exerted_by, 0, "CR 701.43a-b/702.26f: exert expires while phased out");
    crate::phasing::untap_step(&mut cx(&mut g), THEM);
    assert!(g.state.obj(id).zone.is(ZoneKind::Battlefield));
    g.state.update_obj(id, |o| o.controller = YOU);
    untap(&mut g, YOU);
    assert!(!g.state.obj(id).tapped, "returning cannot resurrect expired exert");
}

#[test]
fn review_independent_three_players_keep_each_others_pending_exertions() {
    let table = Box::leak(Box::new(CardTable::new([&CREATURE])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let id = g.state.push_obj(Obj::new(table.id_of(&CREATURE), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    ops::exert(&mut cx(&mut g), id, YOU);
    ops::exert(&mut cx(&mut g), id, THEM);
    ops::exert(&mut cx(&mut g), id, PlayerId(2));
    assert_eq!(g.state.obj(id).marks.exerted_by, 7, "three distinct players retain their exertions");
    untap(&mut g, THEM);
    assert_eq!(g.state.obj(id).marks.exerted_by, 5, "expiry clears only one player's bit");
    untap(&mut g, YOU);
    assert_eq!(g.state.obj(id).marks.exerted_by, 4);
    untap(&mut g, PlayerId(2));
    assert_eq!(g.state.obj(id).marks.exerted_by, 0);
}

#[test]
fn review_independent_doer_differs_from_controller_and_untap_does_not_erase_history() {
    let (mut g, id) = game();
    g.state.update_obj(id, |o| { o.controller = THEM; o.tapped = true; });
    let mut context = cx(&mut g);
    context.doer = Some(YOU);
    cantrip_api::Game::exert(&mut context, id);
    assert_eq!(g.state.obj(id).marks.exerted_by, 1, "duration uses doer rather than current controller");
    ops::untap(&mut cx(&mut g), id);
    assert!(!g.state.obj(id).tapped, "positive unrelated untap");
    assert_eq!(g.state.obj(id).marks.exerted_by, 1, "unrelated untap preserves pending exert");
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Exerted { permanent, player: YOU } if permanent == id)).count(), 1, "untap preserves one exact exert event");
}


#[test]
fn phased_out_three_player_exert_expiry_undo_redo_keeps_other_prohibitions_and_stun() {
    let table = Box::leak(Box::new(CardTable::new([&CREATURE])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let id = g.state.push_obj(Obj::new(table.id_of(&CREATURE), THEM, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.update_obj(id, |o| o.tapped = true);
    for player in [YOU, THEM, PlayerId(2)] { ops::exert(&mut cx(&mut g), id, player); }
    crate::counters::put(&mut cx(&mut g), id, CounterKind::Stun, 1, THEM, crate::counters::How::Effect);
    crate::phasing::phase_out(&mut cx(&mut g), &[id], None, false);
    let before = g.at;
    let mark = (g.state.mark(), g.log.len());
    g.tree.close(before, mark);
    let child = g.tree.add(before, (0, YOU, Answer::Yes), mark, &[]);
    untap(&mut g, YOU);
    g.at = child;
    assert!(g.state.obj(id).zone.is(ZoneKind::PhasedOut), "expiry does not phase the object in");
    assert_eq!(g.state.obj(id).marks.exerted_by, 6, "only exerting player's next step expires");
    assert_eq!(g.state.obj(id).counters(CounterKind::Stun), 1, "phased-out expiry never attempts an untap");
    g.goto(before);
    assert_eq!(g.state.obj(id).marks.exerted_by, 7, "tree undo restores all three prohibitions");
    g.goto(child);
    assert_eq!(g.state.obj(id).marks.exerted_by, 6, "tree redo restores exact one-player expiry");
    untap(&mut g, PlayerId(2));
    assert_eq!(g.state.obj(id).marks.exerted_by, 2, "other phased-out duration expires independently");
    crate::phasing::untap_step(&mut cx(&mut g), THEM);
    assert!(g.state.obj(id).zone.is(ZoneKind::Battlefield));
    untap(&mut g, THEM);
    assert!(g.state.obj(id).tapped, "remaining prohibition prevents returning object's first untap");
    assert_eq!(g.state.obj(id).counters(CounterKind::Stun), 1, "prohibited return keeps stun");
    untap(&mut g, THEM);
    assert!(g.state.obj(id).tapped);
    assert_eq!(g.state.obj(id).counters(CounterKind::Stun), 0, "later allowed untap consumes stun exactly once");
    untap(&mut g, THEM);
    assert!(!g.state.obj(id).tapped);
}
