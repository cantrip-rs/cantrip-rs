use cantrip_api::*;
use cantrip_mechanics::{effects, lands, library, targets, top_of_library, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Reveal the top seven cards of your library. Put all enchantment cards from among them into your hand and the rest on the
/// bottom of your library in a random order." (CR 701.20, 401.4). All of them go, so nobody is asked; a library with fewer
/// cards reveals what it has.
fn reveal_seven_take_the_enchantments(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed = top_of_library(g, you, 7);
    if revealed.is_empty() {
        return Ok(());
    }
    g.reveal(you, &revealed);
    let (enchantments, rest): (Vec<ObjectId>, Vec<ObjectId>) = revealed.into_iter().partition(|&card| g.chars(card).is(CardType::Enchantment));
    g.return_to_hand_revealed(&enchantments);
    library::rest_to_bottom_in_random_order(g, you, &rest);
    Ok(())
}

/// "Lock or unlock a door of target Room you control" (CR 709.5f, 709.5g).
fn lock_or_unlock_a_door_of_the_room(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(room) = targets::object(g, item, 0) else { return Ok(()) };
    effects::lock_or_unlock_a_door(g, item.controller, item.object, room)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, reveal_seven_take_the_enchantments, "When Marina Vendrell enters, reveal the top seven cards of your library. Put all enchantment cards from among them into your hand and the rest on the bottom of your library in a random order."),
        ability(TAP, lock_or_unlock_a_door_of_the_room, "{T}: Lock or unlock a door of target Room you control. Activate only as a sorcery.")
            .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Room) }>)])
            .sorcery_speed(),
    ],
    ..oracle::MARINA_VENDRELL
};
