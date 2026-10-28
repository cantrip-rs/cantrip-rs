use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_oracle as oracle;

/// Flash and trample are core keywords. "Whenever you gain life, put a +1/+1 counter on Tigra." (CR 119.3, 122.1): one counter
/// each time life is gained, however much; none when none is gained.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_GAIN_LIFE, plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on Tigra.")],
    ..oracle::TIGRA_FELINE_FURY
};
