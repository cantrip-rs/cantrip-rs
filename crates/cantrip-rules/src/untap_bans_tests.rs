//! "Creatures don't untap during their controllers' untap steps" (Intruder Alarm) and "enchanted creature ... doesn't untap
//! during its controller's untap step" (Infinite Coursework): `CardDef::untap_bans` (CR 502.3), over a test card. A rule of
//! its source, not an ability of the creature (CR 113.1a): the creature that loses all its abilities is still under it
//! (CR 613.1f), and it is never one of the creature's keywords; the source that loses its own abilities makes no such rule.

use crate::turn::untap_step;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

/// "Creatures don't untap during their controllers' untap steps."
fn creatures(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

static ALARM: CardDef = CardDef { name: "Untap Ban Test Alarm", types: TypeSet::of(&[CardType::Enchantment]), untap_bans: &[creatures], ..CardDef::BLANK };
static BEAR: CardDef =
    CardDef { name: "Untap Ban Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
static LAND: CardDef = CardDef { name: "Untap Ban Test Land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&ALARM, &BEAR, &LAND])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

fn permanent(game: &mut Game, def: &'static CardDef, tapped: bool) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.update_obj(id, |o| o.tapped = tapped);
    id
}

fn lose_all_abilities(game: &mut Game, id: ObjectId) {
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect { source: id, controller: YOU, timestamp, affected: id, kind: EffectKind::LoseAllAbilities, duration: Duration::EndOfTurn, words: Words::NONE });
}

fn untap_step_of_yours(game: &mut Game) {
    game.state.update_turn(|t| {
        t.active = YOU;
        t.set_step(Step::Untap);
    });
    let mut cx = Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    untap_step(&mut cx, YOU).expect("nothing is asked");
}

#[test]
fn a_creature_that_lost_its_abilities_still_doesnt_untap_and_has_no_keyword_for_it() {
    let mut game = game();
    permanent(&mut game, &ALARM, false);
    let bear = permanent(&mut game, &BEAR, true);
    let land = permanent(&mut game, &LAND, true);
    lose_all_abilities(&mut game, bear);
    assert_eq!(game.chars(bear).keywords, Keywords::NONE, "the ban is no ability of the creature");
    untap_step_of_yours(&mut game);
    assert!(game.state.obj(bear).tapped, "the creature untapped");
    assert!(!game.state.obj(land).tapped, "the land did not untap");
}

#[test]
fn a_source_that_lost_its_abilities_bans_nothing() {
    let mut game = game();
    let alarm = permanent(&mut game, &ALARM, false);
    let bear = permanent(&mut game, &BEAR, true);
    lose_all_abilities(&mut game, alarm);
    untap_step_of_yours(&mut game);
    assert!(!game.state.obj(bear).tapped);
}
