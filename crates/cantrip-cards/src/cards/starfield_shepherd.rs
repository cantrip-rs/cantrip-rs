use cantrip_api::*;
use cantrip_mechanics::library::{search_to_hand, Find};
use cantrip_mechanics::{select, triggers::THIS_ENTERS, warp};
use cantrip_oracle as oracle;

/// "When this creature enters, search your library for a basic Plains card or a creature card with mana value 1 or
/// less, reveal it, put it into your hand, then shuffle" (CR 701.23): one card, which may not be found (CR 701.23b).
/// A basic Plains card is a card with the basic supertype and the Plains subtype; a nonbasic land of the Plains type
/// is not one.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // "Plains" is a word of this card's text (CR 612): read it through the words the Shepherd has now.
    let plains = g.words(item.object).subtype(Subtype::Plains);
    search_to_hand(g, item.controller, item.object, Find::ONE.revealed(), |q, c| {
        select::is_basic_of_type(q, c, plains) || (q.chars(c).is(CardType::Creature) && q.mana_value(c) <= 1)
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, search, "When this creature enters, search your library for a basic Plains card or a creature card with mana value 1 or less, reveal it, put it into your hand, then shuffle.")],
    ways: &[warp::way(mana("{1}{W}"), "Warp {1}{W}")],
    end_step_riders: warp::RIDERS,
    delayed: &[warp::EXILE],
    ..oracle::STARFIELD_SHEPHERD
};
