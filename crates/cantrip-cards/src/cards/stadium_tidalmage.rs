use cantrip_api::*;
use cantrip_mechanics::{effects::may_draw_then_discard, triggers::THIS_ENTERS_OR_ATTACKS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_OR_ATTACKS, may_draw_then_discard, "Whenever this creature enters or attacks, you may draw a card. If you do, discard a card.")],
    ..oracle::STADIUM_TIDALMAGE
};
