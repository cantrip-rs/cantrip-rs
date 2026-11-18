use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_gains_trample::<3, 3>),
    ..oracle::FANATICAL_STRENGTH
};
