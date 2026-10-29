use cantrip_api::*;
use cantrip_mechanics::{effects, paradigm, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT_CARD_IN_YOUR_GRAVEYARD], effects::return_target_card_to_battlefield),
    paradigm: Some(paradigm::SPELL_ABILITY),
    delayed: &[paradigm::TRIGGER],
    ..oracle::RESTORATION_SEMINAR
};
