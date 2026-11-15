use cantrip_api::*;
use cantrip_mechanics::combat;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    must_be_blocked: &[combat::this_must_be_blocked_if_able],
    ..oracle::FEAR_OF_BEING_HUNTED
};
