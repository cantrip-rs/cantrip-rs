use cantrip_api::*;
use cantrip_mechanics::{gift, greatest_power, targets};
use cantrip_oracle as oracle;

/// "Target opponent sacrifices a creature with the greatest power among creatures they control. If the gift was promised,
/// return target creature card from your graveyard to your hand." The gift (they draw a card) is given before this runs
/// (CR 702.174j), and the second target exists only when the gift was promised (`if_promised`, CR 601.2c). Each target is
/// checked on its own (CR 608.2b): the opponent may be gone while the card still returns, and the other way round.
fn sacrifice_then_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponent: Vec<PlayerId> = targets::player(g, item, 0).into_iter().collect();
    greatest_power::sacrifice_one_with_the_greatest_power(g, item.object, &opponent)?;
    let Some(card) = targets::object(g, item, 1) else { return Ok(()) };
    g.return_to_hand(&[card]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT, targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 0).if_promised(1, 1)], sacrifice_then_return),
    gift: Some(gift::CARD),
    ..oracle::CONSUMED_BY_GREED
};
