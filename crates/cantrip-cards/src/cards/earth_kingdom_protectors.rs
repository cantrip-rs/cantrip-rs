use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        SACRIFICE_THIS,
        effects::target_creature_gains_indestructible,
        "Sacrifice this creature: Another target Ally you control gains indestructible until end of turn.",
    )
    .targets(&[target(targets::another_permanent_of_subtype_you_control::<{ lands::sub(Subtype::Ally) }>)])],
    ..oracle::EARTH_KINGDOM_PROTECTORS
};
