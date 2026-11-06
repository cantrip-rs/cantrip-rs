//! What a Room's doors give it (CR 709.5), over test cards: an unlocked door's triggered abilities are looked for as the
//! Room's - a state trigger (CR 603.8), a triggered mana ability (CR 605.1b) - and a door with text the engine reads off the
//! Room card instead of its doors is refused (`CardDef::door_unsupported`), never quietly absent. The look-back case
//! (CR 603.10a) is in `look_back_tests`.

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
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
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

fn allows_everything(_: &dyn Query, _: &Asker, _: ObjectId, _: &ActivatedDef) -> bool {
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

/// Each kind of text the engine reads off the Room card rather than its doors is refused on a door, by name; a door with
/// only abilities (a state trigger, a triggered mana ability) is not.
#[test]
fn a_door_with_text_the_room_would_not_have_is_refused() {
    assert_eq!(TRIGGER_RULE_DOOR.door_unsupported(), Some("trigger rules"));
    assert_eq!(ADDITIONAL_DOOR.door_unsupported(), Some("additional triggers"));
    assert_eq!(COST_DOOR.door_unsupported(), Some("cost changes"));
    assert_eq!(ACTIVATION_DOOR.door_unsupported(), Some("activation rules"));
    assert_eq!(ANYWHERE_DOOR.door_unsupported(), Some("a triggered ability that functions outside the battlefield"));
    assert_eq!(KEYWORD_DOOR.door_unsupported(), Some("keywords"));
    assert_eq!(PLAIN_DOOR.door_unsupported(), None);
    assert_eq!(STATE_DOOR.door_unsupported(), None);
    assert_eq!(MANA_DOOR.door_unsupported(), None);
}

static REFUSED_ROOM: CardDef =
    CardDef { name: "Plain Door // Trigger Rule Door", split: Some((&PLAIN_DOOR, &TRIGGER_RULE_DOOR)), ..PLAIN_DOOR };

/// The card table refuses a Room with such a door outright.
#[test]
#[should_panic(expected = "Trigger Rule Door: a door with trigger rules is not supported yet (CR 709.5)")]
fn the_card_table_refuses_a_room_with_such_a_door() {
    let _ = CardTable::new([&PLAIN_DOOR, &TRIGGER_RULE_DOOR, &REFUSED_ROOM]);
}
