use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], effects::each_player_sacrifices_creatures::<1>), ..oracle::INNOCENT_BLOOD };
