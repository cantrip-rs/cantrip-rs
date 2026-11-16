//! Mirror Room // Fractured Realm, a Room (CR 709.5): each door is only what its text does; the Room has the abilities of
//! its unlocked doors (`layers::doors`, `CardTable::texts`).

use cantrip_api::*;
use cantrip_mechanics::{copy, targets, triggers};
use cantrip_oracle as oracle;

/// "Create a token that's a copy of target creature you control, except it's a Reflection in addition to its other
/// creature types" (CR 707.2, 707.9b): the target's copiable values, with Reflection (a word of the Room's text, CR 612)
/// added. A creature no longer there, or no longer the controller's, is not copied (CR 608.2b).
fn copy_target_creature_as_a_reflection(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    copy::create_token_copy_with_subtype(g, item.object, item.controller, creature, Subtype::Reflection, false)?;
    Ok(())
}

pub static MIRROR_ROOM: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        copy_target_creature_as_a_reflection,
        "When you unlock this door, create a token that's a copy of target creature you control, except it's a Reflection in addition to its other creature types.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::MIRROR_ROOM
};

/// "If a triggered ability of a permanent you control triggers, that ability triggers an additional time" (CR 603.2d): the
/// ability's source a permanent under the Room's controller as it is when the ability triggers (as it last existed for one
/// that looks back), the Room itself among them.
fn a_permanent_you_control(_: &dyn Query, asker: &Asker, _: &Event, triggering: &Triggering) -> bool {
    triggering.of_a_permanent_controlled_by(asker.controller)
}

pub static FRACTURED_REALM: CardDef = CardDef { additional_triggers: &[a_permanent_you_control], ..oracle::FRACTURED_REALM };

pub static CARD: CardDef = CardDef { split: Some((&MIRROR_ROOM, &FRACTURED_REALM)), ..oracle::MIRROR_ROOM_FRACTURED_REALM };

pub static EXTRA: &[&CardDef] = &[&MIRROR_ROOM, &FRACTURED_REALM];
