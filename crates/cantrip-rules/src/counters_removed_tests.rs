//! `Event::CountersRemoved`: recorded for every way counters come off an object (CR 122.1) - an effect, a move (CR 122.5),
//! the state-based action of CR 704.5q - with as many as came off, and never for counters that cease to exist as their
//! object changes zones (CR 122.2: they are not "removed"; Chandra, Fire Artisan's ruling). Over a test card.

use crate::{Cx, Game, Setup, counters, ops, settle};
use cantrip_api::*;

static BEAR: CardDef = CardDef {
    name: "Removed Counters Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

const YOU: PlayerId = PlayerId(0);

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

fn bear(game: &mut Game) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The counters removed the history holds from the `from`th record on: object, kind and amount, in order.
fn removed(cx: &Cx, from: usize) -> Vec<(ObjectId, CounterKind, i32)> {
    cx.g.log[from..]
        .iter()
        .filter_map(|r| match r.event {
            Event::CountersRemoved { object, kind, amount } => Some((object, kind, amount)),
            _ => None,
        })
        .collect()
}

/// An effect that removes more than there are records those there were (CR 122.1); one that finds none records nothing.
#[test]
fn a_removal_records_what_came_off_and_nothing_when_none_did() {
    let mut game = game();
    let id = bear(&mut game);
    let mut cx = cx(&mut game);
    counters::put(&mut cx, id, CounterKind::PlusOne, 3, YOU, counters::How::Effect);
    let from = cx.g.log.len();
    counters::remove(&mut cx, id, CounterKind::PlusOne, 5);
    counters::remove(&mut cx, id, CounterKind::PlusOne, 1);
    counters::remove(&mut cx, id, CounterKind::Stun, 1);
    assert_eq!(removed(&cx, from), vec![(id, CounterKind::PlusOne, 3)]);
}

/// A move takes the counters off one object and puts them on another (CR 122.5): a removal from the first.
#[test]
fn a_move_records_the_counters_taken_off() {
    let mut game = game();
    let (from_bear, to_bear) = (bear(&mut game), bear(&mut game));
    let mut cx = cx(&mut game);
    counters::put(&mut cx, from_bear, CounterKind::PlusOne, 2, YOU, counters::How::Effect);
    cx.doer = Some(YOU);
    let from = cx.g.log.len();
    assert_eq!(counters::mv(&mut cx, from_bear, to_bear, CounterKind::PlusOne, 1), 1);
    assert_eq!(removed(&cx, from), vec![(from_bear, CounterKind::PlusOne, 1)]);
}

/// CR 704.5q: N +1/+1 and N -1/-1 counters are removed, N the smaller count - two records of N.
#[test]
fn the_state_based_action_records_both_kinds() {
    let mut game = game();
    let id = bear(&mut game);
    let mut cx = cx(&mut game);
    counters::put(&mut cx, id, CounterKind::PlusOne, 3, YOU, counters::How::Effect);
    counters::put(&mut cx, id, CounterKind::MinusOne, 2, YOU, counters::How::Effect);
    let from = cx.g.log.len();
    assert_eq!(settle::state_based_actions_alone(&mut cx), Ok(true));
    assert_eq!(removed(&cx, from), vec![(id, CounterKind::PlusOne, 2), (id, CounterKind::MinusOne, 2)]);
    assert_eq!(cx.s().obj(id).counters(CounterKind::PlusOne), 1);
}

/// CR 122.2: counters on a permanent that leaves the battlefield are not removed; they cease to exist. Nothing is recorded.
#[test]
fn leaving_the_battlefield_removes_no_counters() {
    let mut game = game();
    let id = bear(&mut game);
    let mut cx = cx(&mut game);
    counters::put(&mut cx, id, CounterKind::PlusOne, 2, YOU, counters::How::Effect);
    let from = cx.g.log.len();
    let moved = ops::move_to(&mut cx, id, Zone::graveyard(YOU));
    assert_ne!(moved, id, "the bear moved");
    assert_eq!(removed(&cx, from), vec![]);
}
