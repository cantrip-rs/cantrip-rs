//! A Room at the table (CR 709.5), over test cards: the client is shown the text of its unlocked doors only, and what an
//! unlocked door's static ability does is in the views of the permanents it applies to, as the rules give it.

use std::collections::HashSet;

use cantrip_engine::*;
use cantrip_host::handles::HandleSpace;
use cantrip_host::protocol::*;
use cantrip_host::view::player_view;

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Room View Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn your_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Creature) && q.controller(affected) == q.controller(source)
}

fn five_five(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = Some(5);
    chars.toughness = Some(5);
}

static QUIET_DOOR: CardDef = CardDef {
    name: "Room View Quiet Door",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Room]),
    text: "Nothing happens here.",
    ..CardDef::BLANK
};

static GROWTH_DOOR: CardDef = CardDef {
    name: "Room View Growth Door",
    cost: mana("{2}{G}"),
    text: "Creatures you control have base power and toughness 5/5.",
    statics: &[static_ability(Layer::PtSet, your_creatures, five_five, "Creatures you control have base power and toughness 5/5.")],
    ..QUIET_DOOR
};

static ROOM: CardDef = CardDef { name: "Room View Quiet Door // Room View Growth Door", split: Some((&QUIET_DOOR, &GROWTH_DOOR)), ..QUIET_DOOR };

static BEAR: CardDef = CardDef {
    name: "Room View Bear",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A game with the Room and a Bear on your battlefield, the Room's doors those `doors` unlock.
fn game(doors: u8) -> (Game, ObjectId, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &QUIET_DOOR, &GROWTH_DOOR, &ROOM, &BEAR])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    let room = g.state.push_obj(Obj::new(table.id_of(&ROOM), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, room);
    g.state.update_obj(room, |o| o.marks.doors = doors);
    // What unlocking a door does to the layers' guard (`ops::unlock_door`): an object with static abilities is in the game.
    g.state.statics_seen = true;
    let bear = g.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, bear);
    (g, room, bear)
}

fn view_of(g: &Game, id: ObjectId) -> ObjectView {
    let mut handles = HandleSpace::new();
    let names = vec!["You".to_string(), "Them".to_string()];
    let view = player_view(g, YOU, &mut handles, &names, &[false, false], &HashSet::new(), &HashSet::new(), &HashSet::new());
    let handle = handles.existing(id);
    view.objects.into_iter().find(|o| o.handle == handle).expect("the object is in the view")
}

#[test]
fn a_locked_doors_text_and_static_are_not_shown() {
    let (g, room, bear) = game(DOOR_LEFT);
    let shown = view_of(&g, room);
    assert_eq!(shown.oracle_text, "Nothing happens here.");
    assert_eq!(shown.unlocked_doors, vec![true, false]);
    let bear = view_of(&g, bear);
    assert_eq!((bear.power, bear.toughness), (Some(2), Some(2)));
}

#[test]
fn an_unlocked_doors_text_and_static_are_shown() {
    let (g, room, bear) = game(DOOR_LEFT | DOOR_RIGHT);
    let shown = view_of(&g, room);
    assert_eq!(shown.oracle_text, "Nothing happens here.\nCreatures you control have base power and toughness 5/5.");
    assert_eq!(shown.unlocked_doors, vec![true, true]);
    let bear = view_of(&g, bear);
    assert_eq!((bear.power, bear.toughness), (Some(5), Some(5)));
}
