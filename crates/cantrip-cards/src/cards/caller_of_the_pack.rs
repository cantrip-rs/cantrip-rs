use cantrip_api::*;
use cantrip_mechanics::myriad;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[myriad::TRIGGER],
    delayed: &[myriad::EXILE],
    ..oracle::CALLER_OF_THE_PACK
};
