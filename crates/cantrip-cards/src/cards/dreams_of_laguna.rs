use cantrip_api::*;
use cantrip_mechanics::{effects, flashback};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::surveil_then_draw::<1>),
    ways: &[flashback::way(cost("{3}{U}"), "Flashback {3}{U}")],
    ..oracle::DREAMS_OF_LAGUNA
};
