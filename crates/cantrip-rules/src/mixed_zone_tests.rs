//! Simultaneous moves to different zones plan every replacement against the same prior state (CR 614.12): the one
//! `move_together` that `return_until_left` (CR 610.3) and `sacrifice_and_return` make, a card to a hand and another to a
//! graveyard, is replaced by the state before either moves.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_api::Game as _;

const P0: PlayerId = PlayerId(0);

fn graveyard_while_hand_empty(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    q.state().zone(Zone::hand(asker.controller)).is_empty()
        && matches!(proposal, Proposal::ZoneChange(change)
            if change.to == Zone::graveyard(asker.controller))
}

fn exile_instead(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(change) = proposal {
        change.to = Zone::exile(change.to.player);
    }
}

static WATCHER: CardDef = CardDef {
    name: "Empty-hand Graveyard Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        graveyard_while_hand_empty,
        exile_instead,
        "If a card would be put into your graveyard while your hand is empty, exile it instead.",
    )],
    ..CardDef::BLANK
};

static CARD: CardDef = CardDef { name: "Moved card", ..CardDef::BLANK };

#[test]
fn cards_moved_to_a_hand_and_a_graveyard_together_are_replaced_by_the_state_before_either_moves() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&WATCHER, &CARD])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let watcher = game.state.push_obj(Obj::new(table.id_of(&WATCHER), P0, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, watcher);
    let hand_card = game.state.push_obj(Obj::new(table.id_of(&CARD), P0, Zone::library(P0)));
    game.state.zone_push(Zone::library(P0), hand_card);
    let grave_card = game.state.push_obj(Obj::new(table.id_of(&CARD), P0, Zone::library(P0)));
    game.state.zone_push(Zone::library(P0), grave_card);
    game.state.see_statics();

    let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    let moved = cx.move_together(&[(hand_card, Zone::hand(P0)), (grave_card, Zone::graveyard(P0))], MoveReason::Other);
    assert_eq!(moved.len(), 2);
    assert_eq!(cx.obj(moved[0]).zone, Zone::hand(P0));
    assert_eq!(cx.obj(moved[1]).zone, Zone::exile(P0));
    assert!(cx.state().zone(Zone::graveyard(P0)).is_empty());
}
