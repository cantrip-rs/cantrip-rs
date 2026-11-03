use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{5}"), effects::creatures_you_control_get::<1, 1>, "{5}: Creatures you control get +1/+1 until end of turn.")],
    ..oracle::WATER_TRIBE_CAPTAIN
};
