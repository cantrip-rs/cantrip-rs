//! A copy effect's exceptions on a Room (CR 707.9b, 709.5). Over test cards: no printed card in the engine copies a Room
//! with "except its name is X" (Superior Spider-Man copies only a creature card), nor copies a Room spell with exceptions.

use crate::actions::stack_item;
use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static LEFT_DOOR: CardDef = CardDef {
    name: "Copied Left Door",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Room]),
    ..CardDef::BLANK
};
static RIGHT_DOOR: CardDef = CardDef { name: "Copied Right Door", ..LEFT_DOOR };
static ROOM: CardDef = CardDef { name: "Copied Left Door // Copied Right Door", split: Some((&LEFT_DOOR, &RIGHT_DOOR)), ..LEFT_DOOR };

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LEFT_DOOR, &RIGHT_DOOR, &ROOM])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, table)
}

fn put(game: &mut Game, card: CardId, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(card, YOU, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// A Room that is a copy "except its name is X" is named X whichever doors are unlocked: the rule that takes a locked
/// door's name away (CR 709.5) takes no name the exception gave (CR 707.9b).
#[test]
fn a_room_copied_with_a_name_keeps_the_name_its_exception_gives() {
    let (mut game, table) = game();
    let room = put(&mut game, table.id_of(&ROOM), Zone::BATTLEFIELD);
    game.state.update_obj(room, |o| {
        o.copy = Some(Copiable::of(table.id_of(&ROOM)).named("Renamed Room"));
        o.marks.doors = DOOR_LEFT;
    });
    let cx = cx(&mut game);
    assert_eq!(cx.chars(room).name, "Renamed Room");
}

/// A copy of a Room spell cast as its left door becomes a token Room with that door unlocked (CR 707.10f, 709.5d), its
/// copiable values the whole card's with the exceptions the spell had (CR 707.9b, 707.10).
#[test]
fn a_copy_of_a_room_spell_keeps_its_exceptions_as_it_becomes_a_token() {
    let (mut game, table) = game();
    let spell = put(&mut game, table.id_of(&ROOM), Zone::STACK);
    game.state.update_obj(spell, |o| {
        o.cast_way = WAY_SPLIT_LEFT;
        o.copy = Some(Copiable::of(table.id_of(&LEFT_DOOR)).named("Renamed Room"));
    });
    game.state.stack_push(stack_item(StackKind::Spell, spell, AbilityRef { card: table.id_of(&LEFT_DOOR), index: u8::MAX }, YOU, &[]));
    let mut cx = cx(&mut game);
    let copy = ops::copy_spell(&mut cx, spell, YOU, false).ok().flatten().expect("the spell is copied");
    let at = cx.s().stack.len() - 1;
    let item = cx.s().stack[at];
    assert_eq!(item.object, copy);
    ops::resolve_copy_as_token(&mut cx, &item, at).ok().expect("nothing is asked");
    let token = *cx.s().zone(Zone::BATTLEFIELD).last().expect("the copy became a token");
    assert!(cx.s().obj(token).token);
    assert_eq!(cx.s().obj(token).marks.doors, DOOR_LEFT);
    assert_eq!(cx.chars(token).card, table.id_of(&ROOM));
    assert_eq!(cx.chars(token).name, "Renamed Room");
}
