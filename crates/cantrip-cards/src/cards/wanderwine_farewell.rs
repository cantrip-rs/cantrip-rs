use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::{self, MERFOLK_WHITE_BLUE};
use cantrip_oracle as oracle;

/// "Return one or two target nonland permanents to their owners' hands. Then if you control a Merfolk, create a 1/1 white and blue
/// Merfolk creature token for each permanent returned to its owner's hand this way." The permanents still legal as it resolves
/// all go at once (CR 608.2b, 400.7); then, with the layers as they stand without them, a Merfolk you control - a returned one no
/// longer counts, a Merfolk token among them included - makes one token for each that really reached a hand. "Merfolk" is read
/// through this card's words (CR 612).
fn return_and_make_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returned = g.return_to_hand(&targets::legal_objects(g, item));
    let merfolk = g.words(item.object).subtype(Subtype::Merfolk);
    if g.permanents_of(item.controller).into_iter().any(|id| g.chars(id).has_subtype(merfolk)) {
        tokens::create(g, item.controller, &MERFOLK_WHITE_BLUE, returned.len() as u32)?;
    }
    Ok(())
}

/// Convoke is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef { spell: spell(&[targets::NONLAND_PERMANENT.between(1, 2)], return_and_make_merfolk), ..oracle::WANDERWINE_FAREWELL };

pub static EXTRA: &[&CardDef] = &[&MERFOLK_WHITE_BLUE];
