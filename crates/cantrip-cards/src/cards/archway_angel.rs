use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::gain_life_for_each_subtype_you_control::<{ lands::sub(Subtype::Gate) }, 2>,
        "When this creature enters, you gain 2 life for each Gate you control.",
    )],
    ..oracle::ARCHWAY_ANGEL
};
