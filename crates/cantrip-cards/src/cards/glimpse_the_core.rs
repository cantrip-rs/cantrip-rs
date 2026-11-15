use cantrip_api::*;
use cantrip_mechanics::library::{search_to_battlefield, Find};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Search your library for a basic Forest card, put that card onto the battlefield tapped, then shuffle" (CR 701.23): a
/// basic land card with the land type Forest, read as this spell's text says it now (CR 612.2). It may be left unfound
/// (CR 701.23b), and the library is shuffled either way.
fn search_for_a_forest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let forest = g.words(item.object).subtype(Subtype::Forest);
    search_to_battlefield(g, item.controller, item.object, Find::ONE, true, move |q, c| {
        let chars = q.chars(c);
        chars.is(CardType::Land) && chars.supertypes.basic && chars.has_subtype(forest)
    })?;
    Ok(())
}

/// "Target Cave card from your graveyard": a card with the land type Cave (CR 205.3i), which no text-changing effect names.
fn cave_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[]) && matches!(t, Target::Object(id) if q.chars(id).has_subtype(Subtype::Cave))
}

static MODAL: ModalDef = modes(&[
    mode(search_for_a_forest, "Search your library for a basic Forest card, put that card onto the battlefield tapped, then shuffle."),
    mode(effects::return_first_target_to_battlefield_tapped, "Return target Cave card from your graveyard to the battlefield tapped.").targets(&[target(cave_card_in_your_graveyard)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::GLIMPSE_THE_CORE };
