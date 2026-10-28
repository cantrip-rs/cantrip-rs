//! Cloak (CR 701.58) and manifest (CR 701.40) as the core does them, `Game::put_onto_battlefield_face_down`, over test
//! cards: a cloaked permanent is a nameless 2/2 creature with ward {2} (CR 701.58a), turned face up for its card's mana
//! cost if it is a creature card (CR 701.58b), and a cloaked instant or sorcery stays face down (CR 701.58g).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;

static BEAST: CardDef = CardDef {
    name: "Cloak Test Beast",
    cost: generic(2),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
};
static BOLT: CardDef = CardDef { name: "Cloak Test Bolt", cost: generic(1), types: TypeSet::of(&[CardType::Instant]), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// Your precombat main phase, with a Beast and a Bolt on top of your library and a Bolt of theirs in their hand.
fn game() -> (Game, ObjectId, ObjectId, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAST, &BOLT])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), owner, zone));
        game.state.zone_push(zone, id);
        id
    };
    let beast = put(&mut game, &BEAST, YOU, Zone::library(YOU));
    let bolt = put(&mut game, &BOLT, YOU, Zone::library(YOU));
    let theirs = put(&mut game, &BOLT, THEM, Zone::hand(THEM));
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, beast, bolt, theirs)
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn face_down(cx: &mut Cx, card: ObjectId, how: FaceDown) -> Option<ObjectId> {
    cantrip_api::Game::put_onto_battlefield_face_down(cx, card, YOU, how, false).expect("nothing is asked")
}

fn has_ward(cx: &Cx, id: ObjectId) -> bool {
    cx.chars(id).abilities().any(|a| matches!(cx.cards().ability(a), AbilityDef::Triggered(t) if t.ward))
}

/// Whether `YOU` are offered turning `id` face up (CR 701.58b), a special action they can afford.
fn offered_face_up(cx: &mut Cx, id: ObjectId) -> bool {
    priority_options(cx, YOU).iter().any(|o| {
        o.kind == ActionKind::ActivateAbility
            && o.source == Some(id)
            && cx.chars(id).abilities().nth(o.index as usize).is_some_and(|a| cx.cards().turns_face_up(a))
    })
}

/// Whether a ward trigger of `id` is waiting to be put on the stack.
fn ward_triggered(cx: &Cx, id: ObjectId) -> bool {
    cx.s().triggers.iter().any(|t| t.source == id && matches!(cx.cards().ability(t.ability), AbilityDef::Triggered(d) if d.ward))
}

#[test]
fn a_cloaked_card_is_a_nameless_two_two_creature_with_ward_two_and_nothing_of_its_own() {
    let (mut game, beast, _, _) = game();
    let mut cx = cx(&mut game);
    let cloaked = face_down(&mut cx, beast, FaceDown::Cloaked).expect("it enters face down");
    let c = cx.chars(cloaked);
    assert!(cx.s().obj(cloaked).face_down);
    assert!(cx.s().obj(cloaked).zone.is(ZoneKind::Battlefield));
    assert_eq!(cx.s().obj(cloaked).controller, YOU);
    assert_eq!((c.power, c.toughness), (Some(2), Some(2)));
    assert!(c.is(CardType::Creature));
    assert_eq!(cx.chars(cloaked).name, "");
    assert!(c.colors.is_empty());
    assert_eq!(cantrip_api::Query::mana_value(&cx, cloaked), 0);
    assert!(has_ward(&cx, cloaked));
    // CR 708.3: it entered face down, which the history keeps.
    assert!(cx.g.log.iter().any(|r| matches!(r.event, Event::EnteredFaceDown { permanent, .. } if permanent == cloaked)));
}

#[test]
fn a_manifested_card_has_no_ward() {
    let (mut game, beast, _, _) = game();
    let mut cx = cx(&mut game);
    let manifested = face_down(&mut cx, beast, FaceDown::Manifested).expect("it enters face down");
    assert!(cx.s().obj(manifested).face_down);
    assert_eq!((cx.chars(manifested).power, cx.chars(manifested).toughness), (Some(2), Some(2)));
    assert!(!has_ward(&cx, manifested));
}

#[test]
fn ward_triggers_when_an_opponent_targets_it_and_not_when_its_controller_does() {
    let (mut game, beast, _, theirs) = game();
    let mut cx = cx(&mut game);
    let cloaked = face_down(&mut cx, beast, FaceDown::Cloaked).expect("it enters face down");
    cx.emit(Event::BecameTarget { target: Target::Object(cloaked), by: theirs, controller: YOU, ability: false });
    assert!(!ward_triggered(&cx, cloaked));
    cx.emit(Event::BecameTarget { target: Target::Object(cloaked), by: theirs, controller: THEM, ability: false });
    assert!(ward_triggered(&cx, cloaked));
}

