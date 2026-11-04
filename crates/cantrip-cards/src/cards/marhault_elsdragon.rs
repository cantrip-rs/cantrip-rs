use cantrip_api::*;
use cantrip_mechanics::rampage;
use cantrip_oracle as oracle;

/// "Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)" (CR 702.1).
pub static CARD: CardDef = CardDef {
    abilities: &[rampage::trigger::<1>("Rampage 1")],
    ..oracle::MARHAULT_ELSDRAGON
};
