use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_BECOMES_TAPPED,
        effects::target_creature_gets::<2, 0>,
        "Whenever this creature becomes tapped, another target Merfolk you control gets +2/+0 until end of turn.",
    )
    .targets(&[target(targets::another_permanent_of_subtype_you_control::<{ lands::sub(Subtype::Merfolk) }>)])],
    ..oracle::TRIBUTARY_VAULTER
};
