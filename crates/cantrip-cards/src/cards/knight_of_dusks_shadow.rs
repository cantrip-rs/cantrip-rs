use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::opponents_cant_gain_life;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    cant_gain_life: &[opponents_cant_gain_life],
    abilities: &[ability(cost("{1}{B}"), effects::this_gets::<1, 1>, "{1}{B}: This creature gets +1/+1 until end of turn.")],
    ..oracle::KNIGHT_OF_DUSKS_SHADOW
};
