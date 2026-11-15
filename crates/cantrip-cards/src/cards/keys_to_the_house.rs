use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

/// "Lock or unlock a door of target Room you control": the controller chooses the door as the ability resolves (CR 709.5f,
/// 709.5g). Nothing if the target is gone or no longer a legal target (CR 608.2b).
fn lock_or_unlock_a_door_of_target_room(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(room) = targets::object(g, item, 0) else { return Ok(()) };
    effects::lock_or_unlock_a_door(g, item.controller, item.object, room)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}, {T}").and(SACRIFICE_THIS),
            |g, item| lands::search_basic_land_to_hand(g, item.controller, item.object),
            "{1}, {T}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        ),
        ability(
            cost("{3}, {T}").and(SACRIFICE_THIS),
            lock_or_unlock_a_door_of_target_room,
            "{3}, {T}, Sacrifice this artifact: Lock or unlock a door of target Room you control. Activate only as a sorcery.",
        )
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Room) }>)])
        .sorcery_speed(),
    ],
    ..oracle::KEYS_TO_THE_HOUSE
};
