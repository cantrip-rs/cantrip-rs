use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. You may reveal a Mutant, Ninja, Turtle, or land card from among them and put it
/// into your hand. Put the rest on the bottom of your library in a random order." (CR 401.4, 701.20a): the three creature types
/// are words of its text (CR 612).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let (mutant, ninja, turtle) = (words.subtype(Subtype::Mutant), words.subtype(Subtype::Ninja), words.subtype(Subtype::Turtle));
    look_at_top(
        g,
        item.controller,
        item.object,
        4,
        (0, 1),
        true,
        |c| c.has_subtype(mutant) || c.has_subtype(ninja) || c.has_subtype(turtle) || c.is(CardType::Land),
        Rest::BottomRandom,
    )?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], look), ..oracle::COWABUNGA };
