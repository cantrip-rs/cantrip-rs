use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +3/+3 until end of turn. If a creature entered the battlefield under your control this turn, draw a card"
/// (CR 608.2c): the condition is asked as the spell resolves, of the history's `Event::Entered` (each creature as it entered, under the controller
/// it entered under, CR 603.6a): one that has since died still counts, one that entered under an opponent's control and was stolen
/// does not, and neither does a noncreature that has become one. With its
/// only target illegal the spell does not resolve at all (CR 608.2b), so nothing is drawn.
fn pump_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets::<3, 3>(g, item)?;
    let entered = g.history().entered().any(|e| e.controller == item.controller && e.is(CardType::Creature));
    if entered {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], pump_then_draw), ..oracle::THOUGHTWEFT_CHARGE };
