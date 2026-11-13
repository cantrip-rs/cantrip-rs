use cantrip_api::*;
use cantrip_mechanics::{library, sneak};
use cantrip_oracle as oracle;

/// A sorcery cast for its sneak cost (CR 702.190) goes to the graveyard as it resolves: only a creature spell enters attacking.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], library::search_for_a_card_to_hand),
    ways: &[sneak::way(mana("{1}{B}"), "Sneak {1}{B}")],
    ..oracle::SPLINTERS_TECHNIQUE
};
