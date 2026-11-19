use cantrip_api::*;
use cantrip_mechanics::triggers::ARTIFACT_YOU_CONTROL_ENTERS;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ARTIFACT_YOU_CONTROL_ENTERS,
        effects::plus_one_counter_on_target,
        "Whenever an artifact you control enters, put a +1/+1 counter on target Pirate you control.",
    )
    .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Pirate) }>)])],
    ..oracle::CAPTAIN_STORM_COSMIUM_RAIDER
};
