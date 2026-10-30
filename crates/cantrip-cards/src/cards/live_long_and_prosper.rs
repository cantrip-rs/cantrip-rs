use cantrip_api::*;
use cantrip_mechanics::double;
use cantrip_oracle as oracle;

/// "Double your life total. Draw cards equal to the number of cards in your hand. You have no maximum hand size until
/// your next turn." Three instructions, followed in order (CR 608.2c): the life total is doubled as the spell resolves
/// (CR 701.10d), the hand counted then, before any of the cards is drawn (CR 608.2h), and the hand-size effect lasts
/// until that player's next turn begins (CR 514.2).
fn prosper(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    double::life_total(g, you);
    g.end_sentence();
    let in_hand = g.state().zone(Zone::hand(you)).len() as u32;
    g.draw(you, in_hand);
    g.end_sentence();
    g.add_effect(item.object, item.object, EffectKind::MaximumHandSize(None), Duration::UntilNextTurnOf(you));
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], prosper), ..oracle::LIVE_LONG_AND_PROSPER };
