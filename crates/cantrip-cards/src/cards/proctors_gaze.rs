use cantrip_api::*;
use cantrip_mechanics::library::{search_to_battlefield, Find};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const UP_TO_ONE: TargetSpec = targets::NONLAND_PERMANENT.between(0, 1);

/// "Return up to one target nonland permanent to its owner's hand. Search your library for a basic land card, put it onto
/// the battlefield tapped, then shuffle." (CR 701.23, 608.2b): the search happens whether or not a permanent was targeted or
/// is still there to return; it may be left unfound (CR 701.23b), and the library is shuffled either way.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    g.end_sentence();
    search_to_battlefield(g, item.controller, item.object, Find::ONE, true, |q, c| {
        let chars = q.chars(c);
        chars.is(CardType::Land) && chars.supertypes.basic
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[UP_TO_ONE], effect), ..oracle::PROCTORS_GAZE };
