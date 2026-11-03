use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{R}{G}, {T}"),
        effects::target_creature_gets::<2, 2>,
        "{R}{G}, {T}: Target creature gets +2/+2 until end of turn.",
    ).targets(&[targets::CREATURE])],
    ..oracle::TUKNIR_DEATHLOCK
};
