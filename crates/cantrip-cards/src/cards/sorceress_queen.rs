use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, effects::set_first_target_base_pt::<0, 2>, "{T}: Target creature other than this creature has base power and toughness 0/2 until end of turn.")
        .targets(&[targets::OTHER_CREATURE])],
    ..oracle::SORCERESS_QUEEN
};
