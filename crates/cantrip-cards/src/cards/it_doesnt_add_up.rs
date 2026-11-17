use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield. Suspect it." (CR 110.2a, 701.60a, 608.2b): it enters under
/// your control, untapped, and the permanent that came back, a new object (CR 400.7), is suspected; nothing is if the card was
/// kept from the battlefield (a replacement effect) or the target is no longer legal.
fn return_to_the_battlefield_and_suspect_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let creature = g.put_onto_battlefield(card, item.controller, false)?;
    g.suspect(creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], return_to_the_battlefield_and_suspect_it),
    ..oracle::IT_DOESNT_ADD_UP
};
