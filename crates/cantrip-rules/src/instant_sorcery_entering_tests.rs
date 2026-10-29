//! Instant and sorcery cards can't enter the battlefield; one that would stays in its previous zone (CR 304.4, 307.4, 110.4),
//! over test cards, by every way onto the battlefield: the card judged by the face it would enter with - its back face put
//! there transformed (CR 712.14a), else its front face (a modal double-faced card's, CR 712.14b), or the face-down creature
//! a manifested card is (CR 701.40a). A permanent spell turned over to a sorcery face as it would enter is put into its
//! owner's graveyard (CR 712.13a), and a token that would be a copy of an instant card is not created (CR 111.5). The
//! scenarios show the rest with printed cards (Ephemerate on a cloaked Lightning Bolt and on Summon: Esper Maduin, Valakut
//! Awakening played as its land back face).

use crate::actions::stack_item;
use crate::ops;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static BOLT: CardDef = CardDef { name: "Instant Entering Test Bolt", types: TypeSet::of(&[CardType::Instant]), ..CardDef::BLANK };
static BEAR: CardDef = CardDef {
    name: "Instant Entering Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
/// A modal double-faced card: a sorcery front face, a land back face (Emeria's Call's shape).
static RUINS: CardDef = CardDef { name: "Instant Entering Test Ruins", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static CALL: CardDef = CardDef {
    name: "Instant Entering Test Call",
    types: TypeSet::of(&[CardType::Sorcery]),
    back: Some(&RUINS),
    back_land: true,
    ..CardDef::BLANK
};
/// A daybound creature whose back face is a sorcery: as the example of CR 712.13a has Invasion of Kylem do.
static RITE: CardDef = CardDef { name: "Instant Entering Test Rite", has_cost: false, types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };
static WOLF: CardDef = CardDef {
    name: "Instant Entering Test Wolf",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Daybound]),
    back: Some(&RITE),
    ..CardDef::BLANK
};

/// "It's an enchantment."
static AN_ENCHANTMENT: EntersAs = EntersAs { types: Some(TypeSet::of(&[CardType::Enchantment])), ..EntersAs::NONE };

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BOLT, &BEAR, &CALL, &WOLF])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// An instant card put onto the battlefield, by every way that puts a card there, stays where it is; a creature card put
/// there with it enters.
#[test]
fn an_instant_card_put_onto_the_battlefield_stays_where_it_is() {
    let (mut game, table) = game();
    let bolt = put(&mut game, table, &BOLT, Zone::graveyard(YOU));
    let bear = put(&mut game, table, &BEAR, Zone::graveyard(YOU));
    let mut cx = cx(&mut game);
    ops::put_onto_battlefield(&mut cx, &[(bolt, YOU, false), (bear, YOU, false)], ZoneReason::Other).expect("nothing is asked");
    assert!(cx.s().zone(Zone::graveyard(YOU)).contains(&bolt));
    assert!(cx.s().obj(bear).became.is_some_and(|new| cx.s().battlefield().contains(&new)));
    let after = ops::put_onto_battlefield_as(&mut cx, bolt, YOU, false, false, ZoneReason::Other, &[]).expect("nothing is asked");
    assert_eq!(after, bolt);
    assert!(cx.s().zone(Zone::graveyard(YOU)).contains(&bolt));
    // Not as what the effect would make it either: "it's an enchantment" makes no instant card a permanent card.
    let becomes = Becomes { what: &AN_ENCHANTMENT, card: table.id_of(&BEAR), words: Words::NONE };
    assert_eq!(ops::put_onto_battlefield_becoming(&mut cx, bolt, YOU, becomes).expect("nothing is asked"), None);
    assert!(cx.s().zone(Zone::graveyard(YOU)).contains(&bolt));
}

/// Put onto the battlefield as an effect says it is ("it's an enchantment"), a daybound card at night would enter with its
/// sorcery back face up (CR 702.145b): it stays where it is. By day it enters, an enchantment.
#[test]
fn a_card_put_onto_the_battlefield_becoming_something_turned_over_to_a_sorcery_face_stays_where_it_is() {
    let (mut game, table) = game();
    let wolf = put(&mut game, table, &WOLF, Zone::graveyard(YOU));
    game.state.update_turn(|t| t.day_night = DayNight::Night);
    let mut cx = cx(&mut game);
    let becomes = Becomes { what: &AN_ENCHANTMENT, card: table.id_of(&BEAR), words: Words::NONE };
    assert_eq!(ops::put_onto_battlefield_becoming(&mut cx, wolf, YOU, becomes).expect("nothing is asked"), None);
    assert!(cx.s().zone(Zone::graveyard(YOU)).contains(&wolf));
    cx.st().update_turn(|t| t.day_night = DayNight::Day);
    let entered = ops::put_onto_battlefield_becoming(&mut cx, wolf, YOU, becomes).expect("nothing is asked").expect("it entered");
    assert!(cx.s().battlefield().contains(&entered));
    assert!(cx.chars(entered).is(CardType::Enchantment));
}

