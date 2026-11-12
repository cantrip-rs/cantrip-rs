//! Typed noted values retain activation and linked-ability provenance.
use crate::{Cx, Game, Setup};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_mechanics::notes;
use cantrip_state::{Frame, FrameKind};
const A: PlayerId = PlayerId(0);
fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}
static FILL: CardDef = CardDef {
    name: "Note filler",
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};
static SOURCE: CardDef = CardDef {
    name: "Noted source",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        ability(
            Cost::mana(ManaCost {
                generic: 300,
                ..ManaCost::ZERO
            }),
            nothing,
            "Note the mana spent.",
        )
        .tracks_mana_spent(),
        ability(cost("{1}"), nothing, "Note a later payment.").tracks_mana_spent(),
        ability(cost("{1}"), nothing, "An ordinary untracked ability."),
        ability(Cost::NONE, nothing, "Note a zero payment.").tracks_mana_spent(),
    ],
    ..CardDef::BLANK
};
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: vec![],
        damage: None,
        asked_at: None,
        doer: Some(A),
        producing: None,
    }
}
fn board() -> (Game, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&FILL, &SOURCE])));
    let deck = vec![cards.id_of(&FILL); 30];
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![deck; 2],
            shuffle: false,
            ..Default::default()
        },
    );
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 2;
        t.active = A;
        t.priority = A;
        t.began = true;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let id = g
        .state
        .push_obj(Obj::new(cards.id_of(&SOURCE), A, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    (g, id)
}
fn pool(g: &mut Game, m: Mana, n: u32) {
    g.state
        .add_mana(A, m, n, ObjectId::NONE, None, ManaLasts::Step, false);
}
fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("request")
}
fn answer(g: &mut Game, a: Answer) {
    let r = g.pending().cloned().unwrap();
    g.answer(r.id, r.player, a).unwrap();
}
fn activate(g: &mut Game, id: ObjectId, index: u8) -> StackItem {
    if g.pending().is_some() {
        let at=g.state.agenda.len()-1;g.state.frame_remove(at);g.state.frame_push(Frame::new(FrameKind::Priority));
    }
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else {
        panic!("{:?}", r.kind)
    };
    let action = Action::of(
        options
            .iter()
            .find(|o| o.source == Some(id) && o.index == index)
            .expect("offered"),
    );
    answer(g, Answer::Act(action));
    for _ in 0..40 {
        let r = next(g);
        match &r.kind {
            RequestKind::PayCosts {
                announcing: false,
                options,
                ..
            } => {
                let o = [
                    ActionKind::AssignMana,
                    ActionKind::ConfirmCasting,
                    ActionKind::PayCost,
                ]
                .into_iter()
                .find_map(|k| options.iter().find(|o| o.kind == k))
                .expect("payable");
                answer(g, Answer::Act(Action::of(o)));
            }
            RequestKind::Priority { .. } => return *g.state.stack.last().unwrap(),
            other => panic!("{other:?}"),
        }
    }
    panic!("unfinished activation")
}
fn link(g: &mut Game, id: ObjectId) -> Linked {
    cx(g).text_linked(id)
}
fn read(g: &mut Game, by: Linked) -> ManaPool {
    notes::last_mana(&cx(g), by)
}
#[test]
fn activation_payment_keeps_full_counts_instead_of_the_narrow_history_tally() {
    let (mut g, s) = board();
    pool(&mut g, Mana::C, 300);
    let item = activate(&mut g, s, 0);
    let by = link(&mut g, s);
    assert!(
        g.events()
            .iter()
            .any(|r| matches!(r.event,Event::ManaSpent{mana,..} if mana.get(Mana::C)==255))
    );
    assert!(read(&mut g, by).is_empty(), "paying is not yet noting");
    notes::note_mana_spent(&mut cx(&mut g), &item, true);
    assert_eq!(read(&mut g, by).get(Mana::C), 300);
}
#[test]
fn a_later_activation_of_the_same_source_cannot_replace_this_activations_receipt() {
    let (mut g, s) = board();
    pool(&mut g, Mana::C, 300);
    let first = activate(&mut g, s, 0);
    pool(&mut g, Mana::R, 1);
    let second = activate(&mut g, s, 1);
    let by = link(&mut g, s);
    assert_ne!(first.serial, second.serial);
    notes::note_mana_spent(&mut cx(&mut g), &second, true);
    assert_eq!(read(&mut g, by).get(Mana::R), 1);
    notes::note_mana_spent(&mut cx(&mut g), &first, true);
    assert_eq!(read(&mut g, by), ManaPool([0, 0, 0, 0, 0, 300]));
}
#[test]
fn ordinary_activations_allocate_no_noted_value_receipts() {
    let (mut g, s) = board();
    pool(&mut g, Mana::G, 1);
    activate(&mut g, s, 2);
    assert!(g.state.noted_values.is_empty());
    assert!(
        !g.events()
            .iter()
            .any(|r| matches!(r.event, Event::ActivationManaPaid { .. }))
    );
}
#[test]
fn an_explicit_zero_payment_replaces_a_previous_nonzero_note() {
    let (mut g, s) = board();
    let by = link(&mut g, s);
    cx(&mut g).note_value(by, NotedValue::Mana(ManaPool([3, 0, 0, 0, 0, 0])));
    let item = activate(&mut g, s, 3);
    notes::note_mana_spent(&mut cx(&mut g), &item, true);
    assert!(read(&mut g, by).is_empty());
}
#[test]
fn copied_abilities_do_not_invent_an_activation_payment() {
    let (mut g, s) = board();
    pool(&mut g, Mana::C, 300);
    let mut item = activate(&mut g, s, 0);
    item.copied = true;
    let by = link(&mut g, s);
    notes::note_mana_spent(&mut cx(&mut g), &item, true);
    assert!(read(&mut g, by).is_empty());
}
#[test]
fn type_only_notes_keep_colorless_but_do_not_keep_the_paid_amount() {
    let (mut g, s) = board();
    pool(&mut g, Mana::C, 300);
    let item = activate(&mut g, s, 0);
    let by = link(&mut g, s);
    notes::note_mana_spent(&mut cx(&mut g), &item, false);
    assert_eq!(read(&mut g, by), ManaPool([0, 0, 0, 0, 0, 1]));
}
#[test]
fn notes_are_separate_for_text_acquisition_and_object_incarnation() {
    let (mut g, s) = board();
    let by = link(&mut g, s);
    let mut other_text = by;
    other_text.text = g.cards.id_of(&FILL);
    let mut grant = by;
    grant.acquired = Acquired {
        by: AcquiredBy::Grant,
        stamp: 42,
        part: 0,
    };
    cx(&mut g).note_value(by, NotedValue::Mana(ManaPool([1, 0, 0, 0, 0, 0])));
    cx(&mut g).note_value(other_text, NotedValue::Mana(ManaPool([0, 2, 0, 0, 0, 0])));
    cx(&mut g).note_value(grant, NotedValue::Mana(ManaPool([0, 0, 3, 0, 0, 0])));
    assert_eq!(read(&mut g, by).get(Mana::W), 1);
    assert_eq!(read(&mut g, other_text).get(Mana::U), 2);
    assert_eq!(read(&mut g, grant).get(Mana::B), 3);
    let new = cx(&mut g).move_to(s, Zone::hand(A));
    let mut new_by = by;
    new_by.object = new;
    assert!(read(&mut g, new_by).is_empty());
    assert_eq!(read(&mut g, by).get(Mana::W), 1);
}
#[test]
fn counter_snapshot_reads_the_old_object_after_exile_and_is_not_mutated_later() {
    let (mut g, s) = board();
    let by = link(&mut g, s);
    cx(&mut g).add_counters(s, CounterKind::Charge, 50000);
    cx(&mut g).add_counters(s, CounterKind::PlusOne, 2);
    let new = cx(&mut g).exile(s);
    assert_eq!(g.state.obj(new).counter_kinds().count(), 0);
    let counters = g.state.obj(s).counter_kinds().collect();
    cx(&mut g).note_value(by, NotedValue::Counters(counters));
    cx(&mut g).add_counters(new, CounterKind::Charge, 3);
    let NotedValue::Counters(c) = &g.state.noted_values.last().unwrap().value else {
        panic!()
    };
    assert!(c.contains(&(CounterKind::Charge, 50000)));
    assert!(c.contains(&(CounterKind::PlusOne, 2)));
}
#[test]
fn notes_of_other_kinds_do_not_hide_the_last_mana_note() {
    let (mut g, s) = board();
    let by = link(&mut g, s);
    cx(&mut g).note_value(by, NotedValue::Mana(ManaPool([0, 0, 0, 2, 0, 0])));
    cx(&mut g).note_value(by, NotedValue::Counters(vec![(CounterKind::Charge, 4)]));
    assert_eq!(read(&mut g, by).get(Mana::R), 2);
}
#[test]
fn note_storage_and_history_roll_back_together() {
    let (mut g, s) = board();
    let by = link(&mut g, s);
    let mark = g.state.mark();
    let log = g.log.len();
    cx(&mut g).note_value(by, NotedValue::Mana(ManaPool([0, 0, 0, 0, 7, 0])));
    assert_eq!(read(&mut g, by).get(Mana::G), 7);
    g.state.rollback(mark);
    g.log.truncate(log);
    assert!(g.state.noted_values.is_empty());
    assert!(read(&mut g, by).is_empty());
}
#[test]
fn notes_do_not_enlarge_hot_object_stack_or_event_records() {
    assert_eq!(std::mem::size_of::<Event>(), 28);
    assert_eq!(std::mem::size_of::<StackItem>(), 320);
    println!(
        "Event={} StackItem={} Obj={} NotedValueRecord={}",
        std::mem::size_of::<Event>(),
        std::mem::size_of::<StackItem>(),
        std::mem::size_of::<Obj>(),
        std::mem::size_of::<NotedValueRecord>()
    );
}
