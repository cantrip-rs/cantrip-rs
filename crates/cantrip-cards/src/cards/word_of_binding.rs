use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.counted()], effects::tap_targets),
    ..oracle::WORD_OF_BINDING
};
