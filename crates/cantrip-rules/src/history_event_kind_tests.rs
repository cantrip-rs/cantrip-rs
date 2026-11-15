//! Exact history event classification, including formerly aliased dispatch kinds.
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

#[test]
fn review_independent_exert_event_is_not_a_tapped_history_event() {
    let (mut g, id) = game();
    ops::exert(&mut cx(&mut g), id, YOU);
    assert!(!g.state.obj(id).tapped, "positive control: exert does not tap");
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Exerted { permanent, .. } if permanent == id)).count(), 1, "exert is independently recorded");
    assert_eq!(cantrip_api::Query::history(&g).count(EventQuery::kind(EventMask::TAPPED).this_turn().object(id)), 0, "exert must not be classified as tapping by history kind queries");
}

#[test]
fn kind_queries_distinguish_tapping_exerting_transforming_and_mutating_with_combined_masks() {
    let (mut g, id) = game();
    for event in [Event::Tapped { permanent: id }, Event::Exerted { permanent: id, player: YOU },
        Event::Transformed { object: id }, Event::Mutated { permanent: id, spell: id, on_top: true }] {
        cx(&mut g).record(event);
    }
    let history = cantrip_api::Query::history(&g);
    for kind in [EventMask::TAPPED, EventMask::EXERTED, EventMask::TRANSFORMED, EventMask::MUTATED] {
        assert_eq!(history.count(EventQuery::kind(kind).object(id).this_turn()), 1, "each exact kind selects its own record");
    }
    assert_eq!(history.count(EventQuery::kind(EventMask(EventMask::TAPPED.0 | EventMask::EXERTED.0)).object(id)), 2);
    assert_eq!(history.count(EventQuery::kind(EventMask(EventMask::TRANSFORMED.0 | EventMask::MUTATED.0)).object(id)), 2);
    assert_eq!(history.count(EventQuery::kind(EventMask::DREW).object(id)), 0, "unrelated history kind remains empty");
    assert_eq!(history.count(EventQuery::default().object(id)), 4, "unfiltered history retains all four events");
    assert!(!EventMask::of(&Event::Exerted { permanent: id, player: YOU }).intersects(EventMask::TAPPED), "dispatch is exact too");
    assert!(!EventMask::of(&Event::Mutated { permanent: id, spell: id, on_top: true }).intersects(EventMask::TRANSFORMED));
}

#[test]
fn exact_tap_and_exert_history_survives_source_departure_and_tree_undo_redo() {
    let (mut g, id) = game();
    let before = g.at;
    let mark = (g.state.mark(), g.log.len());
    g.tree.close(before, mark);
    let child = g.tree.add(before, (0, YOU, Answer::Yes), mark, &[]);
    ops::tap(&mut cx(&mut g), id);
    ops::exert(&mut cx(&mut g), id, YOU);
    ops::exile_all(&mut cx(&mut g), &[id]);
    g.at = child;
    let count = |g: &Game, kind| cantrip_api::Query::history(g).count(EventQuery::kind(kind).object(id));
    assert_eq!((count(&g, EventMask::TAPPED), count(&g, EventMask::EXERTED)), (1, 1), "history uses the departed source's exact identity");
    g.goto(before);
    assert_eq!((count(&g, EventMask::TAPPED), count(&g, EventMask::EXERTED)), (0, 0), "undo removes both exact records");
    g.goto(child);
    assert_eq!((count(&g, EventMask::TAPPED), count(&g, EventMask::EXERTED)), (1, 1), "redo restores each independent kind");
}
