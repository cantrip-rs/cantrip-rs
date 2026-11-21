use cantrip_api::*;
use cantrip_mechanics::{prepare, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_CAST_YOUR_THIRD_SPELL,
        prepare::become_prepared,
        "Whenever you cast your third spell each turn, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )],
    prepare: Some(&crate::lightning_bolt::CARD),
    ..oracle::EMERITUS_OF_CONFLICT
};
