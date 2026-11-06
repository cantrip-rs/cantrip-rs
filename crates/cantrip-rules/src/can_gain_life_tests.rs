//! Whether a player could gain (CR 119.7) or lose (CR 119.8) life, over test cards: what a cost that has a player gain or pay life asks before it
//! can be paid.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

fn opponents(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player != asker.controller
}

/// "Your opponents can't gain life."
static KNIGHT: CardDef = CardDef { name: "Gain Test Knight", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), cant_gain_life: &[opponents], ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// With the Knight on the battlefield its controller can still gain life and their opponent can't; before it, the
/// opponent can.
#[test]
fn a_player_who_cant_gain_life_could_not_pay_a_cost_that_has_them_gain_it() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&KNIGHT])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let id = game.state.push_obj(Obj::new(table.id_of(&KNIGHT), YOU, Zone::BATTLEFIELD));
    let cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    assert!(cantrip_api::Game::can_gain_life(&cx, THEM));
    cx.g.state.zone_push(Zone::BATTLEFIELD, id);
    cx.g.state.see_statics();
    assert!(cantrip_api::Game::can_gain_life(&cx, YOU));
    assert!(!cantrip_api::Game::can_gain_life(&cx, THEM));
}

/// "You can't lose life."
static STASIS: CardDef = CardDef { name: "Lose Test Stasis", types: TypeSet::of(&[CardType::Artifact]), cant_lose_life: &[you], ..CardDef::BLANK };

fn you(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player == asker.controller
}

/// With the Stasis on the battlefield its controller can't lose life and their opponent still can; before it, the controller
/// can (CR 119.8).
#[test]
fn a_player_who_cant_lose_life_could_not_pay_a_cost_that_has_them_pay_it() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&STASIS])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let id = game.state.push_obj(Obj::new(table.id_of(&STASIS), YOU, Zone::BATTLEFIELD));
    let cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    assert!(cantrip_api::Game::can_lose_life(&cx, YOU));
    cx.g.state.zone_push(Zone::BATTLEFIELD, id);
    cx.g.state.see_statics();
    assert!(!cantrip_api::Game::can_lose_life(&cx, YOU));
    assert!(cantrip_api::Game::can_lose_life(&cx, THEM));
}
