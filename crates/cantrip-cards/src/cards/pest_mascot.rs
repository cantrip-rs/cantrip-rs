use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_oracle as oracle;

/// Trample is in the characteristics. "Whenever you gain life" triggers once for each life-gain event, not for each point.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_GAIN_LIFE, plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on this creature.")],
    ..oracle::PEST_MASCOT
};
