//! What a Room's doors give it (CR 709.5), over test cards: an unlocked door's text is the permanent's (`CardTable::texts`
//! walks it after the Room's card) and a locked door's is not; its triggered abilities are looked for as the Room's - a state
//! trigger (CR 603.8), a triggered mana ability (CR 605.1b) - and a door with text the walk can't give the Room is refused
//! (`CardDef::door_unsupported`), never quietly absent. The look-back case (CR 603.10a) is in `look_back_tests`.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn gain_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

/// "When you control no creatures, you gain 2 life."
fn you_control_no_creatures(q: &dyn Query, me: &Asker) -> bool {
    !q.state().battlefield().iter().any(|&id| q.state().obj(id).controller == me.controller && q.chars(id).is(CardType::Creature))
}

static STATE_DOOR: CardDef = CardDef {
    name: "State Door",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Room]),
    abilities: &[state_triggered(you_control_no_creatures, gain_two, "When you control no creatures, you gain 2 life.")],
    ..CardDef::BLANK
};
static PLAIN_DOOR: CardDef = CardDef { name: "Plain Door", abilities: &[], ..STATE_DOOR };
static STATE_ROOM: CardDef = CardDef { name: "State Door // Plain Door", split: Some((&STATE_DOOR, &PLAIN_DOOR)), abilities: &[], ..STATE_DOOR };

fn never(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
    None
}

/// "Whenever a land is tapped for mana, its controller adds an additional {G}" (never matched here: only looked for).
static MANA_DOOR: CardDef = CardDef {
    name: "Mana Door",
    abilities: &[triggered(when(EventMask::TAPPED, never), gain_two, "").mana_ability(TriggeredMana::One(Mana::G))],
    ..PLAIN_DOOR
};
static MANA_ROOM: CardDef = CardDef { name: "Plain Door // Mana Door", split: Some((&PLAIN_DOOR, &MANA_DOOR)), ..PLAIN_DOOR };

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// A Room with its state-trigger door unlocked triggers as soon as its state is true (CR 603.8), as an effect begins in the
/// middle of a resolution (`settle::state_triggers_now`): the door's ability is the Room's, though not the Room card's own.
#[test]
fn an_unlocked_doors_state_trigger_triggers() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&STATE_DOOR, &PLAIN_DOOR, &STATE_ROOM])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let room = game.state.push_obj(Obj::new(table.id_of(&STATE_ROOM), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, room);
    game.state.update_obj(room, |o| o.marks.doors = DOOR_LEFT);
    let mut cx = cx(&mut game);
    crate::settle::state_triggers_now(&mut cx);
    assert!(cx.s().triggers.iter().any(|t| t.source == room));
}

/// A Room whose door prints a triggered mana ability is among the permanents looked at when a permanent is tapped for mana
/// (CR 605.1b), so the ability is found once the door is unlocked.
#[test]
fn a_room_with_a_doors_triggered_mana_ability_is_looked_at() {
    let table = CardTable::new([&STATE_DOOR, &PLAIN_DOOR, &STATE_ROOM, &MANA_DOOR, &MANA_ROOM]);
    assert!(table.mana_triggers[table.id_of(&MANA_ROOM).0 as usize]);
    assert!(!table.mana_triggers[table.id_of(&STATE_ROOM).0 as usize]);
}

fn stops_nothing(_: &dyn Query, _: &Asker, _: &Event, _: &Triggering) -> bool {
    false
}

fn changes_nothing(_: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::NONE
}

fn allows_everything(_: &dyn Query, _: &Asker, _: PlayerId, _: ObjectId, _: &ActivatedDef) -> bool {
    false
}

fn whenever_in_a_graveyard(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
    None
}

