//! Who sacrifices (CR 701.21a): "To sacrifice a permanent, its controller moves it from the battlefield directly to its
//! owner's graveyard. A player can't sacrifice ... a permanent they don't control." The verb takes the sacrificing player
//! and moves nothing that player doesn't control, whoever owns it, and a choice of what to sacrifice offers the chooser
//! only their own permanents, over test cards.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

static BEAR: CardDef = CardDef {
    name: "Sacrifice Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static RELIC: CardDef = CardDef { name: "Sacrifice Test Relic", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR, &RELIC])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

/// Their Bear: you can't sacrifice it, and telling you to leaves it where it is; they can, and do.
#[test]
fn a_player_cant_sacrifice_a_permanent_another_player_controls() {
    let (mut game, table) = game();
    let bear = put(&mut game, table, &BEAR, THEM);
    let mut cx = cx(&mut game);
    assert!(!cantrip_api::Game::can_sacrifice(&cx, YOU, bear));
    assert!(cantrip_api::Game::can_sacrifice(&cx, THEM, bear));
    assert!(cantrip_api::Game::sacrifice(&mut cx, YOU, &[bear]).is_empty());
    assert!(cx.s().obj(bear).zone.is(ZoneKind::Battlefield));
    assert_eq!(cantrip_api::Game::sacrifice(&mut cx, THEM, &[bear]).len(), 1);
    assert_eq!(cx.s().zone(Zone::graveyard(THEM)).len(), 1);
}

/// Your Bear, which they have gained control of: its owner can no longer sacrifice it - "you may sacrifice it" from one of
/// your abilities does nothing - and its new controller sacrifices it into its owner's graveyard, yours.
#[test]
fn a_permanent_another_player_took_is_sacrificed_only_by_its_new_controller() {
    let (mut game, table) = game();
    let relic = put(&mut game, table, &RELIC, THEM);
    let bear = put(&mut game, table, &BEAR, YOU);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, relic, bear, EffectKind::Control(THEM), Duration::Indefinite);
    assert_eq!(cx.s().obj(bear).controller, THEM);
    assert!(!cantrip_api::Game::can_sacrifice(&cx, YOU, bear));
    assert!(cantrip_api::Game::sacrifice(&mut cx, YOU, &[bear]).is_empty());
    assert!(cx.s().obj(bear).zone.is(ZoneKind::Battlefield));
    assert!(cantrip_api::Game::can_sacrifice(&cx, THEM, bear));
    assert_eq!(cantrip_api::Game::sacrifice(&mut cx, THEM, &[bear]).len(), 1);
    assert_eq!(cx.s().zone(Zone::graveyard(YOU)).len(), 1);
    assert!(cx.s().zone(Zone::graveyard(THEM)).is_empty());
}

/// Several players sacrificing at once: each pair is judged by its own player, so the pair naming a permanent another
/// player controls moves nothing and the rest still go, together.
#[test]
fn players_sacrificing_together_each_sacrifice_only_their_own() {
    let (mut game, table) = game();
    let mine = put(&mut game, table, &BEAR, YOU);
    let theirs = put(&mut game, table, &BEAR, THEM);
    let their_relic = put(&mut game, table, &RELIC, THEM);
    let mut cx = cx(&mut game);
    let gone = cantrip_api::Game::sacrifice_each(&mut cx, &[(YOU, mine), (YOU, theirs), (THEM, their_relic)]);
    assert_eq!(gone.len(), 2);
    assert_eq!(cx.s().battlefield().to_vec(), vec![theirs]);
    assert_eq!(cx.s().zone(Zone::graveyard(YOU)).len(), 1);
    assert_eq!(cx.s().zone(Zone::graveyard(THEM)).len(), 1);
}

/// A choice of what to sacrifice handed every Bear on the battlefield offers the chooser only their own (CR 701.21a): the
/// question is narrowed to what they can sacrifice before it is asked.
#[test]
fn a_choice_of_what_to_sacrifice_offers_only_the_choosers_own_permanents() {
    let (mut game, table) = game();
    let source = put(&mut game, table, &RELIC, YOU);
    let mine = put(&mut game, table, &BEAR, YOU);
    let theirs = put(&mut game, table, &BEAR, THEM);
    let cx = cx(&mut game);
    let bears = vec![mine, theirs];
    let ask = RequestKind::choose_cards(source, Purpose::Sacrifice, bears.clone(), bears, 1, false, "");
    match cx.as_allowed(YOU, ask) {
        Ok(RequestKind::ChooseCards { rule, .. }) => assert_eq!(rule.among, Some(vec![mine])),
        other => panic!("expected the question narrowed, got {other:?}"),
    }
}
