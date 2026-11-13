use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::{copy, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    // "Copy target spell you control. You may choose new targets for the copy" (CR 707.10, 707.10c): the Imitator is already
    // sacrificed, a cost; a permanent spell's copy becomes a token (CR 608.3f).
    delayed: &[ability(
        cost("{1}").and(SACRIFICE_THIS),
        copy::copy_target_spell,
        "{1}, Sacrifice this creature: Copy target spell you control. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)",
    )
    .targets(&[targets::SPELL_YOU_CONTROL])],
    statics: &[max_speed(
        "Max speed \u{2014} {1}, Sacrifice this creature: Copy target spell you control. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)",
    )],
    ..oracle::SLICK_IMITATOR
};
