use cantrip_api::*;
use cantrip_mechanics::unleash;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(unleash::enters),
    statics: &[unleash::CANT_BLOCK],
    ..oracle::THRILL_KILL_ASSASSIN
};
