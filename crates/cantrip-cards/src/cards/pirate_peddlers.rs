use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_SACRIFICE_ANOTHER_PERMANENT,
        effects::plus_one_counter_on_this,
        "Whenever you sacrifice another permanent, put a +1/+1 counter on this creature.",
    )
    .on_sacrifice()],
    ..oracle::PIRATE_PEDDLERS
};
