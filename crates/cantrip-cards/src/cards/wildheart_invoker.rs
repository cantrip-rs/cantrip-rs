use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{8}: Target creature gets +5/+5 and gains trample until end of turn" (CR 611.2a, 702.19): any creature, not only this one.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{8}"),
        effects::target_creature_gets_and_gains_trample::<5, 5>,
        "{8}: Target creature gets +5/+5 and gains trample until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::WILDHEART_INVOKER
};
