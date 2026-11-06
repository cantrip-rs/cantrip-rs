//! CR 122.1a / 613.4c: named P/T counter deltas are layered independently, while only the +1/+1 and -1/-1 pair
//! annihilates (704.5q). Armor Thrull, Clockwork Avian, Essence Flare, and Spirit Shackle use different deltas.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
static CREATURE: CardDef = CardDef {
    name: "P/T counter creature", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(3), ..CardDef::BLANK
};
static LOW: CardDef = CardDef { name: "P/T counter lower boundary", power: Some(i32::MIN), ..CREATURE };
static ARTIFACT: CardDef = CardDef { name: "P/T counter artifact", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static VEHICLE: CardDef = CardDef {
    name: "P/T counter vehicle", types: TypeSet::of(&[CardType::Artifact]), subtypes: SubtypeSet::of(&[Subtype::Vehicle]), ..CREATURE
};

fn game() -> Game {
    let table = Box::leak(Box::new(CardTable::new([&CREATURE, &LOW, &ARTIFACT, &VEHICLE])));
    let mut game = Game::new(table, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    game.verify = true;
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    game.state.frame_push(Frame::new(FrameKind::Priority));
    game
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    id
}

fn counters(game: &mut Game, id: ObjectId, kinds: &[(CounterKind, i32)]) {
    let mut counts = [0; CounterKind::COUNT];
    for &(kind, count) in kinds { counts[kind as usize] = count; }
    crate::counters::put_all(&mut cx(game), id, counts, YOU, crate::counters::How::Effect);
}

fn pt(game: &Game, id: ObjectId) -> (Option<i32>, Option<i32>) {
    let c = game.chars(id); (c.power, c.toughness)
}

#[test]
fn the_printed_counter_kinds_change_both_stats_by_their_own_amounts() {
    for (kind, expected) in [
        (CounterKind::Plus1Plus2, (4, 7)), (CounterKind::Plus1Plus0, (4, 3)),
        (CounterKind::Plus0Plus1, (2, 5)), (CounterKind::Plus2Plus2, (6, 7)),
        (CounterKind::Minus2Minus2, (-2, -1)), (CounterKind::Plus0Minus1, (2, 1)),
        (CounterKind::Plus0Minus2, (2, -1)), (CounterKind::Minus2Minus1, (-2, 1)),
        (CounterKind::Plus2Plus0, (6, 3)), (CounterKind::Plus0Plus2, (2, 7)),
        (CounterKind::Minus1Plus0, (0, 3)),
    ] {
        let mut game = game(); let id = put(&mut game, &CREATURE, Zone::BATTLEFIELD);
        counters(&mut game, id, &[(kind, 2)]);
        assert_eq!(pt(&game, id), (Some(expected.0), Some(expected.1)), "{}", kind.name());
    }
}

#[test]
fn unlike_counters_add_independently_and_removal_updates_the_layer_cache() {
    let mut game = game(); let id = put(&mut game, &CREATURE, Zone::BATTLEFIELD);
    counters(&mut game, id, &[(CounterKind::Plus1Plus2, 2), (CounterKind::Plus0Minus1, 3), (CounterKind::Plus0Plus1, 4)]);
    assert_eq!(pt(&game, id), (Some(4), Some(8)));
    cantrip_api::Game::remove_counters(&mut cx(&mut game), id, CounterKind::Plus1Plus2, 1);
    assert_eq!(pt(&game, id), (Some(3), Some(6)));
}

#[test]
fn counters_apply_after_base_stats_and_before_switching_despite_ability_loss() {
    let mut game = game(); let id = put(&mut game, &CREATURE, Zone::BATTLEFIELD);
    counters(&mut game, id, &[(CounterKind::Plus1Plus2, 2)]);
    for kind in [EffectKind::SetPt(5, 7), EffectKind::SwitchPt, EffectKind::LoseAllAbilities] {
        cantrip_api::Game::add_effect(&mut cx(&mut game), id, id, kind, Duration::EndOfTurn);
    }
    assert_eq!(pt(&game, id), (Some(11), Some(7)));
}

#[test]
fn counters_on_a_noncreature_wait_until_it_becomes_a_creature() {
    let mut game = game(); let id = put(&mut game, &ARTIFACT, Zone::BATTLEFIELD);
    counters(&mut game, id, &[(CounterKind::Plus0Plus1, 2)]);
    assert_eq!(pt(&game, id), (None, None));
    for kind in [EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), EffectKind::SetPt(2, 3)] {
        cantrip_api::Game::add_effect(&mut cx(&mut game), id, id, kind, Duration::EndOfTurn);
    }
    assert_eq!(pt(&game, id), (Some(2), Some(5)));
}

#[test]
fn counters_also_modify_a_creature_card_outside_the_battlefield() {
    let mut game = game(); let id = put(&mut game, &CREATURE, Zone::graveyard(YOU));
    counters(&mut game, id, &[(CounterKind::Plus1Plus2, 2)]);
    assert_eq!(pt(&game, id), (Some(4), Some(7)));
}

#[test]
fn printed_stats_on_a_vehicle_do_not_make_it_a_creature_for_its_counters() {
    let mut game = game(); let id = put(&mut game, &VEHICLE, Zone::BATTLEFIELD);
    let before = pt(&game, id);
    counters(&mut game, id, &[(CounterKind::Plus0Plus1, 2)]);
    assert_eq!(pt(&game, id), before);
    cantrip_api::Game::add_effect(&mut cx(&mut game), id, id, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::EndOfTurn);
    assert_eq!(pt(&game, id), (Some(2), Some(5)));
}

#[test]
fn the_full_signed_stat_range_is_preserved_and_counters_offset_before_clamping() {
    let mut game = game(); let id = put(&mut game, &LOW, Zone::BATTLEFIELD);
    counters(&mut game, id, &[(CounterKind::Plus2Plus2, i32::MAX)]);
    assert_eq!(pt(&game, id), (Some(i32::MAX - 1), Some(i32::MAX)));
    counters(&mut game, id, &[(CounterKind::Minus2Minus2, i32::MAX)]);
    assert_eq!(pt(&game, id), (Some(i32::MIN), Some(3)));
}

#[test]
fn only_the_one_one_pair_annihilates_not_other_opposing_counter_kinds() {
    let mut game = game(); let id = put(&mut game, &CREATURE, Zone::BATTLEFIELD);
    counters(&mut game, id, &[(CounterKind::PlusOne, 3), (CounterKind::MinusOne, 1),
                            (CounterKind::Plus2Plus2, 2), (CounterKind::Minus2Minus2, 2)]);
    crate::settle::state_based_actions_alone(&mut cx(&mut game)).unwrap();
    assert_eq!(pt(&game, id), (Some(4), Some(5)));
    assert_eq!(game.state.obj(id).counters(CounterKind::PlusOne), 2);
    assert_eq!(game.state.obj(id).counters(CounterKind::MinusOne), 0);
    assert_eq!(game.state.obj(id).counters(CounterKind::Plus2Plus2), 2);
    assert_eq!(game.state.obj(id).counters(CounterKind::Minus2Minus2), 2);
}

#[test]
fn undo_and_redo_restore_the_counter_values_and_derived_characteristics() {
    let mut game = game(); let id = put(&mut game, &CREATURE, Zone::BATTLEFIELD);
    let before = game.state.mark();
    counters(&mut game, id, &[(CounterKind::Plus1Plus2, 2)]);
    assert_eq!(pt(&game, id), (Some(4), Some(7)));
    let (undos, redos) = game.state.step_back(before);
    assert_eq!(pt(&game, id), (Some(2), Some(3)));
    assert_eq!(game.state.obj(id).counters(CounterKind::Plus1Plus2), 0);
    game.state.step_forward(undos, redos);
    assert_eq!(pt(&game, id), (Some(4), Some(7)));
}

#[test]
fn counter_presence_follows_put_remove_and_clear_for_all_kinds() {
    let mut game = game(); let id = put(&mut game, &CREATURE, Zone::BATTLEFIELD);
    assert!(!game.state.obj(id).has_counters());
    counters(&mut game, id, &[(CounterKind::Plus1Plus2, 0)]);
    assert!(!game.state.obj(id).has_counters());
    counters(&mut game, id, &[(CounterKind::Plus1Plus2, 2), (CounterKind::Charge, 1)]);
    cantrip_api::Game::remove_counters(&mut cx(&mut game), id, CounterKind::Plus1Plus2, 2);
    assert!(game.state.obj(id).has_counters(), "a charge counter still remains");
    assert_eq!(pt(&game, id), (Some(2), Some(3)));
    cantrip_api::Game::remove_counters(&mut cx(&mut game), id, CounterKind::Charge, 3);
    assert!(!game.state.obj(id).has_counters());
    counters(&mut game, id, &[(CounterKind::Plus1Plus2, 1)]);
    game.state.update_obj(id, |obj| obj.clear_counters());
    assert!(!game.state.obj(id).has_counters());
    assert_eq!(pt(&game, id), (Some(2), Some(3)));
}
