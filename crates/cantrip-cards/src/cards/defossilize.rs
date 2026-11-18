use cantrip_api::*;
use cantrip_mechanics::{explore, targets};
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield. That creature explores, then it explores again" (CR 110.2a, 701.44a,
/// 608.2c): the card comes back untapped under your control, and the new object explores twice, the second time even if the first left a
/// +1/+1 counter, a card on top of the library or it itself gone (CR 701.44c). A card a replacement effect sent elsewhere (Containment
/// Priest's exile) never became that creature, and nothing explores; nothing is returned when the target is no longer legal (CR 608.2b).
fn return_then_explore_twice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let creature = g.put_onto_battlefield(card, item.controller, false)?;
    if g.is_on_battlefield(creature) {
        explore(g, creature)?;
        explore(g, creature)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], return_then_explore_twice), ..oracle::DEFOSSILIZE };
