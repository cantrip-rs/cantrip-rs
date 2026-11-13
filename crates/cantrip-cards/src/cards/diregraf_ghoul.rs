use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { enters: Some(mana::enters_tapped), ..oracle::DIREGRAF_GHOUL };
