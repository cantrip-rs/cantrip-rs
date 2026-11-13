use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, untap all nonland permanents you control." (CR 513.1, 701.26b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        effects::untap_all_nonland_permanents_you_control,
        "At the beginning of your end step, untap all nonland permanents you control.",
    )],
    ..oracle::UNSTOPPABLE_PLAN
};
