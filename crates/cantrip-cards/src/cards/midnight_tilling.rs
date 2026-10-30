use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "Mill four cards, then you may return a permanent card from among them to your hand." (CR 701.17, 110.4a): the four cards
/// milled are the ones to take from, a permanent card being an artifact, creature, enchantment, land, planeswalker or battle
/// card; nobody is asked when none of them is, and an empty or short library mills what it has.
fn midnight_tilling(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_may_take_into_hand(g, item.controller, item.object, 4, "permanent", targets::is_permanent_card)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], midnight_tilling), ..oracle::MIDNIGHT_TILLING };
