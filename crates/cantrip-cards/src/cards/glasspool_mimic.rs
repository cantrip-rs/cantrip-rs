use cantrip_api::*;
use cantrip_mechanics::copy::enter_as_copy_of_creature_you_control_and_a_shapeshifter_rogue;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(enter_as_copy_of_creature_you_control_and_a_shapeshifter_rogue),
    ..oracle::GLASSPOOL_MIMIC
};
