use cantrip_api::*;
use cantrip_mechanics::{choose, lands, library, top_of_library, triggers};
use cantrip_oracle as oracle;

/// "Strange new worlds — Whenever Commander Spock enters or attacks, reveal the top card of your library. You may cast that card
/// without paying its mana cost if that spell's mana value is equal to or less than the number of differently named lands you
/// control. If it's a land card, you may put it onto the battlefield tapped. If that card is still on top of your library, draw a
/// card."
///
/// The card is revealed and stays where it is until the ability's steps take it (CR 701.20a, 608.2c). The cast is made while the
/// ability resolves, for nothing, and only a face whose mana value the differently named lands you control reach may be cast
/// (CR 608.2g, 118.9, 601.2b); a land has no such cast, so it is only ever put onto the battlefield tapped (CR 305.1). The draw
/// comes last and only when the card is still the top card of the library: cast, put onto the battlefield, or gone otherwise (a
/// card a replacement effect moved), nothing is drawn.
fn strange_new_worlds(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(&top) = top_of_library(g, you, 1).first() else { return Ok(()) };
    g.reveal(you, &[top]);
    g.end_sentence();
    let named = lands::differently_named_lands(g, you) as u32;
    if choose::may_cast_free_card(g, you, item.object, top, named, Purpose::CastFree)? {
        return Ok(());
    }
    if library::may_put_land_onto_battlefield(g, you, item.object, top, true)? {
        return Ok(());
    }
    if top_of_library(g, you, 1).first() == Some(&top) {
        g.draw(you, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        strange_new_worlds,
        "Whenever Commander Spock enters or attacks, reveal the top card of your library. You may cast that card without paying its mana cost if that spell's mana value is equal to or less than the number of differently named lands you control. If it's a land card, you may put it onto the battlefield tapped. If that card is still on top of your library, draw a card.",
    )],
    ..oracle::COMMANDER_SPOCK
};
