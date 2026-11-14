use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER, tokens::create_a_treasure, "Whenever equipped creature deals combat damage to a player, create a Treasure token."),
        equip(mana("{1}"), "Equip {1}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    ..oracle::GOLDVEIN_PICK
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
