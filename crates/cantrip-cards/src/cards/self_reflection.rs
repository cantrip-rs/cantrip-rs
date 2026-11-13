use cantrip_api::*;
use cantrip_mechanics::{copy, flashback, targets};
use cantrip_oracle as oracle;

/// "Create a token that's a copy of target creature you control" (CR 111.1, 707.2); Flashback {3}{U} (CR 702.34).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], copy::create_token_copy_of_target),
    ways: &[flashback::way(cost("{3}{U}"), "Flashback {3}{U}")],
    ..oracle::SELF_REFLECTION
};