/// Manifested, an instant card enters as the face-down 2/2 creature it is (CR 701.40a), not as the instant it is face up.
#[test]
fn an_instant_card_manifested_enters_face_down() {
    let (mut game, table) = game();
    let bolt = put(&mut game, table, &BOLT, Zone::library(YOU));
    let mut cx = cx(&mut game);
    let manifested = ops::put_onto_battlefield_face_down(&mut cx, bolt, YOU, FaceDown::Manifested, false).expect("nothing is asked");
    let manifested = manifested.expect("it entered");
    assert!(cx.s().battlefield().contains(&manifested));
    assert!(cx.s().obj(manifested).face_down);
}

/// A modal double-faced card whose front face is a sorcery stays where it is put onto the battlefield (CR 712.14b), and
/// enters as its land back face put there with that face up (CR 712.12, a land play).
#[test]
fn a_modal_card_with_a_sorcery_front_enters_only_as_its_land_back_face() {
    let (mut game, table) = game();
    let call = put(&mut game, table, &CALL, Zone::hand(YOU));
    let mut cx = cx(&mut game);
    ops::put_onto_battlefield(&mut cx, &[(call, YOU, false)], ZoneReason::Other).expect("nothing is asked");
    assert!(cx.s().zone(Zone::hand(YOU)).contains(&call));
    let land = ops::put_onto_battlefield_as(&mut cx, call, YOU, false, true, ZoneReason::PlayLand, &[]).expect("nothing is asked");
    assert!(cx.s().battlefield().contains(&land));
    assert!(cx.chars(land).is(CardType::Land));
}

/// At night a daybound card enters transformed (CR 702.145b): one whose back face is a sorcery stays where it is, put onto
/// the battlefield; by day it enters front face up.
#[test]
fn a_card_turned_over_to_a_sorcery_face_as_it_enters_stays_where_it_is() {
    let (mut game, table) = game();
    let wolf = put(&mut game, table, &WOLF, Zone::graveyard(YOU));
    game.state.update_turn(|t| t.day_night = DayNight::Night);
    let mut cx = cx(&mut game);
    ops::put_onto_battlefield(&mut cx, &[(wolf, YOU, false)], ZoneReason::Other).expect("nothing is asked");
    assert!(cx.s().zone(Zone::graveyard(YOU)).contains(&wolf));
    assert_eq!(ops::put_onto_battlefield_as(&mut cx, wolf, YOU, false, false, ZoneReason::Other, &[]).expect("nothing is asked"), wolf);
    assert!(cx.s().zone(Zone::graveyard(YOU)).contains(&wolf));
    cx.st().update_turn(|t| t.day_night = DayNight::Day);
    ops::put_onto_battlefield(&mut cx, &[(wolf, YOU, false)], ZoneReason::Other).expect("nothing is asked");
    assert!(cx.s().obj(wolf).became.is_some_and(|new| cx.s().battlefield().contains(&new)));
}

/// The same card resolving as a creature spell at night would enter with its sorcery face up: it doesn't enter, and is put
/// into its owner's graveyard instead (CR 712.13a). By day it enters.
#[test]
fn a_permanent_spell_turned_over_to_a_sorcery_face_as_it_resolves_goes_to_the_graveyard() {
    for (time, enters) in [(DayNight::Night, false), (DayNight::Day, true)] {
        let (mut game, table) = game();
        let wolf = put(&mut game, table, &WOLF, Zone::STACK);
        game.state.stack_push(stack_item(StackKind::Spell, wolf, AbilityRef::spell(table.id_of(&WOLF)), YOU, &[]));
        let serial = game.state.stack.last().expect("the Wolf").serial;
        game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
        game.state.update_turn(|t| t.day_night = time);
        let frame = game.state.agenda.len() - 1;
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        crate::resolve::resolve(&mut cx).expect("nothing is asked");
        let new = cx.s().obj(wolf).became.expect("it left the stack");
        assert!(cx.s().stack.is_empty());
        assert_eq!(cx.s().obj(new).zone, if enters { Zone::BATTLEFIELD } else { Zone::graveyard(YOU) });
    }
}

/// A token that would be a copy of an instant card is not created (CR 111.5); one that would be a copy of a creature card is.
#[test]
fn a_token_copy_of_an_instant_card_is_not_created() {
    let (mut game, table) = game();
    let mut cx = cx(&mut game);
    let none = ops::create_token_copies(&mut cx, Copiable::of(table.id_of(&BOLT)), YOU, 1).expect("nothing is asked");
    assert!(none.is_empty());
    // Nor one with an exception to its copiable values ("except its name is ..."): still a copy of an instant card.
    let renamed = Copiable { name: Some("Instant Entering Test Renamed"), ..Copiable::of(table.id_of(&BOLT)) };
    assert!(renamed.has_exceptions());
    let none = ops::create_token_copies(&mut cx, renamed, YOU, 1).expect("nothing is asked");
    assert!(none.is_empty());
    assert!(cx.s().battlefield().is_empty());
    let bears = ops::create_token_copies(&mut cx, Copiable::of(table.id_of(&BEAR)), YOU, 1).expect("nothing is asked");
    assert_eq!(bears.len(), 1);
    assert!(cx.s().battlefield().contains(&bears[0]));
}
