use cantrip_api::*;
use cantrip_mechanics::{exile_chosen_from_hand, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, target opponent exiles a card from their hand": the opponent chooses it (CR 608.2d), asked
/// even when it is their only card, and none is asked of an empty hand.
fn exile_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    exile_chosen_from_hand(g, opponent, item.object, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, exile_a_card, "When this creature enters, target opponent exiles a card from their hand.").targets(&[targets::OPPONENT])],
    ..oracle::UNSCRUPULOUS_AGENT
};
