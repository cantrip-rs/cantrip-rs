use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::choose;
use cantrip_mechanics::triggers::EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT;
use cantrip_oracle as oracle;

/// "Draw a card, then you may cast a spell from your hand with mana value less than or equal to that damage without paying
/// its mana cost" (CR 608.2g, 118.9). "That damage" is what the trigger carried from the event (`event_amount`), so it does not
/// change if the creature or the Equipment changes before the ability resolves. The spell cast is the face put on the stack
/// (CR 709.3a, 715.3a), so its mana value is the face's, an X being 0 (CR 107.3b); a land is no spell, and a card with no
/// legal target cannot be cast.
fn draw_then_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let damage = item.matched_damage_amount();
    g.draw(you, 1);
    choose::may_cast_from_hand_free(g, you, item.object, "", |_, w| w.mana_value <= damage)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, draw_then_cast, "Whenever equipped creature deals combat damage to a player, draw a card, then you may cast a spell from your hand with mana value less than or equal to that damage without paying its mana cost.").damage_amount(),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<3, 2>, "Equipped creature gets +3/+2.")],
    ..oracle::BUSTER_SWORD
};
