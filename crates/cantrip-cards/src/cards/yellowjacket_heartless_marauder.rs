use cantrip_api::*;
use cantrip_mechanics::effects::this_gets_and_gains_lifelink;
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

/// Flying is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Villain) }>),
        this_gets_and_gains_lifelink::<1, 0>,
        "Whenever another Villain you control enters, Yellowjacket gets +1/+0 and gains lifelink until end of turn.",
    )],
    ..oracle::YELLOWJACKET_HEARTLESS_MARAUDER
};
