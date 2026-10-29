use cantrip_api::*;
use cantrip_mechanics::{library, select, sneak};
use cantrip_oracle as oracle;

/// Search your library for a card, put that card into your hand, then shuffle.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, select::CARD, false)?;
    Ok(())
}

/// A sorcery cast for its sneak cost (CR 702.190) goes to the graveyard as it resolves: only a creature spell enters attacking.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[sneak::way(mana("{1}{B}"), "Sneak {1}{B}")],
    ..oracle::SPLINTERS_TECHNIQUE
};