#[test]
fn a_cloaked_creature_card_is_turned_face_up_for_its_mana_cost_and_loses_ward() {
    let (mut game, beast, _, _) = game();
    let mut cx = cx(&mut game);
    let cloaked = face_down(&mut cx, beast, FaceDown::Cloaked).expect("it enters face down");
    // CR 701.58b: its card's mana cost, {2}, which one mana does not pay.
    cx.g.state.add_mana(YOU, Mana::C, 1, cloaked, None, false);
    assert!(!offered_face_up(&mut cx, cloaked));
    cx.g.state.add_mana(YOU, Mana::C, 1, cloaked, None, false);
    assert!(offered_face_up(&mut cx, cloaked));
    // Its controller alone may (CR 701.58b).
    assert!(!priority_options(&mut cx, THEM).iter().any(|o| o.source == Some(cloaked)));
    ops::turn_face_up(&mut cx, cloaked, YOU);
    assert!(!cx.s().obj(cloaked).face_down);
    assert_eq!(cx.chars(cloaked).name, "Cloak Test Beast");
    assert_eq!((cx.chars(cloaked).power, cx.chars(cloaked).toughness), (Some(4), Some(4)));
    assert!(!has_ward(&cx, cloaked));
    assert!(cx.g.log.iter().any(|r| matches!(r.event, Event::TurnedFaceUp { permanent, .. } if permanent == cloaked)));
}

#[test]
fn a_cloaked_instant_is_not_offered_face_up_and_stays_face_down_if_it_would_turn_up() {
    let (mut game, _, bolt, _) = game();
    let mut cx = cx(&mut game);
    let cloaked = face_down(&mut cx, bolt, FaceDown::Cloaked).expect("it enters face down");
    cx.g.state.add_mana(YOU, Mana::C, 5, cloaked, None, false);
    // CR 701.58b: not a creature card, so it can't be turned face up this way.
    assert!(!offered_face_up(&mut cx, cloaked));
    // CR 701.58g: revealed and left face down, and nothing triggers on its turning face up.
    ops::turn_face_up(&mut cx, cloaked, YOU);
    assert!(cx.s().obj(cloaked).face_down);
    assert!(has_ward(&cx, cloaked));
    assert!(!cx.g.log.iter().any(|r| matches!(r.event, Event::TurnedFaceUp { .. })));
}

#[test]
fn a_cloaked_permanent_that_leaves_is_its_own_card_again() {
    let (mut game, beast, _, _) = game();
    let mut cx = cx(&mut game);
    let cloaked = face_down(&mut cx, beast, FaceDown::Cloaked).expect("it enters face down");
    let card = ops::move_to(&mut cx, cloaked, Zone::hand(YOU));
    assert!(!cx.s().obj(card).face_down);
    assert_eq!(cx.chars(card).name, "Cloak Test Beast");
    assert!(!has_ward(&cx, card));
}

#[test]
fn nothing_is_cloaked_that_is_no_longer_the_chosen_card_or_is_already_on_the_battlefield() {
    let (mut game, beast, _, _) = game();
    let mut cx = cx(&mut game);
    // CR 400.7: the card moved to the hand is a new object; the chosen one is gone.
    ops::move_to(&mut cx, beast, Zone::hand(YOU));
    assert_eq!(face_down(&mut cx, beast, FaceDown::Cloaked), None);
    // CR 708.2b: a face-down permanent can't be turned face down again.
    let (mut game, beast, _, _) = self::game();
    let mut cx = self::cx(&mut game);
    let cloaked = face_down(&mut cx, beast, FaceDown::Cloaked).expect("it enters face down");
    assert_eq!(face_down(&mut cx, cloaked, FaceDown::Manifested), None);
    assert!(has_ward(&cx, cloaked));
}

/// Face-down values with no action to turn the permanent face up: what a permanent an effect turned face down would be
/// (CR 708.2a; Ixidron's), which can't be turned face up for its mana cost (CR 701.40b, 702.37e).
static TURNED_DOWN: CardDef = CardDef {
    name: "",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    face_down: true,
    ..CardDef::BLANK
};

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

/// "All creatures lose all abilities."
static HUMBLER: CardDef = CardDef {
    name: "Cloak Test Humbler",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[loses_all_abilities(creature, "")],
    ..CardDef::BLANK
};

fn turns_face_up(cx: &Cx, id: ObjectId) -> bool {
    cx.chars(id).abilities().any(|a| cx.cards().turns_face_up(a))
}

/// The action to turn a face-down permanent face up is the rules', given back over "loses all abilities" (CR 116.2b), but
/// only to face-down values that have it: a manifested Beast has it with or without the Humbler; a Beast with face-down
/// values that lack it has it in neither case.
#[test]
fn only_face_down_values_with_the_action_are_turned_face_up_and_losing_abilities_does_not_take_it() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAST, &TURNED_DOWN, &HUMBLER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef, zone: Zone| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
        game.state.zone_push(zone, id);
        id
    };
    let manifested = put(&mut game, &BEAST, Zone::library(YOU));
    let turned = put(&mut game, &BEAST, Zone::library(YOU));
    let mut cx = cx(&mut game);
    let manifested = face_down(&mut cx, manifested, FaceDown::Manifested).expect("it enters face down");
    cx.st().update_obj(turned, |o| o.copy = Some(Copiable::of(table.id_of(&TURNED_DOWN))));
    let turned = ops::put_onto_battlefield(&mut cx, &[(turned, YOU, false)], ZoneReason::Other).expect("nothing is asked")[0];
    assert!(cx.s().obj(turned).face_down);
    assert_eq!((cx.chars(turned).name, cx.chars(turned).power), ("", Some(2)));
    assert!(turns_face_up(&cx, manifested) && !turns_face_up(&cx, turned));
    let humbler = cx.st().push_obj(Obj::new(table.id_of(&HUMBLER), YOU, Zone::BATTLEFIELD));
    cx.st().zone_push(Zone::BATTLEFIELD, humbler);
    cx.st().see_statics();
    assert!(turns_face_up(&cx, manifested) && !turns_face_up(&cx, turned));
}
