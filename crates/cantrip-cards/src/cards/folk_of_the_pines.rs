use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{G}"), effects::this_gets::<1, 0>, "{1}{G}: This creature gets +1/+0 until end of turn.")],
    ..oracle::FOLK_OF_THE_PINES
};
