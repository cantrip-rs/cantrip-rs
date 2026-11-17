use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { enters: Some(history::enters_with_a_plus_one_counter_if_a_creature_died), ..oracle::CACKLING_SLASHER };
