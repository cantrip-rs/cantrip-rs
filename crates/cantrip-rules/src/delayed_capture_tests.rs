//! Independent delayed references (CR 603.7c-e): Infinite Authority, Gift of Immortality and Next of Kin.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_api::Game as Rules;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None }
}
fn capture(g: &mut dyn Rules, when: DelayedWhen, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target, members: &[ObjectId]) {
    g.delayed_trigger_with(when, source, controller, ability, subject, members);
}
fn at(step: Step) -> DelayedWhen { DelayedWhen::Step { step, on_turn_of: None } }
fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
fn destroy_then_schedule(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let others = g.event_objects(item).to_vec();
    let destroyed = g.destroy_all(&others);
    if !destroyed.is_empty() {
        let next = AbilityRef::delayed(item.ability.card, 1);
        capture(g, at(Step::End), item.object, item.controller, next, item.subject, &destroyed);
    }
    Ok(())
}
fn counter_first(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(first) = item.subject.object() { g.add_counters(first, CounterKind::PlusOne, 1); }
    Ok(())
}
fn return_aura(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(host) = item.subject.object() {
        for aura in g.event_objects(item).to_vec() { g.put_onto_battlefield_attached(aura, g.obj(aura).owner, host)?; }
    }
    Ok(())
}
static AUTHORITY: CardDef = CardDef {
    name: "Infinite Authority capture fixture", types: TypeSet::of(&[CardType::Enchantment]), subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(cantrip_mechanics::targets::CREATURE),
    delayed: &[
        delayed_trigger(destroy_then_schedule, "Destroy the other creature at end of combat."),
        delayed_trigger(counter_first, "At the beginning of the next end step, put a +1/+1 counter on the first creature."),
    ], ..CardDef::BLANK
};
static AURA: CardDef = CardDef {
    name: "Gift of Immortality capture fixture", types: TypeSet::of(&[CardType::Enchantment]), subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(cantrip_mechanics::targets::CREATURE),
    delayed: &[delayed_trigger(return_aura, "Return this card to the battlefield attached to that creature at the beginning of the next end step.")],
    ..CardDef::BLANK
};
static BODY: CardDef = CardDef { name: "Delayed capture creature", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(4), ..CardDef::BLANK };
fn watched_tap(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    if matches!(event, Event::Tapped { permanent } if *permanent == asker.source)
        && q.words(asker.source).subtype(Subtype::Goblin) == Subtype::Elf {
        Fired::about(Target::Player(THEM))
    } else { None }
}
fn creator_is_instant(q: &dyn Query, asker: &Asker) -> bool {
    q.chars(asker.source).is(CardType::Instant)
        && q.words(asker.source).subtype(Subtype::Goblin) == Subtype::Elf
}
static WATCHER: CardDef = CardDef {
    name: "Delayed watched source", types: TypeSet::of(&[CardType::Instant]),
    delayed: &[
        triggered(cantrip_mechanics::triggers::THIS_DIES, nothing, "When that creature dies this turn."),
        triggered(when_asked(EventMask::TAPPED, watched_tap), nothing, "Whenever it is tapped this turn.").condition(creator_is_instant).only_if(creator_is_instant),
    ], ..CardDef::BLANK
};
static PLAIN: CardDef = CardDef { name: "Delayed capture control", delayed: &[delayed_trigger(nothing, "Do nothing.")], ..CardDef::BLANK };
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&AUTHORITY, &AURA, &BODY, &PLAIN, &WATCHER])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![]; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.number = 3; t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    for p in [YOU, THEM] { g.state.update_player(p, |s| s.drew_from_empty = false); }
    g
}
fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    let stamp = g.state.timestamp(); g.state.update_obj(id, |o| o.timestamp = stamp); g.state.zone_push(zone, id); id
}
fn step(g: &mut Game, step: Step) {
    g.state.update_turn(|t| t.set_step(step)); crate::settle::step_began(&mut cx(g), step);
}
fn resolve(g: &mut Game) {
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority)); g.state.frame_push(Frame::new(FrameKind::Settle));
    for _ in 0..40 {
        g.run();
        if g.state.stack.is_empty() && g.state.triggers.is_empty() { return; }
        let r = g.pending().expect("priority while the delayed ability is on the stack").clone();
        assert!(matches!(r.kind, RequestKind::Priority { .. }));
        g.answer(r.id, r.player, Answer::Pass).unwrap();
    }
    panic!("delayed ability did not resolve");
}
fn schedule(g: &mut Game, source: ObjectId, host: ObjectId, others: &[ObjectId]) {
    assert!(cx(g).attach(source, host));
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    capture(&mut cx(g), at(Step::EndOfCombat), source, YOU, ability, Target::Object(host), others);
}

