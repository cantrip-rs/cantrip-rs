use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE_SPELL], effects::counter_first_target_unless_pays::<4>),
    ..oracle::SPECTRAL_INTERFERENCE
};
