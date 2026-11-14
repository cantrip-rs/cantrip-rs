use cantrip_api::*;
use cantrip_mechanics::{effects::plus_one_counter_on_this, triggers::YOU_GAIN_LIFE};
use cantrip_oracle as oracle;

/// "Whenever you gain life, put a +1/+1 counter on this creature." It triggers once for each life gain event (CR 603.2c), however
/// much life it gains, and not at all for an opponent's life gain.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_GAIN_LIFE, plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on this creature.")],
    ..oracle::AJANIS_PRIDEMATE
};
