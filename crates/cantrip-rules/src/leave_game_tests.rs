//! A player leaving a game of three (CR 800.4a), over test cards: their objects leave together, each as it was before any of
//! them left; the effects that gave them control end; then what they still control is exiled.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn plus_one(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.power = c.power.map(|p| p + 1);
    c.toughness = c.toughness.map(|t| t + 1);
}

/// "Creatures get +1/+1."
static ANTHEM: CardDef = CardDef {
    name: "Leave Test Anthem",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtModify, creature, plus_one, "")],
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Leave Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn enchanted(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    q.state().obj(source).attached_to == Some(affected)
}

/// "You control enchanted creature."
static MIND: CardDef = CardDef {
    name: "Leave Test Mind",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    statics: &[control_changing(enchanted, "")],
    ..CardDef::BLANK
};

const P1: PlayerId = PlayerId(0);
const P2: PlayerId = PlayerId(1);
const P3: PlayerId = PlayerId(2);

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn put(game: &mut Game, def: &'static CardDef, owner: PlayerId, controller: PlayerId) -> ObjectId {
    let card = game.cards.id_of(def);
    let id = game.state.push_obj(Obj::new(card, owner, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| {
        o.controller = controller;
        o.timestamp = timestamp;
    });
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

/// P3's Anthem is before P3's Bear in the battlefield's list, so leaving one at a time would take the Bear's row with the
/// Anthem gone; together, the Bear's row is the 3/3 it last was. P1's Bear that P3 controls by an effect goes back to P1
/// and stays; P1's other Bear, which P3 controls with no effect to end, is exiled after P3's objects have left, a 2/2 then.
#[test]
fn a_leaving_players_objects_leave_together_and_what_they_still_control_is_exiled_after() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&ANTHEM, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let anthem = put(&mut game, &ANTHEM, P3, P3);
    let own_bear = put(&mut game, &BEAR, P3, P3);
    let stolen = put(&mut game, &BEAR, P1, P1);
    let kept = put(&mut game, &BEAR, P1, P3);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, anthem, stolen, EffectKind::Control(P3), Duration::Indefinite);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(stolen).controller, P3);
    assert_eq!(cx.chars(own_bear).power, Some(3));
    crate::ops::leave_game(&mut cx, P3);
    let s = cx.s();
    assert!(s.obj(anthem).zone.is(ZoneKind::Gone) && s.obj(own_bear).zone.is(ZoneKind::Gone));
    assert_eq!(s.last_known(own_bear).map(|k| (k.power, k.toughness)), Some((Some(3), Some(3))));
    // The effect that gave P3 control ended: P1's Bear is P1's again, still on the battlefield.
    assert!(s.battlefield().contains(&stolen));
    assert_eq!(s.obj(stolen).controller, P1);
    // What P3 still controlled is exiled, as it was once P3's objects had left.
    assert!(!s.battlefield().contains(&kept));
    assert_eq!(s.last_known(kept).map(|k| k.power), Some(Some(2)));
    assert_eq!(s.zone(Zone::exile(P1)).len(), 1);
}

/// CR 800.4a ends every effect that gives the leaving player control, a static ability's too: P3 controls P1's Mind
/// Control-style Aura on P2's Bear, so controls the Bear. As P3 leaves, the Aura no longer gives them the Bear, which goes
/// back to P2 and stays on the battlefield; the Aura, still P3's, is exiled to its owner's exile.
#[test]
fn a_static_control_ability_stops_giving_the_leaving_player_control() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&MIND, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mind = put(&mut game, &MIND, P1, P3);
    game.state.update_obj(mind, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, P3);
    cx.st().update_player(P3, |p| p.lost = true);
    crate::ops::leave_game(&mut cx, P3);
    let s = cx.s();
    assert!(s.battlefield().contains(&bear));
    assert_eq!(s.obj(bear).controller, P2);
    assert!(!s.battlefield().contains(&mind));
    assert_eq!(s.zone(Zone::exile(P1)).len(), 1);
}

/// CR 800.4b: an object doesn't change to the control of a player who has left the game, so an effect that would give them
/// control after they left never begins ("that player gains control of Kain" after a three-player game's Kain dealt them
/// lethal damage). The Bear stays P2's, and `gain_control` says the player did not gain it ("if they do", CR 608.2c).
#[test]
fn a_player_who_has_left_gains_no_control() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mut cx = cx(&mut game);
    cx.st().update_player(P3, |p| p.lost = true);
    crate::ops::leave_game(&mut cx, P3);
    assert!(!cantrip_api::Game::gain_control(&mut cx, bear, bear, P3, Duration::Indefinite));
    let s = cx.s();
    assert!(s.battlefield().contains(&bear));
    assert_eq!(s.obj(bear).controller, P2);
    assert!(!s.effects.iter().any(|e| e.kind == EffectKind::Control(P3)));
}

/// The other side of it: a player still in the game gains control, and `gain_control` says so.
#[test]
fn a_player_still_in_the_game_gains_control() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
    let bear = put(&mut game, &BEAR, P2, P2);
    let mut cx = cx(&mut game);
    assert!(cantrip_api::Game::gain_control(&mut cx, bear, bear, P3, Duration::Indefinite));
    assert_eq!(cx.s().obj(bear).controller, P3);
}