static TRIGGER_RULE_DOOR: CardDef = CardDef { name: "Trigger Rule Door", trigger_rules: &[stops_nothing], ..PLAIN_DOOR };
static ADDITIONAL_DOOR: CardDef = CardDef { name: "Additional Door", additional_triggers: &[stops_nothing], ..PLAIN_DOOR };
static COST_DOOR: CardDef = CardDef { name: "Cost Door", cost_changes: &[changes_nothing], ..PLAIN_DOOR };
static ACTIVATION_DOOR: CardDef = CardDef { name: "Activation Door", activation_rules: &[allows_everything], ..PLAIN_DOOR };
static ANYWHERE_DOOR: CardDef = CardDef {
    name: "Anywhere Door",
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, whenever_in_a_graveyard), gain_two, "").functions_in(IN_A_GRAVEYARD)],
    ..PLAIN_DOOR
};
static KEYWORD_DOOR: CardDef = CardDef { name: "Keyword Door", keywords: Keywords::of(&[Keyword::Flying]), ..PLAIN_DOOR };

/// Each kind of text the walk can't give the Room is refused on a door, by name; a door with abilities (a state trigger, a
/// triggered mana ability) or standing rules (trigger rules, additional triggers, cost changes, activation rules) is not,
/// as an unlocked door's are read through `CardTable::texts` as any card's.
#[test]
fn a_door_with_text_the_room_would_not_have_is_refused() {
    assert_eq!(TRIGGER_RULE_DOOR.door_unsupported(), None);
    assert_eq!(ADDITIONAL_DOOR.door_unsupported(), None);
    assert_eq!(COST_DOOR.door_unsupported(), None);
    assert_eq!(ACTIVATION_DOOR.door_unsupported(), None);
    assert_eq!(ANYWHERE_DOOR.door_unsupported(), Some("a triggered ability that functions outside the battlefield"));
    assert_eq!(KEYWORD_DOOR.door_unsupported(), Some("keywords"));
    assert_eq!(PLAIN_DOOR.door_unsupported(), None);
    assert_eq!(STATE_DOOR.door_unsupported(), None);
    assert_eq!(MANA_DOOR.door_unsupported(), None);
}

static REFUSED_ROOM: CardDef = CardDef { name: "Plain Door // Keyword Door", split: Some((&PLAIN_DOOR, &KEYWORD_DOOR)), ..PLAIN_DOOR };

/// The card table refuses a Room with such a door outright.
#[test]
#[should_panic(expected = "Keyword Door: a door with keywords is not supported yet (CR 709.5)")]
fn the_card_table_refuses_a_room_with_such_a_door() {
    let _ = CardTable::new([&PLAIN_DOOR, &KEYWORD_DOOR, &REFUSED_ROOM]);
}

static RULES_ROOM: CardDef = CardDef { name: "Cost Door // Additional Door", split: Some((&COST_DOOR, &ADDITIONAL_DOOR)), ..PLAIN_DOOR };

