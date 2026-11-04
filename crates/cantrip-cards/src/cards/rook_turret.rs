use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_ARTIFACT_YOU_CONTROL_ENTERS,
        effects::may_draw_then_discard,
        "Whenever another artifact you control enters, you may draw a card. If you do, discard a card.",
    )],
    ..oracle::ROOK_TURRET
};
