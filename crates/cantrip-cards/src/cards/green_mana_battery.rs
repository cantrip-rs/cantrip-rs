use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &mana::mana_battery::<{ Mana::G as u8 }>("{T}, Remove any number of charge counters from this artifact: Add {G}, then add an additional {G} for each charge counter removed this way."),
    ..oracle::GREEN_MANA_BATTERY
};
