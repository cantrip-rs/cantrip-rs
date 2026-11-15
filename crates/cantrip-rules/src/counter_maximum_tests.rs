//! CR 704.5r, 122.4: a permanent with an ability that says it can't have more than N counters of a kind on it, and more than
//! N on it, has all but N of them removed as a state-based action (`CardDef::counter_ceilings`). Over test cards.

use crate::{Cx, Game, Setup, counters, settle};
use cantrip_api::*;

static DREAMER: CardDef = CardDef {
    name: "Counter Maximum Test Dreamer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(1),
    counter_ceilings: &[(CounterKind::Dream, 7)],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Counter Maximum Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

const YOU: PlayerId = PlayerId(0);

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&DREAMER, &BEAR])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

fn put(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// The counters removed the history holds from the `from`th record on: object, kind, amount and who removed them.
fn removed(cx: &Cx, from: usize) -> Vec<(ObjectId, CounterKind, i32, Option<PlayerId>)> {
    cx.g.log[from..]
        .iter()
        .filter_map(|r| match r.event {
            Event::CountersRemoved { object, kind, amount, by, .. } => Some((object, kind, amount, by)),
            _ => None,
        })
        .collect()
}

/// Nine dream counters on a permanent that can't have more than seven: the check removes two, by the rules (no player), and
/// leaves its other counters and another permanent's dream counters alone.
#[test]
fn all_but_n_are_removed() {
    let mut game = game();
    let (dreamer, bear) = (put(&mut game, &DREAMER), put(&mut game, &BEAR));
    let mut cx = cx(&mut game);
    counters::put(&mut cx, dreamer, CounterKind::Dream, 9, YOU, counters::How::Effect);
    counters::put(&mut cx, dreamer, CounterKind::Charge, 9, YOU, counters::How::Effect);
    counters::put(&mut cx, bear, CounterKind::Dream, 9, YOU, counters::How::Effect);
    let from = cx.g.log.len();
    assert_eq!(settle::state_based_actions_alone(&mut cx), Ok(true));
    assert_eq!(removed(&cx, from), vec![(dreamer, CounterKind::Dream, 2, None)]);
    assert_eq!(cx.s().obj(dreamer).counters(CounterKind::Dream), 7);
    assert_eq!(cx.s().obj(dreamer).counters(CounterKind::Charge), 9);
    assert_eq!(cx.s().obj(bear).counters(CounterKind::Dream), 9);
}

/// Exactly N is not more than N: none is removed.
#[test]
fn exactly_n_stays() {
    let mut game = game();
    let dreamer = put(&mut game, &DREAMER);
    let mut cx = cx(&mut game);
    counters::put(&mut cx, dreamer, CounterKind::Dream, 7, YOU, counters::How::Effect);
    let from = cx.g.log.len();
    settle::state_based_actions_alone(&mut cx).unwrap();
    assert_eq!(removed(&cx, from), vec![]);
    assert_eq!(cx.s().obj(dreamer).counters(CounterKind::Dream), 7);
}

/// A permanent that has lost all its abilities has no such ability (CR 613.1f), and keeps its counters.
#[test]
fn not_without_its_abilities() {
    let mut game = game();
    let dreamer = put(&mut game, &DREAMER);
    let mut cx = cx(&mut game);
    counters::put(&mut cx, dreamer, CounterKind::Dream, 8, YOU, counters::How::Effect);
    cantrip_api::Game::add_effect(&mut cx, dreamer, dreamer, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    let from = cx.g.log.len();
    settle::state_based_actions_alone(&mut cx).unwrap();
    assert_eq!(removed(&cx, from), vec![]);
    assert_eq!(cx.s().obj(dreamer).counters(CounterKind::Dream), 8);
}
