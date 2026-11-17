use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Exile up to two target cards from a single graveyard" (CR 115.1, 608.2b): the cards still in that graveyard are exiled
/// together; a target that became illegal is left alone.
fn exile_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.exile_all(&targets::legal_objects(g, item));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, exile_cards, "When this creature enters, exile up to two target cards from a single graveyard.").targets(targets::UP_TO_TWO_CARDS_FROM_A_SINGLE_GRAVEYARD)],
    ..oracle::GRIFFNAUT_TRACKER
};
