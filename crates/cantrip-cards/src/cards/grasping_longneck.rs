use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Reach" (the content JSON) and "When this creature dies, you gain 2 life." (CR 119.3).
pub static CARD: CardDef = CardDef {
    abilities: &[effects::dies_gain_life::<2>("When this creature dies, you gain 2 life.")],
    ..oracle::GRASPING_LONGNECK
};
