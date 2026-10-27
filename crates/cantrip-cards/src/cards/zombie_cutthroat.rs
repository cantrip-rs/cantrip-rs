use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[morph::way(Cost { life: 5, ..Cost::NONE }, "Morph\u{2014}Pay 5 life.")], ..oracle::ZOMBIE_CUTTHROAT };
