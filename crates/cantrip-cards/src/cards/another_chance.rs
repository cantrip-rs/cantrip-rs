use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// You may mill two cards. Then return up to two creature cards from your graveyard to your hand: the milled cards are in the graveyard
/// by then, so they can be chosen, and so can cards that were there already.
fn mill_then_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_mill(g, item, 2)?;
    effects::return_up_to_two_creature_cards_from_your_graveyard_to_your_hand(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], mill_then_return), ..oracle::ANOTHER_CHANCE };
