use cantrip_api::*;
use cantrip_mechanics::{discover, tokens};
use cantrip_oracle as oracle;

/// "Discover 10. If the discovered card's mana value is less than 10, create a number of tapped Treasure tokens equal to the
/// difference" (CR 701.57a, c). The card is the one discover exiled, its mana value as it was exiled (`discover::discover`),
/// so casting it or putting it into the hand changes nothing; with no discovered card (the library ran out of nonlands) there
/// is no mana value to compare. The discovered card's mana value is 10 or less by definition, so "less than 10" is the
/// difference being above 0: a mana value of exactly 10 makes none (no Treasure is created, CR 111.1).
fn discover_ten_then_tapped_treasures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(value) = discover::discover(g, item.controller, 10)? else { return Ok(()) };
    tokens::create_by_rule(g, item.controller, &tokens::TREASURE, 10 - value, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], discover_ten_then_tapped_treasures),
    ..oracle::HIT_THE_MOTHER_LODE
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
