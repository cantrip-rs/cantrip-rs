use cantrip_api::*;
use cantrip_mechanics::{discover, targets};
use cantrip_oracle as oracle;

/// "Return up to one target permanent card from your graveyard to your hand. Discover 4" (CR 601.2c, 608.2b, 701.57a): with no target chosen the
/// spell still resolves and discovers; if the one target it chose is illegal as it resolves the whole spell does nothing, discovering included.
fn return_a_card_then_discover(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        g.return_to_hand(&[card]);
    }
    g.end_sentence();
    discover::discover(g, item.controller, 4)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[up_to_one(targets::permanent_card_in_your_graveyard)], return_a_card_then_discover),
    ..oracle::WALK_WITH_THE_ANCESTORS
};
