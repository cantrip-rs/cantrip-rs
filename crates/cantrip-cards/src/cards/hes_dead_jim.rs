use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy target creature. Surveil 1." (CR 701.8, 701.25, 608.2b): identical to Hour of Defeat.
pub static CARD: CardDef = CardDef { spell: super::hour_of_defeat::CARD.spell, ..oracle::HES_DEAD_JIM };
