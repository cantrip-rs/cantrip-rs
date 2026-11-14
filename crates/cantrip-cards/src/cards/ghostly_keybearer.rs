use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, triggers};
use cantrip_oracle as oracle;

/// "Unlock a locked door of up to one target Room you control" (CR 709.5f): the controller chooses a locked door of the Room as
/// the ability resolves, which is no choice at all with one locked door; a Room that returned to the battlefield not cast has
/// neither door unlocked (CR 709.5d). Nothing if there is no target left or the Room has no locked door. A Room with both doors
/// unlocked is still a legal target.
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
