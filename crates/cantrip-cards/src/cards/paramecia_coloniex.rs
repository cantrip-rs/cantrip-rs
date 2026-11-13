use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileParamecia", "exile Paramecia Coloniex to put a creature card from your graveyard on top of your library?", "Choose a creature card from your graveyard", "Exile it", "Don't");

/// "When this creature dies, you may exile it. When you do, ..." (CR 603.12): the card it became in the graveyard (CR 400.7;
/// each card of a merged permanent, CR 730.3c) is exiled, and only if one really was does the reflexive ability trigger. Its
/// target is chosen as that ability goes on the stack, so the Worm, now in exile, is not among the creature cards in the
/// graveyard.
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.found_all_after_move(item.object).is_empty() {
        return Ok(());
    }
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    if !effects::exile_what_it_became(g, item.object).is_empty() {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "Put target creature card from your graveyard on top of your library": with the card gone from the graveyard the ability
/// does nothing (CR 608.2b).
fn put_on_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_into_library(&[card], LibraryPlace::Top);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::you_mill::<3>, "When this creature enters, mill three cards."),
        triggered(THIS_DIES, exile_it, "When this creature dies, you may exile it. When you do, put target creature card from your graveyard on top of your library."),
    ],
    delayed: &[delayed_trigger(put_on_top, "When you do, put target creature card from your graveyard on top of your library.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::PARAMECIA_COLONIEX
};
