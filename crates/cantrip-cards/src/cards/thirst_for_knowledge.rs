use cantrip_api::*;
use cantrip_mechanics::discard_two_unless_discarding;
use cantrip_oracle as oracle;

/// "Draw three cards. Then discard two cards unless you discard an artifact card." The player decides as the discard is
/// made (CR 608.2d) between one artifact card and two cards of any kind; with no artifact card in hand the choice is not
/// offered. Two cards are chosen freely, so an artifact card may be one of them (its ruling).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.draw(you, 3);
    g.end_sentence();
    discard_two_unless_discarding(g, &[you], item.object, |chars| chars.is(CardType::Artifact))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    labels: &["Discard an artifact card", "Discard two cards"],
    ..oracle::THIRST_FOR_KNOWLEDGE
};
