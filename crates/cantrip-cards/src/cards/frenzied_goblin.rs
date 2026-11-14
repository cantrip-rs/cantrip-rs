use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "You may pay {R}. If you do, target creature can't block this turn" (CR 118.12, 608.2c): unlike a reflexive "when you do", the
/// target is chosen as the trigger goes on the stack, before any payment (CR 603.3d); the payment is made as it resolves, and only
/// the paid-for effect depends on it.
fn pay_so_target_cant_block(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    static QUESTION: Question = Question::new("PayForBlockRestriction", "pay {cost} so target creature can't block this turn?", "", "Pay {cost}", "Don't pay");
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{R}"), &QUESTION)?;
    g.end_sentence();
    if paid {
        effects::first_target_cant_block_this_turn(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        pay_so_target_cant_block,
        "Whenever this creature attacks, you may pay {R}. If you do, target creature can't block this turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::FRENZIED_GOBLIN
};
