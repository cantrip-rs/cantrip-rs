use cantrip_api::*;
use cantrip_mechanics::{effects, library, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "RevealUntilACreature",
    "reveal until you reveal a creature card?",
    "That card goes into your hand; the rest go on the bottom of your library in a random order",
    "Reveal",
    "Don't reveal",
);

/// "You may reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the
/// rest on the bottom of your library in a random order" (CR 701.20, 401.4). A library holding no creature card is revealed
/// whole and put back in a random order.
fn reveal_a_creature_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        library::reveal_until_to_hand_rest_to_bottom(g, item.controller, &|q, card| q.chars(card).is(CardType::Creature));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_NONTOKEN_CREATURE_YOU_CONTROL_DIES,
        reveal_a_creature_to_hand,
        "Whenever another nontoken creature you control dies, you may reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest on the bottom of your library in a random order.",
    )],
    ..oracle::SPINNER_OF_SOULS
};
