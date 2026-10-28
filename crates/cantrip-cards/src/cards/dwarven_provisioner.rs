use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}{W}"), effects::creatures_you_control_get::<1, 1>, "{3}{W}: Creatures you control get +1/+1 until end of turn.")],
    ..oracle::DWARVEN_PROVISIONER
};
