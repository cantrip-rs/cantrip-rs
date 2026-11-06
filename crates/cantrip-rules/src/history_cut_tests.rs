//! The cut of the history that triggers' own questions see while the triggers of an event are looked for (CR 603.10,
//! `Game::history_end`), over a test card.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn panics(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
    panic!("a trigger's question panics");
}

/// A permanent whose triggered ability panics as it is asked about a life total changing.
static PANICKER: CardDef = CardDef {
    name: "Transformed Cast Test Panicker",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, panics), |_, _| Ok(()), "Whenever a life total changes, panic.")],
    ..CardDef::BLANK
};

/// The cut of the history the triggers' own questions see (CR 603.10, `Game::history_end`) is lifted even when looking for
/// triggers panics, so a host that catches the panic goes on with the whole history.
#[test]
fn the_history_is_whole_again_after_a_trigger_panics() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&PANICKER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let id = game.state.push_obj(Obj::new(table.id_of(&PANICKER), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cx(&mut game).emit(Event::LifeChange { player: YOU, delta: -1 })));
    assert!(caught.is_err(), "the trigger did not panic");
    assert_eq!(game.history_end, None, "the history is still cut");
}
