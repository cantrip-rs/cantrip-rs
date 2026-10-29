use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileCavalier", "exile Cavalier of Thorns to put the target card on top of your library?", "The target card goes on top of your library", "Exile it", "Don't");

/// "When this creature enters, reveal the top five cards of your library. Put a land card from among them onto the
/// battlefield and the rest into your graveyard." (CR 701.20): the land is not optional, one is chosen whenever any
/// revealed card is a land; every other revealed card is put into the graveyard (CR 701.20a: revealing moves nothing), together, in the order their owner arranges them (CR 404.3).
fn reveal_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::reveal_top_land_to_battlefield_rest_to_graveyard(g, item.controller, item.object, 5, false)
}

/// "Another target card from your graveyard" (CR 115.1): any card there but the Cavalier itself, which is in the graveyard
/// as the new object it became (CR 400.7) when the trigger is put on the stack.
fn another_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[]) && matches!(t, Target::Object(id) if q.found_after_move(asker.source) != Some(id))
}

/// "When this creature dies, you may exile it. If you do, put another target card from your graveyard on top of your
/// library." (CR 603.10a, 608.2b, 400.7, 608.2c): with its target gone the ability does nothing at all; otherwise the Cavalier, as
/// the card it became in a graveyard, may be exiled, and only if it was exiled does the target go on top of its owner's library.
fn exile_it_to_stack_the_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let Some(card) = g.found_after_move(item.object) else { return Ok(()) };
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let exiled = g.exile(card);
    if g.obj(exiled).zone.is(ZoneKind::Exile) {
        g.put_into_library(&[target], LibraryPlace::Top);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, reveal_five, "When this creature enters, reveal the top five cards of your library. Put a land card from among them onto the battlefield and the rest into your graveyard."),
        triggered(THIS_DIES, exile_it_to_stack_the_top, "When this creature dies, you may exile it. If you do, put another target card from your graveyard on top of your library.")
            .targets(&[target(another_card_in_your_graveyard)]),
    ],
    ..oracle::CAVALIER_OF_THORNS
};
