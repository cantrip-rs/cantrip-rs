use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, effects, targets};
use cantrip_oracle as oracle;

/// "Return up to two target creature cards from your graveyard to your hand, then discard a card." (CR 608.2c, 701.9): the cards
/// still legal return together, and the discard follows whatever came back, even none, so the controller may have to discard
/// one of the cards just returned.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_each_target_to_hand(g, item)?;
    g.end_sentence();
    discard_chosen(g, item.controller, item.object, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)], effect),
    ..oracle::MACABRE_WALTZ
};
