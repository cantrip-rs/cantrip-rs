//! "Remove up to N counters from target creature" as an effect resolves (CR 608.2d, 122.1: Heartless Act, Hex Parasite):
//! `RequestKind::RemoveCounters` with `up_to`, checked by `validate`, and `counters::remove_up_to`, over a test card, since
//! no printed card is built on it yet.

use crate::{Cx, Game, Setup, counters, validate};
use cantrip_api::*;

static BEAR: CardDef = CardDef {
    name: "Counters Test Bear",
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
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// Three from a creature with two +1/+1 counters and two -1/-1 counters: up to three, or exactly three without `up_to`.
fn question(object: ObjectId, up_to: bool) -> Request {
    let pools = vec![
        CounterPool { object, kind: CounterKind::PlusOne, count: 2 },
        CounterPool { object, kind: CounterKind::MinusOne, count: 2 },
    ];
    Request { id: 1, player: YOU, kind: RequestKind::RemoveCounters { source: object, pools, total: 3, up_to }, may_cancel: false }
}

fn allowed(game: &Game, request: &Request, taken: &[(ObjectId, CounterKind, u16)]) -> bool {
    validate::check(game, request, &Answer::Counters(taken.to_vec())).is_ok()
}

/// "Up to three" (CR 608.2d): none, one, or three of two kinds are each an answer; four is not, nor more of a kind than
/// there are (CR 122.1), nor a kind the creature does not have.
#[test]
fn up_to_takes_none_to_the_most_and_no_more_of_a_kind_than_there_are() {
    let mut game = game();
    let id = bear(&mut game);
    let ask = question(id, true);
    assert!(allowed(&game, &ask, &[]));
    assert!(allowed(&game, &ask, &[(id, CounterKind::MinusOne, 1)]));
    assert!(allowed(&game, &ask, &[(id, CounterKind::PlusOne, 1), (id, CounterKind::MinusOne, 2)]));
    assert!(!allowed(&game, &ask, &[(id, CounterKind::PlusOne, 2), (id, CounterKind::MinusOne, 2)]));
    assert!(!allowed(&game, &ask, &[(id, CounterKind::PlusOne, 3)]));
    assert!(!allowed(&game, &ask, &[(id, CounterKind::Stun, 1)]));
}

/// Without `up_to` (a cost, CR 118.3), the number is exact: none and two are refused, three is the answer.
#[test]
fn without_up_to_the_number_is_exact() {
    let mut game = game();
    let id = bear(&mut game);
    let ask = question(id, false);
    assert!(!allowed(&game, &ask, &[]));
    assert!(!allowed(&game, &ask, &[(id, CounterKind::PlusOne, 2)]));
    assert!(allowed(&game, &ask, &[(id, CounterKind::PlusOne, 2), (id, CounterKind::MinusOne, 1)]));
}

/// Nothing is asked for up to none, of a creature with no counters, or of one that has changed zones (CR 400.7): none
/// come off, and no question waits. (The frame is none, so a question asked would not return.)
#[test]
fn nothing_is_asked_with_no_counters_to_remove() {
    let mut game = game();
    let bare = bear(&mut game);
    let gone = bear(&mut game);
    let mut cx = cx(&mut game);
    counters::put(&mut cx, bare, CounterKind::MinusOne, 1, YOU, counters::How::Effect);
    counters::put(&mut cx, gone, CounterKind::PlusOne, 2, YOU, counters::How::Effect);
    assert_eq!(counters::remove_up_to(&mut cx, YOU, bare, bare, 0), Ok(0));
    assert_eq!(cx.s().obj(bare).counters(CounterKind::MinusOne), 1);
    counters::remove(&mut cx, bare, CounterKind::MinusOne, 1);
    assert_eq!(counters::remove_up_to(&mut cx, YOU, bare, bare, 3), Ok(0));
    cx.st().zone_remove(Zone::BATTLEFIELD, gone);
    cx.st().update_obj(gone, |o| o.zone = Zone::GONE);
    assert_eq!(counters::remove_up_to(&mut cx, YOU, gone, gone, 3), Ok(0));
}

/// Choosing a counter kind reads the kinds present and does not remove the selected counter (CR 122.1, 608.2d).
#[test]
fn choosing_the_only_counter_kind_leaves_it_on_the_permanent() {
    let mut game = game();
    let source = bear(&mut game);
    let target = bear(&mut game);
    let mut cx = cx(&mut game);
    counters::put(&mut cx, target, CounterKind::PlusOne, 1, YOU, counters::How::Effect);
    assert_eq!(counters::choose_kind(&mut cx, YOU, source, target, false), Ok(Some(CounterKind::PlusOne)));
    assert_eq!(cx.s().obj(target).counters(CounterKind::PlusOne), 1);
}