/// A Room's texts are its card's, then those of its unlocked doors, left first (CR 709.5): with no unlocked designation -
/// in any zone but the battlefield too (CR 709.5c) - its card's alone, so a locked door's standing rules are not looked
/// for; once a door is unlocked, its own card's flags find it as they find any card. A copy of the Room has the doors its
/// own designations unlock, and a face-down Room none (CR 708.2).
#[test]
fn a_rooms_texts_are_its_card_and_its_unlocked_doors() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&COST_DOOR, &ADDITIONAL_DOOR, &RULES_ROOM, &STATE_DOOR, &PLAIN_DOOR, &STATE_ROOM])));
    let (room_card, left, right) = (table.id_of(&RULES_ROOM), table.id_of(&COST_DOOR), table.id_of(&ADDITIONAL_DOOR));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let room = game.state.push_obj(Obj::new(room_card, YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, room);
    let texts = |game: &Game, id: ObjectId| table.texts(&game.state, game.state.obj(id)).collect::<Vec<_>>();
    let linked = |game: &Game, id: ObjectId| table.texts_linked(&game.state, id).collect::<Vec<_>>();
    assert_eq!(texts(&game, room), [room_card]);
    assert!(!texts(&game, room).iter().any(|c| table.cost_changes[c.0 as usize] || table.trigger_rules[c.0 as usize]));
    game.state.update_obj(room, |o| o.marks.doors = DOOR_RIGHT);
    assert_eq!(texts(&game, room), [room_card, right]);
    assert!(texts(&game, room).iter().any(|c| table.trigger_rules[c.0 as usize]));
    assert!(!texts(&game, room).iter().any(|c| table.cost_changes[c.0 as usize]));
    game.state.update_obj(room, |o| o.marks.doors = DOOR_LEFT | DOOR_RIGHT);
    assert_eq!(texts(&game, room), [room_card, left, right]);
    // Each door's text is the Room's own, linked as the Room's card (CR 607.1).
    assert_eq!(linked(&game, room), [(room_card, None), (left, None), (right, None)]);
    assert_eq!(table.texts_linked_with_exclusions(&game.state, room).map(|(c, l, _)| (c, l)).collect::<Vec<_>>(), linked(&game, room));
    // A copy of the Room (CR 707.2): its copiable values are the Room's, its designations its own.
    let other = game.state.push_obj(Obj::new(table.id_of(&STATE_ROOM), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, other);
    let copy = Copiable::of(room_card);
    game.state.update_obj(other, |o| o.copy = Some(copy));
    assert_eq!(texts(&game, other), [room_card]);
    game.state.update_obj(other, |o| o.marks.doors = DOOR_LEFT);
    assert_eq!(texts(&game, other), [room_card, left]);
}

/// Locking a door (CR 709.5g) removes only that half's unlocked designation, does nothing to a door already locked, and the
/// Room stays the same object (CR 709.5c); a permanent that is not a Room has no doors to lock.
#[test]
fn locking_a_door_removes_that_doors_designation() {
    static NOT_A_ROOM: CardDef = CardDef { name: "Not A Room", split: None, subtypes: SubtypeSet::of(&[]), ..PLAIN_DOOR };
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&STATE_DOOR, &PLAIN_DOOR, &STATE_ROOM, &NOT_A_ROOM])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let room = game.state.push_obj(Obj::new(table.id_of(&STATE_ROOM), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, room);
    game.state.update_obj(room, |o| o.marks.doors = DOOR_LEFT | DOOR_RIGHT);
    let other = game.state.push_obj(Obj::new(table.id_of(&NOT_A_ROOM), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, other);
    game.state.update_obj(other, |o| o.marks.doors = DOOR_LEFT);
    let mut cx = cx(&mut game);
    crate::ops::lock_door(&mut cx, room, DOOR_LEFT);
    assert_eq!(cx.s().obj(room).marks.doors, DOOR_RIGHT);
    crate::ops::lock_door(&mut cx, room, DOOR_LEFT);
    assert_eq!(cx.s().obj(room).marks.doors, DOOR_RIGHT);
    crate::ops::lock_door(&mut cx, other, DOOR_LEFT);
    assert_eq!(cx.s().obj(other).marks.doors, DOOR_LEFT);
    crate::ops::lock_door(&mut cx, room, DOOR_RIGHT);
    assert_eq!(cx.s().obj(room).marks.doors, 0);
}

fn your_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Creature) && q.controller(affected) == q.controller(source)
}

fn five_five(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = Some(5);
    chars.toughness = Some(5);
}

/// "Creatures you control have base power and toughness 5/5."
static PT_DOOR: CardDef = CardDef {
    name: "Pt Door",
    statics: &[static_ability(Layer::PtSet, your_creatures, five_five, "Creatures you control have base power and toughness 5/5.")],
    ..PLAIN_DOOR
};
static PT_ROOM: CardDef = CardDef { name: "Pt Door // Plain Door", split: Some((&PT_DOOR, &PLAIN_DOOR)), ..PLAIN_DOOR };
static BEAR: CardDef = CardDef { name: "Bear", types: TypeSet::of(&[CardType::Creature]), subtypes: SubtypeSet::of(&[]), power: Some(2), toughness: Some(2), ..PLAIN_DOOR };

