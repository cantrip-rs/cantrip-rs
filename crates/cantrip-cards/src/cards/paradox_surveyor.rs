use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::{top_of_library, triggers};
use cantrip_oracle as oracle;

/// "Look at the top five cards of your library. You may reveal a land card or a card with {X} in its mana cost from among them
/// and put it into your hand. Put the rest on the bottom of your library in a random order." (CR 107.3, 401.4, 701.20): the
/// cards are judged as they are in the library, where a card has only its front face's characteristics (CR 712.8a) and its
/// printed mana cost shows an {X} whatever X would be. Up to one is taken, and shown; a library of fewer cards shows what it
/// has (CR 609.3).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let qualifying: Vec<CardId> = top_of_library(g, item.controller, 5)
        .into_iter()
        .filter(|&c| g.chars(c).is(CardType::Land) || g.mana_cost(c).x > 0)
        .map(|c| g.chars(c).card)
        .collect();
    look_at_top(g, item.controller, item.object, 5, (0, 1), true, |c| qualifying.contains(&c.card), Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        look,
        "When this creature enters, look at the top five cards of your library. You may reveal a land card or a card with {X} in its mana cost from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::PARADOX_SURVEYOR
};
