use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Destroy target tapped creature."
pub static RIP_THE_SEAMS: CardDef = CardDef {
    spell: spell(&[targets::TAPPED_CREATURE], effects::destroy_first_target),
    ..oracle::RIP_THE_SEAMS
};

pub static CARD: CardDef = CardDef { adventure: Some(&RIP_THE_SEAMS), ..oracle::THREADBIND_CLIQUE };

pub static EXTRA: &[&CardDef] = &[&RIP_THE_SEAMS];
