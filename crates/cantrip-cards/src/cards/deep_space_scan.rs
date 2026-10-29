use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "Draw X cards, where X is the number of differently named lands you control. You have no maximum hand size until your
/// next turn." X is read on resolution (CR 608.2h); the hand-size effect expires as that player's next turn begins.
fn draw_for_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let count = lands::differently_named_lands(g, player);
    g.draw(player, count as u32);
    g.add_effect(item.object, item.object, EffectKind::MaximumHandSize(None), Duration::UntilNextTurnOf(player));
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], draw_for_lands), ..oracle::DEEP_SPACE_SCAN };
