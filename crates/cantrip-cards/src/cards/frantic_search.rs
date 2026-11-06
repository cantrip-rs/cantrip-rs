use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::discard_chosen;
use cantrip_oracle as oracle;

/// Draw two cards, then discard two cards, then untap up to three lands. The lands are chosen as the spell resolves, not
/// targeted: any lands on the battlefield, tapped or not; nothing is asked when there are none.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    discard_chosen(g, item.controller, item.object, 2)?;
    let lands: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    let chosen = choose::cards(g, item.controller, item.object, Purpose::Untap, lands, 3, true)?;
    g.untap_together(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::FRANTIC_SEARCH };
