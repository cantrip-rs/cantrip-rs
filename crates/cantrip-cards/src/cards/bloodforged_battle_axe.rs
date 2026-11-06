use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_oracle as oracle;

/// Create a token that's a copy of this Equipment (CR 707.2, 111.4): the Equipment's copiable values, or its last known
/// ones if it has left the battlefield by now (CR 608.2h). The token is created unattached.
fn copy_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::copy::create_token_copy_of_it(g, item, |_, values| values)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, copy_this, "Whenever equipped creature deals combat damage to a player, create a token that's a copy of this Equipment."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<2, 0>, "Equipped creature gets +2/+0.")],
    ..oracle::BLOODFORGED_BATTLE_AXE
};