#[test]
fn delayed_capture_retains_both_typed_roles_before_and_after_triggering() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    schedule(&mut g, source, host, &[other]);
    let d = g.state.delayed[0]; assert_eq!(d.subject, Target::Object(host)); assert_eq!(g.state.members(d.members), &[other]);
    step(&mut g, Step::EndOfCombat);
    let t = g.state.triggers[0]; assert_eq!(t.subject, Target::Object(host)); assert_eq!(g.state.members(t.members), &[other]);
    assert_eq!(t.source, source); assert_eq!(t.controller, YOU); assert!(g.state.delayed.is_empty());
}
#[test]
fn delayed_capture_infinite_nested_end_step_keeps_first_creature_distinct() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    schedule(&mut g, source, host, &[other]); step(&mut g, Step::EndOfCombat); resolve(&mut g);
    assert!(g.state.obj(other).became.is_some()); assert_eq!(g.state.delayed.len(), 1);
    step(&mut g, Step::End); resolve(&mut g);
    assert_eq!(g.state.obj(host).counters(CounterKind::PlusOne), 1);
    assert_eq!(g.state.obj(source).counters(CounterKind::PlusOne), 0);
}
fn gift(g: &mut Game) -> (ObjectId, ObjectId, ObjectId) {
    let old_host = put(g, &BODY, Zone::BATTLEFIELD); let source = put(g, &AURA, Zone::BATTLEFIELD);
    assert!(cx(g).attach(source, old_host)); cx(g).destroy_all(&[old_host]);
    crate::settle::state_based_actions_alone(&mut cx(g)).unwrap();
    let aura_card = g.state.obj(source).became.expect("the unattached Aura moved to the graveyard");
    let host_card = g.state.obj(old_host).became.unwrap();
    let host = cx(g).put_onto_battlefield(host_card, YOU, false).unwrap();
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    capture(&mut cx(g), at(Step::End), source, YOU, ability, Target::Object(host), &[aura_card]);
    (source, aura_card, host)
}
#[test]
fn delayed_capture_gift_returns_the_graveyard_aura_to_the_returned_creature() {
    let mut g = game(); let (source, aura_card, host) = gift(&mut g);
    step(&mut g, Step::End); resolve(&mut g);
    let returned = g.state.obj(aura_card).became.expect("the captured Aura card returned");
    assert_eq!(g.state.obj(returned).attached_to, Some(host)); assert_ne!(source, aura_card); assert_ne!(aura_card, returned);
}
#[test]
fn delayed_capture_legacy_constructors_keep_their_single_role() {
    let mut g = game(); let source = put(&mut g, &PLAIN, Zone::BATTLEFIELD); let host = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    cx(&mut g).delayed_trigger(at(Step::End), source, YOU, ability, Target::Object(host));
    cx(&mut g).delayed_trigger_about(at(Step::End), source, YOU, ability, &[host]);
    step(&mut g, Step::End); assert_eq!(g.state.triggers.len(), 2);
    assert_eq!(g.state.triggers[0].subject, Target::Object(host)); assert!(g.state.members(g.state.triggers[0].members).is_empty());
    assert_eq!(g.state.triggers[1].subject, Target::None); assert_eq!(g.state.members(g.state.triggers[1].members), &[host]);
}

#[test]
fn delayed_capture_moving_the_aura_does_not_replace_the_original_host() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let new_host = put(&mut g, &BODY, Zone::BATTLEFIELD);
    schedule(&mut g, source, host, &[other]); assert!(cx(&mut g).attach(source, new_host));
    step(&mut g, Step::EndOfCombat); resolve(&mut g); step(&mut g, Step::End); resolve(&mut g);
    assert_eq!(g.state.obj(host).counters(CounterKind::PlusOne), 1);
    assert_eq!(g.state.obj(new_host).counters(CounterKind::PlusOne), 0);
}

