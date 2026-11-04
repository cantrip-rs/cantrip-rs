use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "When this creature dies, you gain 1 life." (CR 119.3).
pub static CARD: CardDef = CardDef {
    abilities: &[effects::dies_gain_life::<1>("When this creature dies, you gain 1 life.")],
    ..oracle::TARPAN
};
