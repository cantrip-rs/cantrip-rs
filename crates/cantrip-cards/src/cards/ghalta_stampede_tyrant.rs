use cantrip_api::*;
use cantrip_mechanics::{hand, select, triggers};
use cantrip_oracle as oracle;

/// "When Ghalta enters, put any number of creature cards from your hand onto the battlefield" (CR 608.2d, 101.4): chosen from the
/// controller's hand as the ability resolves, none, some or all, and put there together, not cast. Nothing is asked with no
/// creature card in the hand.
fn put_any_number_of_creature_cards_from_your_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_up_to_onto_battlefield(g, item.controller, item.object, select::creature_cards(select::ANY_NUMBER))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        put_any_number_of_creature_cards_from_your_hand,
        "When Ghalta enters, put any number of creature cards from your hand onto the battlefield.",
    )],
    ..oracle::GHALTA_STAMPEDE_TYRANT
};
