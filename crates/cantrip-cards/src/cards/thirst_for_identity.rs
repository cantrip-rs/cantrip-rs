use cantrip_api::*;
use cantrip_mechanics::discard_two_unless_discarding;
use cantrip_oracle as oracle;

/// "Draw three cards. Then discard two cards unless you discard a creature card." The player decides as the discard is made (CR 608.2d)
/// between one creature card and two cards of any kind; with no creature card in hand the choice is not offered.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.draw(you, 3);
    discard_two_unless_discarding(g, &[you], item.object, |chars| chars.is(CardType::Creature))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    labels: &["Discard a creature card", "Discard two cards"],
    ..oracle::THIRST_FOR_IDENTITY
};
