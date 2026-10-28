use cantrip_api::*;
use cantrip_mechanics::triggers::ward;
use cantrip_oracle as oracle;

/// The nightbound face: "Ward {3}" (CR 702.21a); its nightbound is the core's.
pub static BACK: CardDef = CardDef {
    abilities: &[ward::<3>("Ward {3} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3}.)")],
    ..oracle::DIRE_STRAIN_DEMOLISHER
};

/// The daybound face: "Ward {1}" (CR 702.21a); its daybound is the core's.
pub static CARD: CardDef = CardDef {
    abilities: &[ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)")],
    back: Some(&BACK),
    ..oracle::BURLY_BREAKER
};
