use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE], library::owner_puts_first_target_on_top_or_bottom_then_surveil::<1>),
    ..oracle::LOST_IN_SPACE
};
