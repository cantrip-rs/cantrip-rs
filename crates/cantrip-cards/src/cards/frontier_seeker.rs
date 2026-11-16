use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, look at the top five cards of your library. You may reveal a Mount creature card or a Plains card from
/// among them and put it into your hand. Put the rest on the bottom of your library in a random order." (CR 701.20a, 401.4): a Plains
/// card is any card with the subtype Plains (a Savannah too), a Mount creature card a creature card with the subtype Mount, both
/// subtypes read through the words of the source's text (CR 612.2); at most one is taken.
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let (plains, mount) = (words.subtype(Subtype::Plains), words.subtype(Subtype::Mount));
    look_at_top(
        g,
        item.controller,
        item.object,
        5,
        (0, 1),
        true,
        |c| c.has_subtype(plains) || (c.is(CardType::Creature) && c.has_subtype(mount)),
        Rest::BottomRandom,
    )?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look,
        "When this creature enters, look at the top five cards of your library. You may reveal a Mount creature card or a Plains card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::FRONTIER_SEEKER
};
