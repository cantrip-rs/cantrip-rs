use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL_WITH_A_SINGLE_TARGET], effects::change_first_target),
    ..oracle::DEFLECTION
};
