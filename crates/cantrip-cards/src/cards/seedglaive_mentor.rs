use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::VALIANT};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        VALIANT,
        effects::plus_one_counter_on_this,
        "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, put a +1/+1 counter on it.",
    )],
    ..oracle::SEEDGLAIVE_MENTOR
};
