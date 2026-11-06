//! An object that has changed zones is a new object with no memory of the old one (CR 400.7): an instruction to move the
//! old object does nothing. Every move verb of `Game` is given a stale id - a card moved on since it was named, a token
//! that ceased to exist - and moves nothing, records nothing and makes no object, answering as it does for "nothing moved".
//! (Creeping Chill's "you may exile it" from the graveyard, after Cling to Dust exiled it in response, made a second copy.)

use crate::{Cx, Game, Setup};
use cantrip_api::Game as _;
use cantrip_api::*;

const P0: PlayerId = PlayerId(0);

static CARD: CardDef = CardDef { name: "Moved card", types: TypeSet::of(&[CardType::Creature]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&CARD])))
}

/// A game with a card in `zone`, then moved on from there to exile: the old id, and the card in exile.
fn moved_on(zone: Zone) -> (Game, ObjectId, ObjectId) {
    let table = table();
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let old = game.state.push_obj(Obj::new(table.id_of(&CARD), P0, zone));
    game.state.zone_push(zone, old);
    let new = {
        let mut cx = cx(&mut game);
        let new = cx.exile_all(&[old]);
        assert_eq!(new.len(), 1, "the card is exiled");
        new[0]
    };
    assert_ne!(old, new);
    (game, old, new)
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Every zone's contents, the objects and the records: what a move would change.
fn snapshot(game: &Game) -> (Vec<Vec<ObjectId>>, usize, usize) {
    let mut zones = Vec::new();
    for p in [P0, PlayerId(1)] {
        for zone in [Zone::library(p), Zone::hand(p), Zone::graveyard(p), Zone::exile(p)] {
            zones.push(game.state.zone(zone).to_vec());
        }
    }
    zones.push(game.state.zone(Zone::BATTLEFIELD).to_vec());
    zones.push(game.state.zone(Zone::STACK).to_vec());
    (zones, game.state.objects.len(), game.log.len())
}

/// `act` with the stale id `old` of a card that was in `zone` moves nothing: the game is as it was.
fn moves_nothing<T: std::fmt::Debug>(zone: Zone, act: impl FnOnce(&mut Cx, ObjectId, ObjectId) -> T, expect: impl FnOnce(ObjectId, T)) {
    let (mut game, old, new) = moved_on(zone);
    let before = snapshot(&game);
    let answer = act(&mut cx(&mut game), old, new);
    assert_eq!(snapshot(&game), before, "a stale id is moved nowhere (CR 400.7): no zone, object or record changes");
    assert_eq!(game.state.zone(Zone::exile(P0)), &[new], "the card is in exile once");
    expect(old, answer);
}

fn none<T: std::fmt::Debug>(_: ObjectId, answer: Vec<T>) {
    assert!(answer.is_empty(), "nothing moved: {answer:?}");
}

#[test]
fn move_to_and_exile_return_the_stale_id() {
    for to in [Zone::exile(P0), Zone::graveyard(P0), Zone::hand(P0), Zone::library(P0)] {
        moves_nothing(Zone::graveyard(P0), |cx, old, _| cx.move_to(old, to), |old, got| assert_eq!(got, old));
    }
    moves_nothing(Zone::graveyard(P0), |cx, old, _| cx.exile(old), |old, got| assert_eq!(got, old));
}

#[test]
fn move_together_moves_only_the_live_ones() {
    moves_nothing(Zone::graveyard(P0), |cx, old, _| cx.move_together(&[(old, Zone::exile(P0))], MoveReason::Other), none);
    moves_nothing(Zone::library(P0), |cx, old, _| cx.move_together_revealed(&[(old, Zone::hand(P0))], MoveReason::Other), none);
    // A live object beside the stale one still moves, and alone.
    let (mut game, old, new) = moved_on(Zone::graveyard(P0));
    let moved = cx(&mut game).move_together(&[(old, Zone::hand(P0)), (new, Zone::hand(P0))], MoveReason::Other);
    assert_eq!(moved.len(), 1);
    assert_eq!(game.state.zone(Zone::hand(P0)), &moved[..]);
    assert!(game.state.zone(Zone::exile(P0)).is_empty() && game.state.zone(Zone::graveyard(P0)).is_empty());
}

#[test]
fn the_named_verbs_move_nothing() {
    let from = Zone::graveyard(P0);
    moves_nothing(from, |cx, old, _| cx.exile_all(&[old]), none);
    moves_nothing(from, |cx, old, _| cx.put_into_graveyard(&[old]), none);
    moves_nothing(from, |cx, old, _| cx.return_to_hand(&[old]), none);
    moves_nothing(from, |cx, old, _| cx.return_to_hand_revealed(&[old]), none);
    moves_nothing(from, |cx, old, _| cx.put_into_library(&[old], LibraryPlace::Top), none);
    moves_nothing(from, |cx, old, _| cx.put_into_library(&[old], LibraryPlace::FromTop(1)), none);
    moves_nothing(Zone::hand(P0), |cx, old, _| cx.discard_together(&[old]), none);
    moves_nothing(Zone::hand(P0), |cx, old, _| cx.discard_revealed(&[old]), none);
    moves_nothing(Zone::hand(P0), |cx, old, _| cx.discard(old), |_, ()| ());
}

#[test]
fn exiling_with_a_permanent_moves_nothing_and_links_nothing() {
    let (mut game, old, new) = moved_on(Zone::graveyard(P0));
    let source = game.state.push_obj(Obj::new(game.state.obj(new).card, P0, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let by = Linked { object: source, text: game.state.obj(new).card, acquired: Acquired::OWN };
    let before = snapshot(&game);
    let mut cx = cx(&mut game);
    assert!(cx.exile_with(&[old], by, false).is_empty());
    assert!(cx.exile_with(&[old], by, true).is_empty());
    assert!(cx.exile_face_down(&[old], Some(by)).is_empty());
    assert!(cx.exile_face_down(&[old], None).is_empty());
    assert_eq!(cx.encode(old, source), None);
    assert_eq!(snapshot(&game), before);
    assert_eq!(game.state.obj(new).exiled, None, "the card exiled before is linked to nothing");
    assert!(!game.state.obj(new).face_down);
}

#[test]
fn exiling_with_a_permanent_links_the_live_card_with_its_own_zone() {
    // A stale id before a live one: the live card is exiled, remembered as from the library it left.
    let (mut game, old, _) = moved_on(Zone::graveyard(P0));
    let card = game.state.obj(old).card;
    let live = game.state.push_obj(Obj::new(card, P0, Zone::library(P0)));
    game.state.zone_push(Zone::library(P0), live);
    let source = game.state.push_obj(Obj::new(card, P0, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let by = Linked { object: source, text: card, acquired: Acquired::OWN };
    let exiled = cx(&mut game).exile_face_down(&[old, live], Some(by));
    assert_eq!(exiled.len(), 1);
    let o = game.state.obj(exiled[0]);
    assert_eq!(o.exiled.map(|l| l.from), Some(Zone::library(P0)));
    assert!(o.face_down);
    assert_eq!(o.seen_by, 0, "a card from a library, face down, is seen by nobody (CR 406.3)");
    // Face up, the same: the live card is linked to the permanent as from the library it left (CR 607.2a), not the
    // graveyard the stale one was in.
    let (mut game, old, _) = moved_on(Zone::graveyard(P0));
    let live = game.state.push_obj(Obj::new(card, P0, Zone::library(P0)));
    game.state.zone_push(Zone::library(P0), live);
    let source = game.state.push_obj(Obj::new(card, P0, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let by = Linked { object: source, text: card, acquired: Acquired::OWN };
    let exiled = cx(&mut game).exile_with(&[old, live], by, false);
    assert_eq!(exiled.len(), 1);
    assert_eq!(game.state.obj(exiled[0]).exiled.map(|l| l.from), Some(Zone::library(P0)));
}

#[test]
fn putting_onto_the_battlefield_moves_nothing() {
    let from = Zone::graveyard(P0);
    moves_nothing(from, |cx, old, _| cx.put_onto_battlefield(old, P0, false), |old, got| assert_eq!(got, Ok(old)));
    moves_nothing(from, |cx, old, _| cx.put_onto_battlefield_with_counters(old, P0, false, &[(CounterKind::PlusOne, 1)]), |old, got| {
        assert_eq!(got, Ok(old))
    });
    moves_nothing(from, |cx, old, _| cx.put_onto_battlefield_transformed(old, P0, false), |old, got| assert_eq!(got, Ok(old)));
    moves_nothing(from, |cx, old, _| cx.put_onto_battlefield_attacking(old, P0, Target::Player(PlayerId(1)), &[]), |old, got| {
        assert_eq!(got, Ok(old))
    });
    moves_nothing(from, |cx, old, _| cx.put_all_onto_battlefield(&[(old, P0)]), |_, got| assert_eq!(got, Ok(Vec::new())));
    moves_nothing(from, |cx, old, _| cx.put_onto_battlefield_face_down(old, P0, FaceDown::Manifested, false), |_, got| {
        assert_eq!(got, Ok(None))
    });
}

#[test]
fn a_permanent_that_left_is_not_destroyed_or_sacrificed_again() {
    let from = Zone::BATTLEFIELD;
    moves_nothing(from, |cx, old, _| cx.destroy_all(&[old]), none);
    moves_nothing(from, |cx, old, _| cx.destroy_all_no_regen(&[old]), none);
    moves_nothing(from, |cx, old, _| cx.sacrifice(P0, &[old]), none);
    moves_nothing(from, |cx, old, _| cx.move_to(old, Zone::graveyard(P0)), |old, got| assert_eq!(got, old));
}

#[test]
fn an_object_that_ceased_to_exist_moves_nowhere() {
    let table = table();
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let mut obj = Obj::new(table.id_of(&CARD), P0, Zone::graveyard(P0));
    obj.token = true;
    let token = game.state.push_obj(obj);
    game.state.zone_push(Zone::graveyard(P0), token);
    crate::ops::cease_to_exist(&mut cx(&mut game), Zone::graveyard(P0), token);
    let before = snapshot(&game);
    let mut cx = cx(&mut game);
    assert_eq!(cx.move_to(token, Zone::exile(P0)), token);
    assert!(cx.return_to_hand(&[token]).is_empty());
    assert_eq!(cx.put_onto_battlefield(token, P0, false), Ok(token));
    assert_eq!(snapshot(&game), before);
}
