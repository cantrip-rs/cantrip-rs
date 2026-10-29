use cantrip_api::*;
use cantrip_mechanics::{effects, hand, targets};
use cantrip_oracle as oracle;

/// "Draw X cards. Then you may put a permanent card with mana value X or less from your hand onto the battlefield tapped."
/// (CR 107.3, 121.1, 608.2d, 614.1d): X is the spell's, and the cards just drawn are in the hand to choose from. A card's
/// mana value in a hand counts its {X} as 0 (CR 107.3b, `Query::mana_value`).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = item.x as u32;
    effects::draw(g, item, x)?;
    g.end_sentence();
    hand::may_put_onto_battlefield_tapped(g, item.controller, item.object, |g, c| {
        targets::is_permanent_card(&g.chars(c)) && g.mana_value(c) <= x
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::MIND_INTO_MATTER };
