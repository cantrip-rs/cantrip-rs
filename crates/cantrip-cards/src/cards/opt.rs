use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], effects::scry_then_draw::<1>), ..oracle::OPT };
