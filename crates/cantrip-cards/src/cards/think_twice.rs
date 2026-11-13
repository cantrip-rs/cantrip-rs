use cantrip_api::*;
use cantrip_mechanics::{effects, flashback};
use cantrip_oracle as oracle;

/// "Draw a card." (CR 121.1) and "Flashback {2}{U}" (CR 702.34a): the flashback way is the shared one.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, 1)),
    ways: &[flashback::way(cost("{2}{U}"), "Flashback {2}{U}")],
    ..oracle::THINK_TWICE
};
