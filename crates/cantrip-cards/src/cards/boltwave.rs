use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], effects::damage_each_opponent_n::<3>), ..oracle::BOLTWAVE };
