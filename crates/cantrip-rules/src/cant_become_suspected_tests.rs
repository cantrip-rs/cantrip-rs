//! "Enchanted creature ... can't become suspected" (Airtight Alibi): `CardDef::suspect_bans` (CR 701.60, 101.2), over a test
//! card. It stops a spell or ability that suspects the creature, is no ability of the creature (CR 113.1a, 613.1f), leaves a
//! creature that is already suspected so (CR 701.60a), and a source that lost its abilities makes no such rule.

use crate::{Cx, Game, Setup};
use cantrip_api::Game as Rules;
use cantrip_api::*;

/// "Creatures can't become suspected."
fn creatures(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

static ALIBI: CardDef =
    CardDef { name: "Suspect Rule Test Alibi", types: TypeSet::of(&[CardType::Enchantment]), suspect_bans: &[creatures], ..CardDef::BLANK };
static BEAR: CardDef =
    CardDef { name: "Suspect Rule Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&ALIBI, &BEAR])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

fn permanent(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn lose_all_abilities(game: &mut Game, id: ObjectId) {
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect { source: id, controller: YOU, timestamp, affected: id, kind: EffectKind::LoseAllAbilities, duration: Duration::EndOfTurn, words: Words::NONE });
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

#[test]
fn a_creature_that_cant_become_suspected_stays_unsuspected_even_without_abilities() {
    let mut game = game();
    permanent(&mut game, &ALIBI);
    let bear = permanent(&mut game, &BEAR);
    lose_all_abilities(&mut game, bear);
    cx(&mut game).suspect(bear);
    assert!(!Query::is_suspected(&cx(&mut game), bear));
}

#[test]
fn a_source_that_lost_its_abilities_bans_nothing() {
    let mut game = game();
    let alibi = permanent(&mut game, &ALIBI);
    let bear = permanent(&mut game, &BEAR);
    lose_all_abilities(&mut game, alibi);
    cx(&mut game).suspect(bear);
    assert!(Query::is_suspected(&cx(&mut game), bear));
}

#[test]
fn a_creature_that_is_already_suspected_stays_suspected() {
    let mut game = game();
    let bear = permanent(&mut game, &BEAR);
    cx(&mut game).suspect(bear);
    permanent(&mut game, &ALIBI);
    assert!(Query::is_suspected(&cx(&mut game), bear));
}
