//! "As this [permanent] transforms into [this face], ..." (CR 712.20, `CardDef::as_transforms`), over test cards: the
//! ability is the face's it turns over to, applied as it turns over by whatever turns it over - an effect, the
//! permanent's own ability, day and night - with its controller as "you", and never as it enters transformed (CR 712.14a).
//! Sephiroth, One-Winged Angel's "you get an emblem" is the model.

use crate::{Cx, Game, Setup, Stop, day_night, ops};
use cantrip_api::*;

/// The emblem the back face's ability gives.
static BACK_EMBLEM: CardDef = CardDef { name: "Transform Test Back Emblem", ..CardDef::BLANK };
/// The emblem the front face's ability gives, as the permanent transforms back.
static FRONT_EMBLEM: CardDef = CardDef { name: "Transform Test Front Emblem", ..CardDef::BLANK };

/// "As this creature transforms into [the back face], you get an emblem."
fn back_emblem(g: &mut dyn cantrip_api::Game, asker: &Asker) -> Result<(), Suspend> {
    let def = g.card_id(&BACK_EMBLEM);
    g.create_emblem(def, asker.controller);
    Ok(())
}

/// "As this creature transforms into [the front face], you get an emblem."
fn front_emblem(g: &mut dyn cantrip_api::Game, asker: &Asker) -> Result<(), Suspend> {
    let def = g.card_id(&FRONT_EMBLEM);
    g.create_emblem(def, asker.controller);
    Ok(())
}

/// An ability that asks something: the question stops the run.
fn asks(_: &mut dyn cantrip_api::Game, _: &Asker) -> Result<(), Suspend> {
    Err(Suspend)
}

static BACK: CardDef = CardDef {
    name: "Transform Test Angel",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(5),
    as_transforms: Some(back_emblem),
    ..CardDef::BLANK
};
static FRONT: CardDef = CardDef {
    name: "Transform Test Soldier",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    as_transforms: Some(front_emblem),
    back: Some(&BACK),
    ..CardDef::BLANK
};
/// A daybound card whose nightbound back face has the ability: day and night turn it over (CR 702.145c).
static NIGHT_BACK: CardDef = CardDef {
    name: "Transform Test Nightbound",
    types: TypeSet::of(&[CardType::Creature]),
    keywords: Keywords::of(&[Keyword::Nightbound]),
    as_transforms: Some(back_emblem),
    ..CardDef::BLANK
};
static DAY_FRONT: CardDef = CardDef {
    name: "Transform Test Daybound",
    types: TypeSet::of(&[CardType::Creature]),
    keywords: Keywords::of(&[Keyword::Daybound]),
    back: Some(&NIGHT_BACK),
    ..CardDef::BLANK
};
/// A back face whose ability asks a question (Shinryu's "choose an opponent").
static ASKING_BACK: CardDef = CardDef { name: "Transform Test Asker", types: TypeSet::of(&[CardType::Creature]), as_transforms: Some(asks), ..CardDef::BLANK };
static ASKING_FRONT: CardDef = CardDef { name: "Transform Test Asking Front", types: TypeSet::of(&[CardType::Creature]), back: Some(&ASKING_BACK), ..CardDef::BLANK };

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

/// Humility's first half: "All creatures lose all abilities."
static HUMBLER: CardDef = CardDef {
    name: "Transform Test Humbler",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[loses_all_abilities(creature, "")],
    ..CardDef::BLANK
};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&FRONT, &BACK, &DAY_FRONT, &NIGHT_BACK, &ASKING_FRONT, &ASKING_BACK, &HUMBLER, &BACK_EMBLEM, &FRONT_EMBLEM])))
}

