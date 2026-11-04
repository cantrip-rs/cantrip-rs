use cantrip_api::*;
use cantrip_mechanics::{effects, flashback};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, 2)),
    // "As an additional cost to cast this spell, discard a card" (CR 601.2b, 118.8): paid with the mana cost, and with the flashback
    // cost too ("its flashback cost and any additional costs", CR 702.34a).
    additional_cost: Some(discard_cards(1)),
    ways: &[flashback::way(cost("{3}{R}"), "Flashback {3}{R}")],
    ..oracle::LAUGHING_MAD
};
