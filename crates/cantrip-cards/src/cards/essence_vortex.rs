use cantrip_api::*;
use cantrip_mechanics::{destroy_target_no_regen, targets};
use cantrip_oracle as oracle;

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");

/// The creature's controller may pay its current toughness in life to avoid its destruction (CR 118.1, 608.2h).
fn destroy_unless_paid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let payer = g.controller(creature);
    let toughness = g.chars(creature).toughness.unwrap_or(0).max(0) as u32;
    let paid = g.may_pay(payer, item.object, item.object, Cost { life: toughness, ..Cost::NONE }, &PAY_QUESTION)?;
    g.end_sentence();
    if !paid {
        destroy_target_no_regen(g, item, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], destroy_unless_paid),
    ..oracle::ESSENCE_VORTEX
};