#[test]
fn delayed_capture_failed_destruction_does_not_create_the_nested_delay() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    cx(&mut g).add_effect(source, other, EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible])), Duration::EndOfTurn);
    schedule(&mut g, source, host, &[other]); step(&mut g, Step::EndOfCombat); resolve(&mut g);
    assert!(g.state.obj(other).became.is_none()); assert!(g.state.delayed.is_empty());
    step(&mut g, Step::End); resolve(&mut g); assert_eq!(g.state.obj(host).counters(CounterKind::PlusOne), 0);
}

#[test]
fn delayed_capture_source_leaving_and_control_changing_do_not_change_the_capture() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    schedule(&mut g, source, host, &[other]); g.state.update_obj(source, |o| o.controller = THEM); cx(&mut g).exile_all(&[source]);
    step(&mut g, Step::EndOfCombat);
    assert_eq!(g.state.triggers[0].source, source); assert_eq!(g.state.triggers[0].controller, YOU);
    resolve(&mut g); step(&mut g, Step::End); resolve(&mut g);
    assert_eq!(g.state.obj(host).counters(CounterKind::PlusOne), 1);
}

#[test]
fn delayed_capture_gift_does_not_follow_a_host_that_left_and_returned() {
    let mut g = game(); let (_, aura_card, host) = gift(&mut g);
    let exiled = cx(&mut g).exile_all(&[host])[0]; let new_host = cx(&mut g).put_onto_battlefield(exiled, YOU, false).unwrap();
    assert_ne!(host, new_host); step(&mut g, Step::End); resolve(&mut g);
    assert!(g.state.obj(aura_card).became.is_none()); assert!(g.state.obj(aura_card).zone.is(ZoneKind::Graveyard));
}

#[test]
fn delayed_capture_gift_does_not_follow_a_card_that_left_its_graveyard() {
    let mut g = game(); let (_, aura_card, _) = gift(&mut g);
    let exiled = cx(&mut g).exile_all(&[aura_card])[0];
    let new_card = crate::ops::move_to(&mut cx(&mut g), exiled, Zone::graveyard(YOU));
    assert_ne!(aura_card, new_card);
    step(&mut g, Step::End); resolve(&mut g);
    assert!(g.state.obj(new_card).became.is_none()); assert!(g.state.obj(new_card).zone.is(ZoneKind::Graveyard));
}

#[test]
fn delayed_capture_next_of_kin_uses_the_new_creature_instead_of_the_dead_host() {
    let mut g = game(); let (source, aura_card, returned_original) = gift(&mut g);
    g.state.delayed_remove(0);
    let selected = put(&mut g, &BODY, Zone::hand(YOU)); let chosen = cx(&mut g).put_onto_battlefield(selected, YOU, false).unwrap();
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    capture(&mut cx(&mut g), at(Step::End), source, YOU, ability, Target::Object(chosen), &[aura_card]);
    step(&mut g, Step::End); resolve(&mut g);
    let returned = g.state.obj(aura_card).became.unwrap();
    assert_eq!(g.state.obj(returned).attached_to, Some(chosen)); assert_ne!(chosen, returned_original);
}

#[test]
fn delayed_capture_concurrent_abilities_keep_their_subjects_and_lists_separate() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let a = put(&mut g, &BODY, Zone::BATTLEFIELD); let b = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let x = put(&mut g, &BODY, Zone::BATTLEFIELD); let y = put(&mut g, &BODY, Zone::BATTLEFIELD);
    schedule(&mut g, source, a, &[x]); schedule(&mut g, source, b, &[y]); step(&mut g, Step::EndOfCombat);
    assert_eq!(g.state.triggers[0].subject, Target::Object(a)); assert_eq!(g.state.members(g.state.triggers[0].members), &[x]);
    assert_eq!(g.state.triggers[1].subject, Target::Object(b)); assert_eq!(g.state.members(g.state.triggers[1].members), &[y]);
}

