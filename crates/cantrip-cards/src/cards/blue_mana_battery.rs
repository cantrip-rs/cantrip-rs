use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &mana::mana_battery::<{ Mana::U as u8 }>("{T}, Remove any number of charge counters from this artifact: Add {U}, then add an additional {U} for each charge counter removed this way."),
    ..oracle::BLUE_MANA_BATTERY
};
