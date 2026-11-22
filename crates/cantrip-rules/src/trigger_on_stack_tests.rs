//! A triggered ability going onto the stack is an event of its own (`Event::TriggerOnStack`) that records what caused it
//! (`TriggerCause`, CR 603.2, 603.3): asked of the abilities that listen for it, over test cards. An ability that triggers on it is
//! put on the stack above the one it is about (CR 603.3b) and is about that ability; the cause tells it which event the ability
//! triggered from, so it may count the abilities one kind of event caused and not those its own kind did.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn life_changes(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    if matches!(event, Event::LifeChange { .. }) { Fired::PLAIN } else { None }
}

/// "Whenever an ability this creature controls triggers from a life total changing, ..." about that ability.
fn caused_by_a_life_total_changing(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::TriggerOnStack { source, serial, cause: Some(cause), .. } = *event else { return None };
    if cause.kind != EventMask::LIFE_CHANGE.bit() || q.controller(source) != me.controller {
        return None;
    }
    match serial {
        Some(serial) => Fired::about(Target::Ability(serial)),
        None => Fired::PLAIN,
    }
}

/// A creature with an ability that triggers on a life total changing and one that triggers on the first being put on the stack.
static WATCHER: CardDef = CardDef {
    name: "Trigger On Stack Test Watcher",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[
        triggered(when(EventMask::LIFE_CHANGE, life_changes), |_, _| Ok(()), "Whenever a life total changes, nothing."),
        triggered(when_asked(EventMask::TRIGGER_ON_STACK, caused_by_a_life_total_changing), |_, _| Ok(()), "Whenever an ability this player controls triggers from a life total changing, nothing."),
    ],
    ..CardDef::BLANK
};

/// A creature with only the first.
static BEAR: CardDef = CardDef { name: "Trigger On Stack Test Bear", abilities: &[WATCHER.abilities[0]], ..WATCHER };

/// The watcher's second ability, and a first that targets an artifact: with none on the battlefield it is removed as it is put
/// on the stack (CR 603.3d).
static AIMLESS: CardDef = CardDef {
    name: "Trigger On Stack Test Aimless",
    abilities: &[
        triggered(when(EventMask::LIFE_CHANGE, life_changes), |_, _| Ok(()), "Whenever a life total changes, nothing happens to target artifact.")
            .targets(&[cantrip_mechanics::targets::ARTIFACT]),
        WATCHER.abilities[1],
    ],
    ..WATCHER
};

static FILLER: CardDef = CardDef { name: "Trigger On Stack Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn game(cards: &[&'static CardDef]) -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new(cards.iter().copied().chain([&FILLER]))));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 3;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}

fn enter(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// CR 603.3: the watcher's first ability triggers on a life total changing and is put on the stack; that is an event, which its
/// second ability is asked of, and it is put on the stack above the first and is about it, once: it was not caused by a life
/// total changing, so the second ability's own going onto the stack is not counted again (it would never end).
#[test]
fn an_ability_triggering_on_a_trigger_going_on_the_stack_is_about_that_trigger_and_above_it() {
    let mut game = game(&[&WATCHER]);
    let watcher = enter(&mut game, &WATCHER);
    cx(&mut game).emit(Event::LifeChange { player: YOU, delta: -1 });
    game.run();
    let stack = &game.state.stack;
    assert_eq!(stack.len(), 2, "the first ability, and the second above it, and nothing more");
    assert!(stack.iter().all(|i| i.object == watcher && i.kind == StackKind::Triggered));
    assert_eq!(stack[1].subject, Target::Ability(stack[0].serial), "the second is about the first");
    assert_ne!(stack[0].ability, stack[1].ability);
}

/// CR 603.3d: an ability with no legal target for it is removed from the stack as it is put there, but it triggered all the
/// same: the watcher's ability triggers on it, about no stack object (the event's `serial` is none).
#[test]
fn an_ability_removed_for_want_of_a_target_still_triggered() {
    let mut game = game(&[&AIMLESS]);
    let aimless = enter(&mut game, &AIMLESS);
    cx(&mut game).emit(Event::LifeChange { player: YOU, delta: -1 });
    game.run();
    let stack = &game.state.stack;
    assert_eq!(stack.len(), 1, "the targeting ability was removed; the watcher's is on the stack");
    assert_eq!((stack[0].object, stack[0].ability.index), (aimless, 1));
    assert_eq!(stack[0].subject, Target::None);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::TriggerOnStack { source, serial: None, .. } if source == aimless)));
}

fn recorded(game: &Game) -> usize {
    game.log.iter().filter(|r| matches!(r.event, Event::TriggerOnStack { .. })).count()
}

/// The event is emitted and recorded only while an ability in the game could trigger on it (`layers::trigger_on_stack_listened`),
/// whatever the card table holds: with the watcher in the table and nothing that listens in the game, the bear's ability goes on
/// the stack and nothing is recorded; with the watcher on the battlefield, each of its abilities' going on the stack is.
#[test]
fn it_is_recorded_only_while_an_object_in_the_game_listens() {
    let mut quiet = game(&[&WATCHER, &BEAR]);
    assert!(quiet.cards.any_listens_trigger_on_stack, "the table holds a listener");
    enter(&mut quiet, &BEAR);
    cx(&mut quiet).emit(Event::LifeChange { player: YOU, delta: -1 });
    quiet.run();
    assert_eq!(quiet.state.stack.len(), 1);
    assert_eq!(recorded(&quiet), 0, "no object in the game listens: nothing is recorded");

    let mut loud = game(&[&WATCHER, &BEAR]);
    enter(&mut loud, &WATCHER);
    cx(&mut loud).emit(Event::LifeChange { player: YOU, delta: -1 });
    loud.run();
    assert_eq!(loud.state.stack.len(), 2, "the watcher's first, and its second about it");
    assert_eq!(recorded(&loud), 2, "each going on the stack is recorded while the watcher is on the battlefield");
}

/// A phased-out permanent is treated as though it does not exist (CR 702.26b): a phased-out watcher listens for nothing.
#[test]
fn a_phased_out_listener_listens_for_nothing() {
    let mut game = game(&[&WATCHER, &BEAR]);
    enter(&mut game, &BEAR);
    let watcher = game.state.push_obj(Obj::new(game.cards.id_of(&WATCHER), YOU, Zone::PHASED_OUT));
    game.state.zone_push(Zone::PHASED_OUT, watcher);
    cx(&mut game).emit(Event::LifeChange { player: YOU, delta: -1 });
    game.run();
    assert_eq!(game.state.stack.len(), 1);
    assert_eq!(recorded(&game), 0);
}

/// An ability an effect gave an object listens as a printed one does (CR 613.1f): the bear given the watcher's second ability.
#[test]
fn a_given_listener_listens() {
    let mut game = game(&[&WATCHER, &BEAR]);
    let bear = enter(&mut game, &BEAR);
    let given = AbilityRef { card: game.cards.id_of(&WATCHER), index: 1 };
    cantrip_api::Game::add_effect(&mut cx(&mut game), bear, bear, EffectKind::AddAbility(given), Duration::Indefinite);
    cx(&mut game).emit(Event::LifeChange { player: YOU, delta: -1 });
    game.run();
    let stack = &game.state.stack;
    assert_eq!(stack.len(), 2, "the bear's first, and the given one above it about it");
    assert_eq!(stack[1].subject, Target::Ability(stack[0].serial));
    assert_eq!(recorded(&game), 2);
}
