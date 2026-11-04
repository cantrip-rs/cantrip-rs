use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effects::target_creature_gets_plus_one_gains_hexproof_and_untaps),
    ..oracle::MAGIC_DAMPER
};
