use cantrip_api::*;
use cantrip_mechanics::{effects, flashback};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::draw_two_then_discard_two),
    ways: &[flashback::way(cost("{2}{R}"), "Flashback {2}{R}")],
    ..oracle::FAITHLESS_LOOTING
};
