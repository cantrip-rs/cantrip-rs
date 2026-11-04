use cantrip_api::*;
use cantrip_mechanics::rampage;
use cantrip_oracle as oracle;

/// "Rampage 3 (Whenever this creature becomes blocked, it gets +3/+3 until end of turn for each creature blocking it beyond the first.)" (CR 702.1).
pub static CARD: CardDef = CardDef {
    abilities: &[rampage::trigger::<3>("Rampage 3")],
    ..oracle::AERATHI_BERSERKER
};
