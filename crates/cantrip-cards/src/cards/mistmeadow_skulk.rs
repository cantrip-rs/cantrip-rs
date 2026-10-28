use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        protection::from_mana_value_at_least::<3>("Protection from mana value 3 or greater"),
    ],
    ..oracle::MISTMEADOW_SKULK
};
