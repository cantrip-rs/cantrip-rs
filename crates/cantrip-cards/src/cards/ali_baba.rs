use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{R}"), effects::tap_targets, "{R}: Tap target Wall.")
        .targets(&[targets::WALL])],
    ..oracle::ALI_BABA
};
