//! One state-based check is judged on the board as it is when it is made (CR 704.3), the objects of the players who lose in
//! it included: they leave with their owners as part of the check (CR 800.4a), after the rest of it was judged, over test
//! cards.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn plus_zero_one(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.toughness = c.toughness.map(|t| t + 1);
}

/// "All creatures get +0/+1."
static BULWARK: CardDef = CardDef {
    name: "Check Test Bulwark",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtModify, creature, plus_zero_one, "")],
    ..CardDef::BLANK
};

static SHELL: CardDef = CardDef {
    name: "Check Test Shell",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

/// One check, in a settling frame of the game's own.
fn check(game: &mut Game) -> bool {
    if !game.state.agenda.last().is_some_and(|f| f.kind == FrameKind::Settle) {
        game.state.frame_push(Frame::new(FrameKind::Settle));
    }
    let frame = game.state.agenda.len() - 1;
    let mut cx = Cx { g: game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    crate::settle::state_based_actions(&mut cx).expect("nothing is asked")
}

/// The first player is at 0 life and owns the Bulwark; the second player's Shell has 1 damage, a 1/2 under the Bulwark. The
/// check that has the first player lose sees the Shell with toughness 2: it survives that check, and dies to the next, made
/// once the Bulwark has left the game with its owner.
#[test]
fn a_losers_anthem_still_counts_in_the_check_they_lose_in() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BULWARK, &SHELL])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(); 3], ..Default::default() });
    for p in 0..3 {
        game.state.update_player(PlayerId(p), |pl| pl.drew_from_empty = false);
    }
    put(&mut game, table, &BULWARK, PlayerId(0));
    let shell = put(&mut game, table, &SHELL, PlayerId(1));
    game.state.update_obj(shell, |o| o.damage = 1);
    game.state.update_player(PlayerId(0), |p| p.life = 0);
    assert!(check(&mut game));
    assert!(game.state.player(PlayerId(0)).lost);
    assert!(game.state.battlefield().contains(&shell), "the Shell had toughness 2 when the check was made");
    assert!(check(&mut game));
    assert!(!game.state.battlefield().contains(&shell), "with the Bulwark gone, the next check finds the Shell's damage lethal");
}
