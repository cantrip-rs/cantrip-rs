use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[cantrip_mechanics::statics::CREATURES_YOU_CONTROL_GET_PLUS_ONE],
    abilities: &[triggered(cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ENTERS, cantrip_mechanics::effects::deals_1_damage_to_each_opponent, "Whenever a creature you control enters, this enchantment deals 1 damage to each opponent.")],
    ..oracle::WARLEADERS_CALL
};
