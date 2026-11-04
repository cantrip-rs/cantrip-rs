use cantrip_api::*;
use cantrip_mechanics::rampage;
use cantrip_oracle as oracle;

/// "Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)" (CR 702.23a).
pub static CARD: CardDef = CardDef {
    abilities: &[rampage::trigger::<2>("Rampage 2")],
    ..oracle::WOLVERINE_PACK
};
