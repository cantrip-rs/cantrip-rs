use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, triggers};
use cantrip_oracle as oracle;

/// "Unlock a locked door of up to one target Room you control" (CR 709.5f): nothing if there is no target left; the shared
/// `effects::unlock_a_locked_door` lets the controller choose among the Room's locked doors.
fn unlock_a_locked_door_of_the_target_room(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(room) = targets::object(g, item, 0) else { return Ok(()) };
    effects::unlock_a_locked_door(g, item.controller, item.object, room)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        unlock_a_locked_door_of_the_target_room,
        "Whenever this creature deals combat damage to a player, unlock a locked door of up to one target Room you control.",
    )
    .targets(&[up_to_one(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Room) }>)])],
    ..oracle::GHOSTLY_KEYBEARER
};
