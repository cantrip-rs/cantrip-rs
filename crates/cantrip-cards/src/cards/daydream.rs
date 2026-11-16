use cantrip_api::*;
use cantrip_mechanics::{effects, flashback, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effects::exile_target_then_return_with_a_plus_one_counter),
    ways: &[flashback::way(cost("{2}{W}"), "Flashback {2}{W}")],
    ..oracle::DAYDREAM
};
