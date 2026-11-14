use cantrip_api::*;
use cantrip_mechanics::copy::{self, SACRIFICE_THIS_TOKEN_EXCEPTION};
use cantrip_mechanics::{behold, flashback, targets};
use cantrip_oracle as oracle;

/// Flashback with an added cost (CR 702.34a, 118.8): {1}{R} plus beholding three Elementals (CR 701.4a), which exiles nothing.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], copy::create_token_copy_with_haste_and_sacrifice),
    ways: &[flashback::way(cost("{1}{R}").and(behold::cost_of(3, behold::elemental, "Elemental")), "Flashback\u{2014}{1}{R}, Behold three Elementals")],
    ..oracle::KINDLE_THE_INNER_FLAME
};

pub static EXTRA: &[&CardDef] = &[&SACRIFICE_THIS_TOKEN_EXCEPTION];
