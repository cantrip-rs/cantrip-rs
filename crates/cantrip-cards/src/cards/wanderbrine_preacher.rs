use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_BECOMES_TAPPED, |g, item| effects::gain_life(g, item, 2), "Whenever this creature becomes tapped, you gain 2 life.")],
    ..oracle::WANDERBRINE_PREACHER
};
