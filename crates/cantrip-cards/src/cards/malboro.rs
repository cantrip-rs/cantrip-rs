use cantrip_api::*;
use cantrip_mechanics::effects::each_opponent_discards_a_card;
use cantrip_mechanics::{library, triggers, typecycling};
use cantrip_oracle as oracle;

/// "Each opponent discards a card, loses 2 life, and exiles the top three cards of their library." Three steps in the order
/// the ruling gives (CR 608.2c): the discards are chosen in turn order and made together, then each opponent loses 2 life,
/// then the top three cards of every opponent's library are exiled together. A step that does nothing (an empty hand, a short
/// library) does not stop the next.
fn bad_breath(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_discards_a_card(g, item)?;
    g.each_opponent_loses_life(item.controller, 2);
    library::each_opponent_exiles_top(g, item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, bad_breath, "Bad Breath \u{2014} When this creature enters, each opponent discards a card, loses 2 life, and exiles the top three cards of their library."),
        typecycling::ability(Subtype::Swamp, mana("{2}"), "Swampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::MALBORO
};
