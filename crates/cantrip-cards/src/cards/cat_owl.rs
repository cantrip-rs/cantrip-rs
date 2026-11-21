use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// Flying is a core keyword read from its characteristics. "Whenever this creature attacks, untap target artifact or creature."
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, effects::untap_targets, "Whenever this creature attacks, untap target artifact or creature.")
        .targets(&[targets::ARTIFACT_OR_CREATURE])],
    ..oracle::CAT_OWL
};
