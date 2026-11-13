use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_DIES;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_OR_DIES, effects::you_mill::<2>, "When this creature enters or dies, mill two cards.")],
    ..oracle::CROW_OF_DARK_TIDINGS
};
