//! An object that leaves the battlefield becomes a new object (CR 400.7): the id it had is no longer on the battlefield,
//! though the object keeps the zone it left from for its last-known information (CR 608.2h, 113.7a).

use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static BEAR: CardDef = CardDef {
    name: "Left Battlefield Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A permanent exiled: its old id is no longer on the battlefield, though it keeps the zone it left from, and what it became
/// is in exile.
#[test]
fn an_object_that_has_left_is_not_on_the_battlefield() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let bear = game.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, bear);
    assert!(game.is_on_battlefield(bear));
    let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    cantrip_api::Game::exile(&mut cx, bear);
    let exiled = game.state.obj(bear).became.expect("a new object in exile");
    assert!(game.state.obj(bear).zone.is(ZoneKind::Battlefield), "the old object keeps the zone it left from");
    assert!(!game.is_on_battlefield(bear));
    assert!(!game.is_on_battlefield(exiled));
}
