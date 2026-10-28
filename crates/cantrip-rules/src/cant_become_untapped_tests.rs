//! "Enchanted creature ... can't become untapped" (Frozen in Ice, Blossombind): `CardDef::cant_become_untapped` (CR 101.2,
//! 701.26b), over a test card. It holds in the untap step and for an effect that untaps (`Game::untap`), leaves a stun counter
//! where it is (nothing would untap the permanent for the counter to replace, CR 122.1d), is no ability of the creature (CR 113.1a,
//! 613.1f), and a source that lost its abilities makes no such rule.

use crate::turn::untap_step;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

/// "Creatures can't become untapped."
fn creatures(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

static FROST: CardDef =
    CardDef { name: "Untap Rule Test Frost", types: TypeSet::of(&[CardType::Enchantment]), cant_become_untapped: &[creatures], ..CardDef::BLANK };
static BEAR: CardDef =
    CardDef { name: "Untap Rule Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
static LAND: CardDef = CardDef { name: "Untap Rule Test Land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FROST, &BEAR, &LAND])));
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
    game.state.effect_push(ContinuousEffect { source: id, controller: YOU, timestamp, affected: id, kind: EffectKind::LoseAllAbilities, duration: Duration::EndOfTurn });
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn untap_step_of_yours(game: &mut Game) {
    game.state.update_turn(|t| {
        t.active = YOU;
        t.set_step(Step::Untap);
    });
    untap_step(&mut cx(game), YOU).expect("nothing is asked");
}

#[test]
fn a_creature_that_lost_its_abilities_still_cant_untap_in_the_untap_step_or_by_an_effect() {
    let mut game = game();
    permanent(&mut game, &FROST, false);
    let bear = permanent(&mut game, &BEAR, true);
    let land = permanent(&mut game, &LAND, true);
    lose_all_abilities(&mut game, bear);
    untap_step_of_yours(&mut game);
    assert!(game.state.obj(bear).tapped, "the creature untapped in the untap step");
    assert!(!game.state.obj(land).tapped, "the land did not untap");
    crate::ops::untap(&mut cx(&mut game), bear);
    assert!(game.state.obj(bear).tapped, "an effect untapped the creature");
}

#[test]
fn a_stun_counter_stays_on_a_creature_that_cant_become_untapped() {
    let mut game = game();
    permanent(&mut game, &FROST, false);
    let bear = permanent(&mut game, &BEAR, true);
    crate::counters::put(&mut cx(&mut game), bear, CounterKind::Stun, 1, YOU, crate::counters::How::Effect);
    untap_step_of_yours(&mut game);
    assert_eq!(game.state.obj(bear).counters(CounterKind::Stun), 1, "the counter was removed for an untap that could not happen");
}

#[test]
fn a_source_that_lost_its_abilities_bans_nothing() {
    let mut game = game();
    let frost = permanent(&mut game, &FROST, false);
    let bear = permanent(&mut game, &BEAR, true);
    lose_all_abilities(&mut game, frost);
    untap_step_of_yours(&mut game);
    assert!(!game.state.obj(bear).tapped);
}
