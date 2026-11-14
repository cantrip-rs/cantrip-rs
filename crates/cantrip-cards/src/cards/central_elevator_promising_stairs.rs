//! Central Elevator // Promising Stairs, a Room (CR 709.5): see `unholy_annex_ritual_chamber` for how a door is a half of the card.

use cantrip_api::*;
use cantrip_mechanics::triggers::{self, YOUR_UPKEEP};
use cantrip_mechanics::{effects, library, same_name, select};
use cantrip_oracle as oracle;

/// The Rooms `player` controls, a Room being a permanent with the subtype as `source`'s text has it (CR 612.2).
fn rooms_you_control(q: &dyn Query, source: ObjectId, player: PlayerId) -> Vec<ObjectId> {
    let room = q.words(source).subtype(Subtype::Room);
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).has_subtype(room)).collect()
}

/// "A Room card that doesn't have the same name as a Room you control" (CR 201.2a): a Room card in the library, a split card
/// with both names (CR 709.4a), against a Room on the battlefield with the names of its unlocked doors (CR 709.5, `same_name`).
fn room_named_unlike_one_you_control(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let rooms = rooms_you_control(q, choice.source, choice.chooser);
    Verdict::each(choice, |card| !rooms.iter().any(|&room| same_name(q, card, room)))
}

const ROOM_CARD_NAMED_UNLIKE_YOURS: Selection =
    Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Room), select::part(&(room_named_unlike_one_you_control as SelectFn), 0, "")])
        .called("Room");

/// Central Elevator: "When you unlock this door, search your library for a Room card that doesn't have the same name as a Room
/// you control, reveal it, put it into your hand, then shuffle." A Room card with no name in common may be found, or none (CR 701.23b).
fn elevator(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, ROOM_CARD_NAMED_UNLIKE_YOURS, true)?;
    Ok(())
}

/// "Eight or more different names among unlocked doors of Rooms you control" (CR 201.2b, 709.5): the names a Room on the
/// battlefield has are those of its unlocked doors alone (`library::object_names`), counted once each.
fn different_names_among_your_unlocked_doors(q: &dyn Query, source: ObjectId, player: PlayerId) -> usize {
    let mut names: Vec<&'static str> = Vec::new();
    for room in rooms_you_control(q, source, player) {
        for name in library::object_names(q, room) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names.len()
}

/// Promising Stairs: "At the beginning of your upkeep, surveil 1. You win the game if there are eight or more different names
/// among unlocked doors of Rooms you control." The names are counted after the surveil (CR 608.2c).
fn stairs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)?;
    g.end_sentence();
    if different_names_among_your_unlocked_doors(g, item.object, item.controller) >= 8 {
        g.win_game(item.controller);
    }
    Ok(())
}

pub static CENTRAL_ELEVATOR: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        elevator,
        "When you unlock this door, search your library for a Room card that doesn't have the same name as a Room you control, reveal it, put it into your hand, then shuffle.",
    )],
    ..oracle::CENTRAL_ELEVATOR
};

pub static PROMISING_STAIRS: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_UPKEEP,
        stairs,
        "At the beginning of your upkeep, surveil 1. You win the game if there are eight or more different names among unlocked doors of Rooms you control.",
    )],
    ..oracle::PROMISING_STAIRS
};

pub static CARD: CardDef = CardDef { split: Some((&CENTRAL_ELEVATOR, &PROMISING_STAIRS)), ..oracle::CENTRAL_ELEVATOR_PROMISING_STAIRS };

pub static EXTRA: &[&CardDef] = &[&CENTRAL_ELEVATOR, &PROMISING_STAIRS];
