use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_BECOMES_TAPPED, effects::loot, "Whenever this creature becomes tapped, draw a card, then discard a card.")],
    ..oracle::SILVERGILL_PEDDLER
};
