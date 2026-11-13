use cantrip_api::*;
use cantrip_mechanics::raid;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(raid::enters_with_a_counter),
    ..oracle::GOBLIN_BOARDERS
};
