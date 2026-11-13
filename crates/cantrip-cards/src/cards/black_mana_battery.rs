use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &mana::mana_battery::<{ Mana::B as u8 }>("{T}, Remove any number of charge counters from this artifact: Add {B}, then add an additional {B} for each charge counter removed this way."),
    ..oracle::BLACK_MANA_BATTERY
};
