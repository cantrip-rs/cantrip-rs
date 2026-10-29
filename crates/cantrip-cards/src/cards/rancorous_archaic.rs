use cantrip_api::*;
use cantrip_mechanics::spent;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(spent::enters_with_a_plus_one_counter_for_each_color_spent),
    ..oracle::RANCOROUS_ARCHAIC
};
