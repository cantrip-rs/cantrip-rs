use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_you_gain_life::<2, 2, 2>),
    ..oracle::MOMENT_OF_TRIUMPH
};
