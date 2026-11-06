//! Bound references and an open creature restriction are different replacement predicates (CR 614.1a, 700.4).
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::replacements::{exile_if_it_would_die, CREATURE_EXILED_INSTEAD_OF_DYING, EXILE_INSTEAD_OF_DYING};
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
static FIXED: CardDef = CardDef { name: "Fixed exile source", types: TypeSet::of(&[CardType::Sorcery]),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING], ..CardDef::BLANK };
static OPEN: CardDef = CardDef { name: "Open creature exile source", types: TypeSet::of(&[CardType::Sorcery]),
    granted_replacements: &[CREATURE_EXILED_INSTEAD_OF_DYING], ..CardDef::BLANK };
static CREATURE: CardDef = CardDef { name: "Exile predicate creature", types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(2), toughness: Some(2), ..CardDef::BLANK };

fn cx(g: &mut Game) -> Cx<'_> { Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None,
    lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None } }
fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id); g.state.see_statics(); id
}
fn board(def: &'static CardDef) -> (Game, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&FIXED, &OPEN, &CREATURE])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    let source = put(&mut g, def, Zone::STACK);
    let object = put(&mut g, &CREATURE, Zone::BATTLEFIELD);
    let item = crate::actions::stack_item(StackKind::Spell, source, AbilityRef { card: cards.id_of(def), index: 0 }, YOU, &[]);
    let item_copy = item;
    g.state.stack_push(item); let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    exile_if_it_would_die(&mut cx(&mut g), &item_copy, 0, object);
    (g, object)
}
fn change_type(g: &mut Game, object: ObjectId, kind: CardType) {
    cantrip_api::Game::add_effect(&mut cx(g), object, object, EffectKind::SetTypes(TypeSet::of(&[kind])), Duration::EndOfTurn);
    assert!(g.chars(object).is(kind));
    assert!(!g.chars(object).is(CardType::Creature));
}
fn destroy_zone(g: &mut Game, object: ObjectId) -> ZoneKind {
    cantrip_api::Game::destroy_all(&mut cx(g), &[object]);
    g.obj(g.obj(object).became.expect("the object left the battlefield")).zone.kind()
}

#[test]
fn a_bound_permanent_remains_covered_after_losing_creature_type() {
    let (mut g, object) = board(&FIXED);
    change_type(&mut g, object, CardType::Artifact);
    assert_eq!(destroy_zone(&mut g, object), ZoneKind::Exile);
}
#[test]
fn an_open_creature_restriction_is_checked_when_the_permanent_would_die() {
    let (mut g, object) = board(&OPEN);
    change_type(&mut g, object, CardType::Artifact);
    assert_eq!(destroy_zone(&mut g, object), ZoneKind::Graveyard);
    let (mut g, object) = board(&OPEN);
    assert_eq!(destroy_zone(&mut g, object), ZoneKind::Exile);
}
#[test]
fn an_open_creature_restriction_does_not_also_mean_planeswalker() {
    let (mut g, object) = board(&OPEN);
    change_type(&mut g, object, CardType::Planeswalker);
    assert_eq!(destroy_zone(&mut g, object), ZoneKind::Graveyard);
}
#[test]
fn a_bound_exile_replacement_does_not_replace_a_return_to_hand_or_another_object() {
    let (mut g, object) = board(&FIXED);
    let other = put(&mut g, &CREATURE, Zone::BATTLEFIELD);
    assert_eq!(destroy_zone(&mut g, other), ZoneKind::Graveyard);
    cantrip_api::Game::return_to_hand(&mut cx(&mut g), &[object]);
    assert_eq!(g.obj(g.obj(object).became.unwrap()).zone.kind(), ZoneKind::Hand);
}
#[test]
fn a_bound_exile_replacement_does_not_follow_a_later_battlefield_incarnation() {
    let (mut g, object) = board(&FIXED);
    cantrip_api::Game::return_to_hand(&mut cx(&mut g), &[object]);
    let hand = g.obj(object).became.unwrap();
    let returned = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), hand, YOU, false).unwrap();
    assert_ne!(returned, object);
    assert_eq!(destroy_zone(&mut g, returned), ZoneKind::Graveyard);
}