#[test]
fn delayed_capture_an_empty_list_and_a_player_subject_are_not_conflated() {
    let mut g = game(); let source = put(&mut g, &PLAIN, Zone::BATTLEFIELD);
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    capture(&mut cx(&mut g), at(Step::End), source, YOU, ability, Target::Player(THEM), &[]);
    step(&mut g, Step::End);
    assert_eq!(g.state.triggers[0].subject, Target::Player(THEM)); assert!(g.state.members(g.state.triggers[0].members).is_empty());
}

#[test]
fn delayed_capture_clone_and_journal_rollback_preserve_both_roles() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    schedule(&mut g, source, host, &[other]); let before = g.state.clone(); let mark = g.state.mark(); let log = g.log.len();
    step(&mut g, Step::EndOfCombat); let after = g.state.clone(); assert!(after.delayed.is_empty());
    g.state.rollback(mark); g.log.truncate(log); assert!(g.state == before);
    step(&mut g, Step::EndOfCombat); assert!(g.state == after);
    let mut fork = g.fork_for_search(); resolve(&mut fork); step(&mut fork, Step::End); resolve(&mut fork);
    assert_eq!(fork.state.obj(host).counters(CounterKind::PlusOne), 1);
    assert_eq!(g.state.obj(host).counters(CounterKind::PlusOne), 0);
}

#[test]
fn delayed_capture_event_variants_are_rejected_before_mutating_the_state() {
    let mut g = game(); let source = put(&mut g, &PLAIN, Zone::BATTLEFIELD);
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0); let before = g.state.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        capture(&mut cx(&mut g), DelayedWhen::ThisTurn, source, YOU, ability, Target::Player(THEM), &[source]);
    }));
    assert!(result.is_err()); assert!(g.state == before);
}

#[test]
fn delayed_capture_keeps_the_creating_abilitys_acquired_text_and_words() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    assert!(cx(&mut g).attach(source, host));
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    let mut item = crate::actions::stack_item(StackKind::Triggered, source, ability, YOU, &[]);
    item.acquired = Acquired { by: AcquiredBy::Grant, stamp: g.state.timestamp(), part: 0 };
    item.words = Words::NONE.changed(Word::Subtype(Subtype::Goblin), Word::Subtype(Subtype::Elf));
    g.state.stack_push(item); let serial = g.state.stack.last().unwrap().serial;
    let frame = g.state.agenda.len(); g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.own_text.set(true);
    let mut run = Cx { frame, ..cx(&mut g) };
    capture(&mut run, at(Step::End), source, YOU, ability, Target::Object(host), &[other]);
    let d = run.s().delayed[0];
    assert_eq!(d.source, source); assert_eq!(d.controller, YOU); assert_eq!(d.acquired, item.acquired);
    assert_eq!(d.words, Some(item.words)); assert_eq!(d.since, run.g.log.len() as u32);
    assert_eq!(d.subject, Target::Object(host)); assert_eq!(run.s().members(d.members), &[other]);
}

#[test]
fn delayed_capture_stack_copy_and_tree_goto_keep_both_roles() {
    let mut g = game(); let source = put(&mut g, &AUTHORITY, Zone::BATTLEFIELD);
    let host = put(&mut g, &BODY, Zone::BATTLEFIELD); let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    schedule(&mut g, source, host, &[other]); step(&mut g, Step::EndOfCombat);
    g.state.frame_push(Frame::new(FrameKind::Priority)); g.state.frame_push(Frame::new(FrameKind::Settle)); g.run();
    assert_eq!(g.state.stack.len(), 1); let item = g.state.stack[0];
    cx(&mut g).copy_ability(Target::Ability(item.serial), YOU, false).unwrap().unwrap();
    assert_eq!(g.state.stack.len(), 2);
    for copy in &g.state.stack { assert_eq!(copy.subject, Target::Object(host)); assert_eq!(g.state.members(copy.members), &[other]); }
    // Hand over a forced position with both abilities already on the stack, before asking priority.
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority)); g.run();
    let mut fork = g.fork_for_search(); let root = fork.checkpoint(); let before = fork.state.clone(); let before_log = fork.log.clone();
    for _ in 0..12 {
        if fork.state.stack.is_empty() { break; }
        let r = fork.pending().unwrap().clone(); assert!(matches!(r.kind, RequestKind::Priority { .. }));
        fork.answer(r.id, r.player, Answer::Pass).unwrap();
    }
    assert!(fork.state.stack.is_empty()); assert_eq!(fork.state.delayed.len(), 1);
    assert_eq!(fork.log.iter().filter(|r| matches!(r.event, Event::Resolved { source: s, ability } if s == source && ability == item.ability)).count(), 2);
    let after = fork.checkpoint(); let after_state = fork.state.clone(); let after_log = fork.log.clone();
    fork.goto(root); assert!(fork.state == before); assert_eq!(fork.log, before_log);
    fork.goto(after); assert!(fork.state == after_state); assert_eq!(fork.log, after_log);
    assert_eq!(g.state.stack.len(), 2); assert!(g.state.obj(other).became.is_none());
}

