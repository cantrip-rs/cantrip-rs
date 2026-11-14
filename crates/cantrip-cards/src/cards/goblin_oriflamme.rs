use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { statics: &[statics::ATTACKING_CREATURES_YOU_CONTROL_GET_PLUS_ONE_POWER], ..oracle::GOBLIN_ORIFLAMME };
