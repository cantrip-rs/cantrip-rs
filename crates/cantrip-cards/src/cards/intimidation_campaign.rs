use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Each opponent loses 1 life, you gain 1 life, and you draw a card." The three are one sentence, so no sentence ends
/// between them (CR 608.2c); the loss and the gain are `each_opponent_loses_1_life_you_gain_1`'s, then the draw.
fn drain_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_loses_1_life_you_gain_1(g, item)?;
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, drain_and_draw, "When this enchantment enters, each opponent loses 1 life, you gain 1 life, and you draw a card."),
        triggered(triggers::YOU_COMMIT_A_CRIME, effects::may_return_this_to_hand, "Whenever you commit a crime, you may return this enchantment to its owner's hand. (It returns only from the battlefield. Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)"),
    ],
    ..oracle::INTIMIDATION_CAMPAIGN
};
