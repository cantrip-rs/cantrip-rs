//! A permanent that stops being suspected changes the game with no event (CR 701.60a, 603.8): the change is announced like
//! any other effect that ends, so the state triggers are judged after it and a card says where its instruction ends.
use crate::{Cx, Game, Setup, StateCheck};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
static BEAR: CardDef = CardDef {
    name: "Unsuspect test creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
    });
    g.verify = true;
    g
}
fn put(g: &mut Game) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: 0,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: vec![],
        damage: None,
        asked_at: None,
        doer: None,
        producing: None,
        reached: None,
    }
}
fn resolving(cx: &mut Cx, source: ObjectId) {
    cx.g.check = StateCheck { depth: 1, resolving: Some(source), ..StateCheck::NONE };
}
#[test]
fn ending_a_suspicion_changes_the_game() {
    let mut g = game();
    let bear = put(&mut g);
    let mut cx = cx(&mut g);
    cx.suspect(bear);
    assert!(Query::is_suspected(&cx, bear));
    resolving(&mut cx, bear);
    cx.unsuspect(bear);
    assert!(!Query::is_suspected(&cx, bear));
    assert!(cx.g.check.due);
    assert!(cx.g.check.done);
}
#[test]
fn a_permanent_that_is_not_suspected_is_unchanged() {
    let mut g = game();
    let bear = put(&mut g);
    let mut cx = cx(&mut g);
    resolving(&mut cx, bear);
    cx.unsuspect(bear);
    assert!(!cx.g.check.due);
    assert!(!cx.g.check.done);
}
#[test]
fn suspicions_ended_by_one_instruction_are_one_change() {
    let mut g = game();
    let one = put(&mut g);
    let two = put(&mut g);
    let mut cx = cx(&mut g);
    cx.suspect(one);
    cx.suspect(two);
    resolving(&mut cx, one);
    instruction(&mut cx, |g| {
        g.unsuspect(one);
        g.unsuspect(two);
    });
    assert!(!Query::is_suspected(&cx, one));
    assert!(!Query::is_suspected(&cx, two));
    assert!(cx.g.check.due);
}
#[test]
#[should_panic(expected = "changed the game twice with no event between")]
fn suspicions_ended_with_nothing_said_between_are_caught() {
    let mut g = game();
    let one = put(&mut g);
    let two = put(&mut g);
    let mut cx = cx(&mut g);
    cx.suspect(one);
    cx.suspect(two);
    resolving(&mut cx, one);
    cx.unsuspect(one);
    cx.unsuspect(two);
}