/// A door's static ability applies while that door is unlocked and only then (CR 709.5): unlocking it turns the effect on at
/// once, with the Room's timestamp (CR 613.7a), locking it (CR 709.5g) turns it off at once, and unlocking or locking the
/// other door leaves it as it was.
#[test]
fn a_doors_static_ability_applies_while_the_door_is_unlocked() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&PT_DOOR, &PLAIN_DOOR, &PT_ROOM, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let room = game.state.push_obj(Obj::new(table.id_of(&PT_ROOM), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, room);
    let bear = game.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, bear);
    let pt = |game: &Game| (game.chars(bear).power, game.chars(bear).toughness);
    assert_eq!(pt(&game), (Some(2), Some(2)));
    let mut cx = cx(&mut game);
    crate::ops::unlock_door(&mut cx, room, DOOR_RIGHT, YOU);
    assert_eq!(pt(&game), (Some(2), Some(2)));
    let mut cx = crate::room_door_tests::cx(&mut game);
    crate::ops::unlock_door(&mut cx, room, DOOR_LEFT, YOU);
    assert_eq!(pt(&game), (Some(5), Some(5)));
    let mut cx = crate::room_door_tests::cx(&mut game);
    crate::ops::lock_door(&mut cx, room, DOOR_RIGHT);
    assert_eq!(pt(&game), (Some(5), Some(5)));
    let mut cx = crate::room_door_tests::cx(&mut game);
    crate::ops::lock_door(&mut cx, room, DOOR_LEFT);
    assert_eq!(pt(&game), (Some(2), Some(2)));
}

fn gives_the_cards_delayed_ability(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let card = view.text_of(source);
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(AbilityRef::delayed(card, 0));
    }
}

/// "Creatures you control have [the card's delayed ability]."
static GRANT_DOOR: CardDef = CardDef {
    name: "Grant Door",
    delayed: &[triggered(when(EventMask::TAPPED, never), gain_two, "Whenever a land is tapped, gain two life.")],
    statics: &[static_ability(Layer::Ability, your_creatures, gives_the_cards_delayed_ability, "Creatures you control have it.")],
    ..PLAIN_DOOR
};
static GRANT_ROOM: CardDef = CardDef { name: "Plain Door // Grant Door", split: Some((&PLAIN_DOOR, &GRANT_DOOR)), ..PLAIN_DOOR };

/// What an unlocked door's static ability gives "the card's" ability names is printed on the door, not the Room's own card
/// (CR 709.5, 607.1): `LayerQuery::text_of` answers the door for its static abilities, which are linked as the Room's.
#[test]
fn a_doors_static_ability_names_the_doors_text() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&PLAIN_DOOR, &GRANT_DOOR, &GRANT_ROOM, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let room = game.state.push_obj(Obj::new(table.id_of(&GRANT_ROOM), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, room);
    let bear = game.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, bear);
    assert!(game.chars(bear).abilities.iter().all(|a| a.is_none()));
    let mut cx = crate::room_door_tests::cx(&mut game);
    crate::ops::unlock_door(&mut cx, room, DOOR_RIGHT, YOU);
    let given: Vec<_> = game.chars(bear).abilities.iter().flatten().copied().collect();
    assert_eq!(given, vec![AbilityRef::delayed(table.id_of(&GRANT_DOOR), 0)]);
}

/// A face-down permanent has none of its card's text (CR 708.2): a face-down Room's doors give it nothing, whatever
/// designations it has: its unlocked door's static ability applies to nothing.
#[test]
fn a_face_down_rooms_doors_do_nothing() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&PT_DOOR, &PLAIN_DOOR, &PT_ROOM, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let room = game.state.push_obj(Obj::new(table.id_of(&PT_ROOM), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, room);
    let bear = game.state.push_obj(Obj::new(table.id_of(&BEAR), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, bear);
    let mut cx = cx(&mut game);
    crate::ops::unlock_door(&mut cx, room, DOOR_LEFT, YOU);
    assert_eq!((game.chars(bear).power, game.chars(bear).toughness), (Some(5), Some(5)));
    let down = table.id_of(&BEAR);
    game.state.update_obj(room, |o| {
        o.face_down = true;
        o.marks.face_down_as = Some(down);
    });
    assert_eq!(game.state.obj(room).marks.doors, DOOR_LEFT);
    assert_eq!(table.texts(&game.state, game.state.obj(room)).collect::<Vec<_>>(), [down]);
    assert_eq!((game.chars(bear).power, game.chars(bear).toughness), (Some(2), Some(2)));
}
