use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_CAST_A_SPELL, effects::untap_all_nonland_permanents_you_control, "Whenever you cast a spell, untap all nonland permanents you control.")],
    ..oracle::PARADOX_ENGINE
};
