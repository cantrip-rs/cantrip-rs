use cantrip_api::*;
use cantrip_mechanics::{effects, scry, triggers};
use cantrip_oracle as oracle;

/// "Scry X, where X is the number of planeswalkers you control": counted as the ability resolves. With none it is
/// scry 0, which is no scry, and nothing that triggers on one triggers (CR 701.22b, the ruling).
fn scry_planeswalkers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = g.permanents_of(you).into_iter().filter(|&p| g.chars(p).is(CardType::Planeswalker)).count();
    scry(g, you, item.object, x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::draw_three_then_discard_two, "When Oath of Jace enters, draw three cards, then discard two cards."),
        triggered(triggers::YOUR_UPKEEP, scry_planeswalkers, "At the beginning of your upkeep, scry X, where X is the number of planeswalkers you control."),
    ],
    ..oracle::OATH_OF_JACE
};
