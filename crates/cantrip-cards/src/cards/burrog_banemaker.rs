use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Deathtouch is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{B}"), effects::this_gets::<1, 1>, "{1}{B}: This creature gets +1/+1 until end of turn.")],
    ..oracle::BURROG_BANEMAKER
};
