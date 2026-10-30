use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +3/+3 until end of turn. If a creature entered the battlefield under your control this turn, draw a card"
/// (CR 608.2c): the condition is asked as the spell resolves, a creature that entered and has since died still counting. With its
/// only target illegal the spell does not resolve at all (CR 608.2b), so nothing is drawn.
fn pump_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets::<3, 3>(g, item)?;
    let entered = history::creature_entered_under_control_this_turn(g.history(), g.state(), item.controller, None, &|id| g.chars(id).is(CardType::Creature));
    if entered {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], pump_then_draw), ..oracle::THOUGHTWEFT_CHARGE };
