use cantrip_api::*;
use cantrip_mechanics::{effects, paradigm, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effects::target_player_draws_and_loses_life::<2, 2>),
    paradigm: Some(paradigm::SPELL_ABILITY),
    delayed: &[paradigm::TRIGGER],
    ..oracle::DECORUM_DISSERTATION
};
