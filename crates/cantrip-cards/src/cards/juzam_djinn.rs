use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            effects::source_deals_one_to_controller,
            "At the beginning of your upkeep, this creature deals 1 damage to you.",
        ),
    ],
    ..oracle::JUZ_M_DJINN
};