#[test]
fn delayed_capture_each_step_retains_the_same_typed_roles_for_each_trigger() {
    let mut g = game(); let source = put(&mut g, &PLAIN, Zone::BATTLEFIELD); let host = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    capture(&mut cx(&mut g), DelayedWhen::EachStep { step: Step::End, on_turn_of: None }, source, YOU, ability, Target::Player(THEM), &[host]);
    step(&mut g, Step::End); step(&mut g, Step::End);
    assert_eq!(g.state.delayed.len(), 1); assert_eq!(g.state.triggers.len(), 2);
    for t in &g.state.triggers { assert_eq!(t.subject, Target::Player(THEM)); assert_eq!(g.state.members(t.members), &[host]); }
}

// The source is no longer on the stack; its original identity and the watched creature's last-known identity remain distinct.
#[test]
fn this_turn_watching_keeps_source_and_watched_death_separate() {
    let mut g = game();
    let source = put(&mut g, &WATCHER, Zone::graveyard(YOU));
    let watched = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    cx(&mut g).delayed_trigger(DelayedWhen::ThisTurnWatching(watched), source, YOU, ability, Target::None);
    cx(&mut g).destroy_all(&[other]);
    assert!(g.state.triggers.is_empty());
    cx(&mut g).destroy_all(&[watched]);
    assert_eq!(g.state.triggers.len(), 1);
    let trigger = g.state.triggers[0];
    assert_eq!(trigger.source, source);
    assert_eq!(trigger.subject, Target::Object(watched));
    assert_eq!(trigger.controller, YOU);
    assert!(g.state.obj(watched).became.is_some());
}

#[test]
fn this_turn_watching_keeps_event_payload_and_captured_words_until_cleanup() {
    let mut g = game();
    let source = put(&mut g, &WATCHER, Zone::BATTLEFIELD);
    let watched = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let other = put(&mut g, &BODY, Zone::BATTLEFIELD);
    let ability = AbilityRef::delayed(g.state.obj(source).card, 1);
    let mut item = crate::actions::stack_item(StackKind::Triggered, source, ability, YOU, &[]);
    item.words = Words::NONE.changed(Word::Subtype(Subtype::Goblin), Word::Subtype(Subtype::Elf));
    g.state.stack_push(item); let serial = g.state.stack.last().unwrap().serial;
    let frame = g.state.agenda.len(); g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.own_text.set(true);
    let mut run = Cx { frame, ..cx(&mut g) };
    run.delayed_trigger(DelayedWhen::ThisTurnWatching(watched), source, YOU, ability, Target::None);
    drop(run);
    cx(&mut g).exile_all(&[source]);
    cx(&mut g).emit(Event::Tapped { permanent: other });
    assert!(g.state.triggers.is_empty());
    cx(&mut g).emit(Event::Tapped { permanent: watched });
    cx(&mut g).emit(Event::Tapped { permanent: watched });
    assert_eq!(g.state.triggers.len(), 2);
    for t in &g.state.triggers {
        assert_eq!(t.source, source); assert_eq!(t.subject, Target::Player(THEM));
        assert_eq!(t.words, Some(item.words));
    }
    crate::turn::cleanup(&mut cx(&mut g)).unwrap();
    assert!(g.state.delayed.is_empty());
    cx(&mut g).emit(Event::Tapped { permanent: watched });
    assert_eq!(g.state.triggers.len(), 2);
}
