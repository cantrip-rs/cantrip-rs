use cantrip_api::*;
use cantrip_mechanics::{gift, targets};
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield. Then if the gift was promised and that creature isn't
/// legendary, create a token that's a copy of that creature, except it's 1/1" (CR 400.3, 702.174j, 707.2, 707.9b). The gift was
/// given before this, as the spell began to resolve. "That creature" is the card that entered, as it is on the battlefield:
/// a card that never entered (a replacement effect sent it elsewhere) makes no copy.
fn return_then_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let returned = g.put_onto_battlefield(card, item.controller, false)?;
    if item.gift.is_some() && g.is_on_battlefield(returned) && !g.chars(returned).supertypes.legendary {
        let values = g.copiable_values(returned).sized(1, 1);
        g.create_token_copies_of(values, item.controller, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], return_then_copy),
    gift: Some(gift::CARD),
    ..oracle::COILING_REBIRTH
};
