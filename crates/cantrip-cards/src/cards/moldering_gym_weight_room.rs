//! Moldering Gym // Weight Room, a Room (CR 709.5): each door has only a "when you unlock this door" trigger, which the
//! engine fires for the door cast and for a door unlocked later by paying its mana cost (CR 709.5h).

use cantrip_api::*;
use cantrip_mechanics::lands::search_basic_land_to_battlefield;
use cantrip_mechanics::manifest_dread::manifest_dread_object;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Moldering Gym: "When you unlock this door, search your library for a basic land card, put it onto the battlefield tapped,
/// then shuffle" (CR 701.23): the search may find none.
fn gym(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_basic_land_to_battlefield(g, item.controller, item.object, true)
}

/// Weight Room: "When you unlock this door, manifest dread, then put three +1/+1 counters on that creature" (CR 701.62a,
/// 122.1, 608.2c): the counters go on the face-down permanent the manifest made, after it; with a library of no cards nothing is
/// manifested and there is no creature to put them on, and one that has left the battlefield by then (CR 400.7) gets none.
fn weight_room(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = manifest_dread_object(g, item.controller, item.object)? else { return Ok(()) };
    g.end_sentence();
    g.add_counters(creature, CounterKind::PlusOne, 3);
    Ok(())
}

pub static MOLDERING_GYM: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        gym,
        "When you unlock this door, search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    )],
    ..oracle::MOLDERING_GYM
};

pub static WEIGHT_ROOM: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_RIGHT_DOOR,
        weight_room,
        "When you unlock this door, manifest dread, then put three +1/+1 counters on that creature.",
    )],
    ..oracle::WEIGHT_ROOM
};

pub static CARD: CardDef = CardDef { split: Some((&MOLDERING_GYM, &WEIGHT_ROOM)), ..oracle::MOLDERING_GYM_WEIGHT_ROOM };

pub static EXTRA: &[&CardDef] = &[&MOLDERING_GYM, &WEIGHT_ROOM];