fn game() -> Game {
    let mut game = Game::new(table(), &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    game
}

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    game.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Each emblem in the command zone, by name and owner.
fn emblems(cx: &Cx) -> Vec<(&'static str, PlayerId)> {
    cx.s().zone(Zone::COMMAND).iter().map(|&id| (cx.cards().get(cx.s().obj(id).card).name, cx.s().obj(id).owner)).collect()
}

fn transformed_events(cx: &Cx, id: ObjectId) -> usize {
    cx.g.log.iter().filter(|r| r.event == Event::Transformed { object: id }).count()
}

/// An effect transforms it (Tetzin's "transform up to one other target double-faced artifact", `Game::transform`): the
/// back face's ability applies, giving its controller the emblem, and the front face's does not - it would not have that
/// ability after (CR 712.20). Transformed back, the front face's applies. Each is part of the one transformation.
#[test]
fn an_effect_transforming_it_applies_the_ability_of_the_face_it_turns_over_to() {
    let mut game = game();
    let soldier = put(&mut game, &FRONT, Zone::BATTLEFIELD);
    let mut cx = cx(&mut game);
    ops::transform(&mut cx, soldier, None).unwrap();
    assert!(cx.s().obj(soldier).transformed);
    assert_eq!(emblems(&cx), [("Transform Test Back Emblem", YOU)]);
    assert_eq!(transformed_events(&cx, soldier), 1);
    ops::transform(&mut cx, soldier, None).unwrap();
    assert!(!cx.s().obj(soldier).transformed);
    assert_eq!(emblems(&cx), [("Transform Test Back Emblem", YOU), ("Transform Test Front Emblem", YOU)]);
    assert_eq!(transformed_events(&cx, soldier), 2);
}

/// Its own ability transforms it (Sephiroth's "transform Sephiroth", `Game::transform_by` with its stack item): the same,
/// and "you" is the permanent's controller as it transforms, here an opponent who has gained control of it (CR 613.1b).
#[test]
fn its_own_ability_transforming_it_gives_the_emblem_to_its_controller() {
    let mut game = game();
    let soldier = put(&mut game, &FRONT, Zone::BATTLEFIELD);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, soldier, soldier, EffectKind::Control(THEM), Duration::EndOfTurn);
    assert_eq!(cx.s().obj(soldier).controller, THEM);
    let ability = AbilityRef { card: cx.cards().id_of(&FRONT), index: 0 };
    let mut item = crate::actions::stack_item(StackKind::Triggered, soldier, ability, THEM, &[]);
    item.since = cx.g.log.len() as u32;
    ops::transform(&mut cx, soldier, Some(&item)).unwrap();
    assert!(cx.s().obj(soldier).transformed);
    assert_eq!(emblems(&cx), [("Transform Test Back Emblem", THEM)]);
    // CR 701.27f: having transformed since, the same ability's instruction is ignored - no transformation, no emblem.
    ops::transform(&mut cx, soldier, Some(&item)).unwrap();
    assert!(cx.s().obj(soldier).transformed);
    assert_eq!(emblems(&cx).len(), 1);
}

/// Put onto the battlefield transformed, it enters with its back face up and did not transform (CR 712.14a; Sephiroth's
/// ruling "If this card somehow enters the battlefield with its back face up, it didn't transform, so you won't get an
/// emblem").
#[test]
fn entering_transformed_is_no_transformation() {
    let mut game = game();
    let card = put(&mut game, &FRONT, Zone::hand(YOU));
    let mut cx = cx(&mut game);
    let angel = ops::put_onto_battlefield_as(&mut cx, card, YOU, false, true, ZoneReason::Other, &[]).unwrap();
    assert!(cx.s().obj(angel).transformed && cx.s().battlefield().contains(&angel));
    assert!(emblems(&cx).is_empty());
    assert_eq!(transformed_events(&cx, angel), 0);
}

/// A permanent that would have lost all its abilities once turned over has no "as this transforms" ability then
/// (CR 712.20, 613.1f): it transforms, and nothing is applied.
#[test]
fn a_permanent_without_its_abilities_transforms_with_none_applied() {
    let mut game = game();
    put(&mut game, &HUMBLER, Zone::BATTLEFIELD);
    let soldier = put(&mut game, &FRONT, Zone::BATTLEFIELD);
    let mut cx = cx(&mut game);
    ops::transform(&mut cx, soldier, None).unwrap();
    assert!(cx.s().obj(soldier).transformed);
    assert!(emblems(&cx).is_empty());
    assert_eq!(transformed_events(&cx, soldier), 1);
}

/// Day and night turn a daybound permanent over (CR 702.145c): that is a transformation too, and its face's ability
/// applies.
#[test]
fn night_turning_it_over_applies_the_ability() {
    let mut game = game();
    let wolf = put(&mut game, &DAY_FRONT, Zone::BATTLEFIELD);
    game.state.update_turn(|t| t.day_night = DayNight::Day);
    let mut cx = cx(&mut game);
    day_night::become_designation(&mut cx, DayNight::Night).unwrap();
    assert!(cx.s().obj(wolf).transformed);
    assert_eq!(emblems(&cx), [("Transform Test Back Emblem", YOU)]);
}

/// An ability that asks a question stops the transformation where it is asked, before its event, so the run that made it
/// transform is replayed with the answer as every question is.
#[test]
fn an_ability_that_asks_stops_the_run_before_the_transformation_completes() {
    let mut game = game();
    let asker = put(&mut game, &ASKING_FRONT, Zone::BATTLEFIELD);
    let mut cx = cx(&mut game);
    assert_eq!(ops::transform(&mut cx, asker, None), Err(Stop::Suspend));
    assert_eq!(transformed_events(&cx, asker), 0);
}
